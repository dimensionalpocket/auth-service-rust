use sqlx::migrate::Migrator;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Row, Sqlite, SqlitePool};
use std::collections::HashSet;
use std::fs;

static MIGRATOR: Migrator = sqlx::migrate!("./config/database/migrations");

pub struct Database {
  pub pool: SqlitePool,
}

impl Database {
  pub async fn new(sqlite_file_path: &str) -> Result<Self, sqlx::Error> {
    Self::new_with_pool_size(sqlite_file_path, None).await
  }

  pub async fn new_with_pool_size(
    sqlite_file_path: &str,
    pool_size: Option<u32>,
  ) -> Result<Self, sqlx::Error> {
    // Generate database URL internally
    let database_url = format!("sqlite:{sqlite_file_path}");

    // Create database if it doesn't exist
    if !Sqlite::database_exists(&database_url)
      .await
      .unwrap_or(false)
    {
      Sqlite::create_database(&database_url).await?;
    }

    let pool = match pool_size {
      Some(size) => {
        SqlitePoolOptions::new()
          .max_connections(size)
          .connect(&database_url)
          .await?
      }
      None => SqlitePool::connect(&database_url).await?,
    };

    // Configure SQLite settings after connection
    println!("🔧 Configuring SQLite database: {sqlite_file_path}");
    Self::configure_sqlite(&pool).await?;

    Ok(Database { pool })
  }

