use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use std::fs;

pub struct Database {
  pub pool: SqlitePool,
}

impl Database {
  pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {

    // Create database if it doesn't exist
    if !Sqlite::database_exists(database_url)
      .await
      .unwrap_or(false)
    {
      Sqlite::create_database(database_url).await?;
    }

    let pool = SqlitePool::connect(database_url).await?;

    // Configure SQLite settings after connection
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
      println!("Executing SQLite configuration: {command}");
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
    sqlx::migrate!("./config/database/migrations")
      .run(&self.pool)
      .await
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
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
  use super::Database;
  use sqlx::SqlitePool;
  use tempfile::NamedTempFile;

  pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let database_url = format!("sqlite:{}", temp_file.path().display());

    let pool = SqlitePool::connect(&database_url)
      .await
      .expect("Failed to connect to test database");

    // Configure SQLite settings (same as production)
    Database::configure_sqlite(&pool)
      .await
      .expect("Failed to configure SQLite");

    // Run migrations
    sqlx::migrate!("./config/database/migrations")
      .run(&pool)
      .await
      .expect("Failed to run migrations");

    (pool, temp_file)
  }
}
