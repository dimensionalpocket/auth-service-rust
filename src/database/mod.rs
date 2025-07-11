use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use std::{env, fs};

pub struct Database {
  pub pool: SqlitePool,
}

impl Database {
  pub async fn new() -> Result<Self, sqlx::Error> {
    let database_url =
      env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/development.db".to_string());

    // Create database if it doesn't exist
    if !Sqlite::database_exists(&database_url)
      .await
      .unwrap_or(false)
    {
      Sqlite::create_database(&database_url).await?;
    }

    let pool = SqlitePool::connect(&database_url).await?;

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

    // Set cache size to 64MB (negative value = KB, positive = pages)
    // Larger cache improves performance for read-heavy workloads
    // "PRAGMA cache_size = -65536;",

    // Enable memory-mapped I/O for better performance
    // Uses 256MB of memory-mapped I/O
    // "PRAGMA mmap_size = 268435456;",

    // Set temp store to memory for temporary tables and indices
    // Faster than disk-based temporary storage
    // "PRAGMA temp_store = MEMORY;",

    // Optimize for faster writes at the cost of some durability
    // Only use in development; consider removing in production
    // "PRAGMA wal_autocheckpoint = 1000;",
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
      for statement in sql_content.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() && !statement.starts_with("--") {
          sqlx::query(statement).execute(&self.pool).await?;
        }
      }
    }

    println!("✅ Seeds completed successfully");
    Ok(())
  }
}

#[cfg(test)]
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