  /// SQLite PRAGMA commands for optimal performance and data integrity
  /// Add more commands to this array as needed for your application
  pub const SQLITE_PRAGMA_COMMANDS: &'static [&'static str] = &[
    // Enable WAL (Write-Ahead Logging) mode for better concurrency
    // WAL mode allows multiple readers while a writer is active
    "PRAGMA journal_mode = WAL;",
    // Enable foreign key constraints enforcement
    // SQLite doesn't enforce foreign keys by default
    "PRAGMA foreign_keys = ON;",
    // Set synchronous mode to NORMAL for better performance
    // NORMAL is safer than OFF but faster than FULL
    "PRAGMA synchronous = NORMAL;",
    // Additional performance optimizations (commented out for conservative defaults)
    // Uncomment and adjust these based on your specific performance requirements:

    // Set cache size to 2MB (negative value = KB, positive = pages)
    "PRAGMA cache_size = -2000;",
    // Set page size to 4MB (memory usage considering cache size: 8MB)
    "PRAGMA page_size = 4096;",
    // Enable memory-mapped I/O for better performance
    // Uses 1MB of memory-mapped I/O
    "PRAGMA mmap_size = 1048576;",
    // Set the maximum size of the journal file
    // This limits the size of the WAL file to 26MB
    "PRAGMA journal_size_limit = 27103364;",
  ];

  /// Configure SQLite settings for optimal performance and data integrity
  pub async fn configure_sqlite(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Execute each PRAGMA command
    for command in Self::SQLITE_PRAGMA_COMMANDS.iter() {
      println!("   {command}");
      sqlx::query(command).execute(pool).await?;
    }

    // Verify critical settings were applied correctly
    Self::verify_sqlite_config(pool).await?;

    println!("✅ SQLite configuration completed successfully");
    Ok(())
  }

  /// Verify that critical SQLite settings were applied correctly
  async fn verify_sqlite_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Check WAL mode is enabled
    let journal_mode: String = sqlx::query("PRAGMA journal_mode;")
      .fetch_one(pool)
      .await?
      .get(0);

    if journal_mode.to_uppercase() != "WAL" {
      eprintln!("⚠️  Warning: WAL mode not enabled, got: {journal_mode}");
    }

    // Check foreign keys are enabled
    let foreign_keys: i64 = sqlx::query("PRAGMA foreign_keys;")
      .fetch_one(pool)
      .await?
      .get(0);

    if foreign_keys != 1 {
      eprintln!("⚠️  Warning: Foreign keys not enabled");
    }

    println!("✅ SQLite configuration verified");
    Ok(())
  }

  pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
    // Get list of applied migrations (handle case where table doesn't exist yet)
    // Only get successfully applied migrations
    let applied = sqlx::query("SELECT version FROM _sqlx_migrations WHERE success = true")
      .fetch_all(&self.pool)
      .await
      .unwrap_or_else(|_| {
        // Table doesn't exist yet, so no migrations have been applied
        Vec::new()
      });
    let applied_versions: HashSet<_> = applied
      .iter()
      .map(|row| row.get::<i64, _>("version"))
      .collect();

    // Check for pending migrations and log them
    let mut pending_count = 0;
    // Note: In testing, we observed that MIGRATOR.iter() can return duplicate entries
    // for the same migration version. This deduplication ensures each migration
    // is only logged once, preventing confusing output like:
    // "Pending migration: 1 - create user roles"
    // "Pending migration: 1 - create user roles" (duplicate)
    let mut seen_versions = HashSet::new();
    for migration in MIGRATOR.iter() {
      if !applied_versions.contains(&migration.version)
        && !seen_versions.contains(&migration.version)
      {
        println!(
          "Pending migration: {} - {}",
          migration.version, migration.description
        );
        seen_versions.insert(migration.version);
        pending_count += 1;
      }
    }

    if pending_count == 0 {
      println!("No pending migrations found");
      return Ok(());
    }

    println!("Applying {pending_count} pending migration(s)...");

    // Run the migrations
    MIGRATOR.run(&self.pool).await?;

    println!("Successfully applied {pending_count} migration(s)");
    Ok(())
  }

  pub async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
    let seeds_dir = "config/database/seeds";

    // Check if seeds directory exists
    if !std::path::Path::new(seeds_dir).exists() {
      println!("No seeds directory found, skipping seeding");
      return Ok(());
    }

    // Read all .sql files in seeds directory
    let mut seed_files = fs::read_dir(seeds_dir)?
      .filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension()? == "sql" {
          Some(path)
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    // Sort files to ensure consistent execution order
    seed_files.sort();

    // Execute each seed file
    for seed_file in seed_files {
      let sql_content = fs::read_to_string(&seed_file)?;
      println!("Running seed: {}", seed_file.display());

      // Split by semicolon and execute each statement
      for (index, statement) in sql_content.split(';').enumerate() {
        let statement = statement.trim();

        if statement.is_empty() {
          continue;
        }

        // Remove comment lines and extract SQL
        let sql_lines: Vec<&str> = statement
          .lines()
          .filter(|line| !line.trim().is_empty() && !line.trim().starts_with("--"))
          .collect();

        if sql_lines.is_empty() {
          continue;
        }

        let clean_sql = sql_lines.join("\n").trim().to_string();

        match sqlx::query(&clean_sql).execute(&self.pool).await {
          Ok(result) => {
            println!(
              "  ✅ Statement {} executed successfully, rows affected: {}",
              index + 1,
              result.rows_affected()
            );
          }
          Err(e) => {
            eprintln!(
              "❌ Error executing statement {} in seed file: {}",
              index + 1,
              seed_file.display()
            );
            eprintln!("Error: {e}");
            eprintln!("SQL that failed:");
            eprintln!("--- START SQL ---");
            eprintln!("{clean_sql}");
            eprintln!("--- END SQL ---");
            return Err(Box::new(e));
          }
        }
      }
    }

    println!("✅ Seeds completed successfully");
    Ok(())
  }

  /// Generate schema dump content as a string
  pub async fn dump_schema_content(&self) -> Result<String, sqlx::Error> {
    // Query to get all table creation statements
    let tables = sqlx::query_scalar::<_, String>(
      "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&self.pool)
    .await?;

    // Query to get all index creation statements
    let indexes = sqlx::query_scalar::<_, String>(
          "SELECT sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL ORDER BY name"
      )
      .fetch_all(&self.pool)
      .await?;

    // Combine all SQL statements
    let mut schema_sql = String::new();
    schema_sql.push_str("-- Database Schema Dump\n");
    schema_sql.push_str("-- Generated automatically by dps-auth-api-migrate\n\n");

    // Add table creation statements
    for table_sql in tables {
      schema_sql.push_str(&table_sql);
      schema_sql.push_str(";\n\n");
    }

    // Add index creation statements
    for index_sql in indexes {
      schema_sql.push_str(&index_sql);
      schema_sql.push_str(";\n\n");
    }

    Ok(schema_sql)
  }

  /// Dump schema to a file at the specified path
  pub async fn dump_schema_to_file(
    &self,
    file_path: &str,
  ) -> Result<(), Box<dyn std::error::Error>> {
    // Generate schema dump using pure Rust/sqlx instead of sqlite3 command
    let schema_sql = self.dump_schema_content().await?;

    // Create directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(file_path).parent() {
      std::fs::create_dir_all(parent)?;
    }

    // Write schema to specified file
    std::fs::write(file_path, schema_sql)?;
    println!("✅ Schema dumped to {file_path}");

    Ok(())
  }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
  use super::Database;
  use sqlx::SqlitePool;
  use tempfile::NamedTempFile;

  pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");
    let pool = database.pool;

    // Configure SQLite settings (same as production)
    Database::configure_sqlite(&pool)
      .await
      .expect("Failed to configure SQLite");

    // Run migrations
    let database = Database { pool: pool.clone() };
    database.migrate().await.expect("Failed to run migrations");

    (pool, temp_file)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::NamedTempFile;

  #[tokio::test]
  async fn test_dump_schema_content() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Create test tables manually
    sqlx::query(
      "CREATE TABLE test_table_1 (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        created_at INTEGER NOT NULL
      )",
    )
    .execute(&database.pool)
    .await
    .expect("Failed to create test_table_1");

    sqlx::query(
      "CREATE TABLE test_table_2 (
        id INTEGER PRIMARY KEY,
        description TEXT,
        status TEXT DEFAULT 'active'
      )",
    )
    .execute(&database.pool)
    .await
    .expect("Failed to create test_table_2");

    // Test schema dump content generation
    let schema_content = database
      .dump_schema_content()
      .await
      .expect("Failed to dump schema content");

    // Verify the schema content contains expected elements
    assert!(schema_content.contains("-- Database Schema Dump"));
    assert!(schema_content.contains("-- Generated automatically by dps-auth-api-migrate"));

    // Should contain our test tables
    assert!(schema_content.contains("CREATE TABLE test_table_1"));
    assert!(schema_content.contains("CREATE TABLE test_table_2"));

    // Should contain proper SQL formatting
    assert!(schema_content.contains(";\n\n"));

    // Tables should be ordered alphabetically
    let table1_pos = schema_content.find("CREATE TABLE test_table_1").unwrap();
    let table2_pos = schema_content.find("CREATE TABLE test_table_2").unwrap();
    assert!(
      table1_pos < table2_pos,
      "Tables should be ordered alphabetically"
    );
  }

  #[tokio::test]
  async fn test_dump_schema_to_file() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Create test table manually
    sqlx::query(
      "CREATE TABLE test_products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        price DECIMAL(10,2),
        category_id INTEGER
      )",
    )
    .execute(&database.pool)
    .await
    .expect("Failed to create test_products table");

    // Create a temporary file for schema dump
    let schema_temp_file = NamedTempFile::new().expect("Failed to create temp schema file");
    let schema_file_path = schema_temp_file.path().display().to_string();

    // Test schema dump to file
    database
      .dump_schema_to_file(&schema_file_path)
      .await
      .expect("Failed to dump schema to file");

    // Verify the file was created and contains expected content
    assert!(std::path::Path::new(&schema_file_path).exists());

    let file_content =
      std::fs::read_to_string(&schema_file_path).expect("Failed to read schema file");

    assert!(file_content.contains("-- Database Schema Dump"));
    assert!(file_content.contains("CREATE TABLE test_products"));
    assert!(file_content.contains("AUTOINCREMENT"));
    assert!(file_content.contains("UNIQUE"));
  }

  #[tokio::test]
  async fn test_dump_schema_to_file_creates_directory() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Create test table manually
    sqlx::query(
      "CREATE TABLE test_orders (
        order_id INTEGER PRIMARY KEY,
        customer_name TEXT NOT NULL,
        order_date TEXT DEFAULT CURRENT_TIMESTAMP
      )",
    )
    .execute(&database.pool)
    .await
    .expect("Failed to create test_orders table");

    // Create a path with non-existent directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let schema_file_path = temp_dir.path().join("subdir").join("schema.sql");
    let schema_file_path_str = schema_file_path.display().to_string();

    // Test schema dump to file in non-existent directory
    database
      .dump_schema_to_file(&schema_file_path_str)
      .await
      .expect("Failed to dump schema to file");

    // Verify the directory and file were created
    assert!(schema_file_path.exists());

    let file_content =
      std::fs::read_to_string(&schema_file_path).expect("Failed to read schema file");

    assert!(file_content.contains("-- Database Schema Dump"));
    assert!(file_content.contains("CREATE TABLE test_orders"));
    assert!(file_content.contains("DEFAULT CURRENT_TIMESTAMP"));
  }

  #[tokio::test]
  async fn test_dump_schema_content_empty_database() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Test schema dump content generation on empty database
    let schema_content = database
      .dump_schema_content()
      .await
      .expect("Failed to dump schema content");

    // Should still contain headers even with no tables
    assert!(schema_content.contains("-- Database Schema Dump"));
    assert!(schema_content.contains("-- Generated automatically by dps-auth-api-migrate"));

    // Should not contain any CREATE TABLE statements
    assert!(!schema_content.contains("CREATE TABLE"));
  }

  #[tokio::test]
  async fn test_dump_schema_content_with_indexes() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Create test table manually
    sqlx::query(
      "CREATE TABLE test_customers (
        id INTEGER PRIMARY KEY,
        email TEXT NOT NULL,
        name TEXT NOT NULL,
        created_at INTEGER NOT NULL
      )",
    )
    .execute(&database.pool)
    .await
    .expect("Failed to create test_customers table");

    // Create test indexes manually
    sqlx::query("CREATE INDEX idx_customers_email ON test_customers(email)")
      .execute(&database.pool)
      .await
      .expect("Failed to create email index");

    sqlx::query("CREATE UNIQUE INDEX idx_customers_email_unique ON test_customers(email)")
      .execute(&database.pool)
      .await
      .expect("Failed to create unique email index");

    // Test schema dump content generation
    let schema_content = database
      .dump_schema_content()
      .await
      .expect("Failed to dump schema content");

    // Should contain the table
    assert!(schema_content.contains("CREATE TABLE test_customers"));

    // Should contain both indexes
    assert!(schema_content.contains("CREATE INDEX idx_customers_email ON test_customers(email)"));
    assert!(schema_content
      .contains("CREATE UNIQUE INDEX idx_customers_email_unique ON test_customers(email)"));

    // Indexes should come after tables in the output
    let table_pos = schema_content.find("CREATE TABLE test_customers").unwrap();
    let index_pos = schema_content
      .find("CREATE INDEX idx_customers_email")
      .unwrap();
    assert!(
      table_pos < index_pos,
      "Tables should come before indexes in schema dump"
    );
  }
}

// TODO: tests for creating a Database with different pool sizes
