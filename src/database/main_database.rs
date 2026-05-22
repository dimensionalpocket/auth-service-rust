use crate::database::sqlite_database::SqliteDatabaseCore;
use crate::types::Database;
use sqlx::migrate::Migrator;
use sqlx::SqlitePool;

static MIGRATOR: Migrator = sqlx::migrate!("./config/databases/main/migrations");

pub struct MainDatabase {
  pub pool: SqlitePool,
}

impl MainDatabase {
  pub async fn new(sqlite_file_path: &str) -> Result<Self, sqlx::Error> {
    Self::new_with_pool_size(sqlite_file_path, None).await
  }

  pub async fn new_with_pool_size(
    sqlite_file_path: &str,
    pool_size: Option<u32>,
  ) -> Result<Self, sqlx::Error> {
    println!("🔧 Configuring SQLite database: {sqlite_file_path}");
    let core = SqliteDatabaseCore::new_with_pool_size(
      sqlite_file_path,
      pool_size,
      Self::SQLITE_PRAGMA_COMMANDS,
    )
    .await?;

    Ok(MainDatabase { pool: core.pool })
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
    SqliteDatabaseCore::configure_sqlite(pool, Self::SQLITE_PRAGMA_COMMANDS).await
  }

  pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
    SqliteDatabaseCore::migrate(&self.pool, &MIGRATOR).await
  }

  /// Revert the last N migrations
  ///
  /// This executes the `.down.sql` files in reverse order.
  /// Each revert is transactional and will roll back on error.
  ///
  /// # Arguments
  /// * `steps` - Number of migrations to revert (default: 1)
  ///
  /// # Returns
  /// * `Ok(usize)` - Number of migrations successfully reverted
  /// * `Err` - If any migration revert fails
  pub async fn revert(&self, steps: usize) -> Result<usize, sqlx::migrate::MigrateError> {
    SqliteDatabaseCore::revert(&self.pool, &MIGRATOR, steps).await
  }

  pub async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
    SqliteDatabaseCore::seed(&self.pool, "config/databases/main/seeds").await
  }

  /// Generate schema dump content as a string
  pub async fn dump_schema_content(&self) -> Result<String, sqlx::Error> {
    SqliteDatabaseCore::dump_schema_content(&self.pool).await
  }

  /// Dump schema to a file at the specified path
  pub async fn dump_schema_to_file(
    &self,
    file_path: &str,
  ) -> Result<(), Box<dyn std::error::Error>> {
    SqliteDatabaseCore::dump_schema_to_file(&self.pool, file_path).await
  }
}

impl Database for MainDatabase {
  fn pool(&self) -> &SqlitePool {
    &self.pool
  }

  fn migrations_dir() -> &'static str {
    "config/databases/main/migrations"
  }

  fn seeds_dir() -> &'static str {
    "config/databases/main/seeds"
  }

  fn pragma_commands() -> &'static [&'static str] {
    Self::SQLITE_PRAGMA_COMMANDS
  }

  fn migrator() -> &'static Migrator {
    &MIGRATOR
  }

  async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
    MainDatabase::migrate(self).await
  }

  async fn revert(&self, steps: usize) -> Result<usize, sqlx::migrate::MigrateError> {
    MainDatabase::revert(self, steps).await
  }

  async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
    MainDatabase::seed(self).await
  }

  async fn dump_schema_content(&self) -> Result<String, sqlx::Error> {
    MainDatabase::dump_schema_content(self).await
  }

  async fn dump_schema_to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    MainDatabase::dump_schema_to_file(self, file_path).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use tempfile::NamedTempFile;

  #[dps_auth_db_test]
  async fn test_dump_schema_content() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
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

  #[dps_auth_db_test]
  async fn test_dump_schema_to_file() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
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

  #[dps_auth_db_test]
  async fn test_dump_schema_to_file_creates_directory() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
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

  #[dps_auth_db_test]
  async fn test_dump_schema_content_empty_database() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
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

  #[dps_auth_db_test]
  async fn test_dump_schema_content_with_indexes() {
    // Create a test database without migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
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

  #[dps_auth_db_test]
  async fn test_revert_single_migration() {
    use tempfile::NamedTempFile;

    // Create test database and run migrations
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    database.migrate().await.expect("Failed to run migrations");

    // Verify migrations were applied
    let applied_before: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    assert!(applied_before > 0, "No migrations were applied");

    // Revert one migration
    let reverted = database
      .revert(1)
      .await
      .expect("Failed to revert migration");
    assert_eq!(reverted, 1, "Expected to revert 1 migration");

    // Verify one migration was reverted (success != 0 means still applied)
    let applied_after: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    // The count should have decreased by the number of reverted migrations
    assert!(
      applied_after < applied_before,
      "Migration count should decrease after revert. Before: {applied_before}, After: {applied_after}"
    );
    assert_eq!(
      applied_before - applied_after,
      1,
      "Expected exactly 1 migration to be reverted"
    );
  }

  #[dps_auth_db_test]
  async fn test_revert_multiple_migrations() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    database.migrate().await.expect("Failed to run migrations");

    let applied_before: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    // Revert 2 migrations
    let reverted = database
      .revert(2)
      .await
      .expect("Failed to revert migrations");
    assert_eq!(reverted, 2, "Expected to revert 2 migrations");

    let applied_after: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    assert_eq!(
      applied_after,
      applied_before - 2,
      "Expected two less applied migrations"
    );
  }

  #[dps_auth_db_test]
  async fn test_revert_more_than_available() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    database.migrate().await.expect("Failed to run migrations");

    let applied_before: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    // Try to revert more migrations than exist
    let reverted = database
      .revert(100)
      .await
      .expect("Failed to revert migrations");

    // Should only revert the number of available migrations
    assert_eq!(
      reverted, applied_before as usize,
      "Should revert all available migrations"
    );

    let applied_after: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success != 0")
        .fetch_one(&database.pool)
        .await
        .expect("Failed to count migrations");

    // All user migrations should be reverted (may have 0 or 1 remaining for schema itself)
    assert!(
      applied_after <= 1,
      "All user migrations should be reverted. Remaining: {applied_after}"
    );
    assert_eq!(
      applied_before - applied_after,
      reverted as i64,
      "Count reduction should match number reverted"
    );
  }

  #[dps_auth_db_test]
  async fn test_revert_empty_database() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = MainDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Don't run migrations - try to revert on empty database
    let reverted = database
      .revert(1)
      .await
      .expect("Failed to handle empty revert");

    assert_eq!(reverted, 0, "Should not revert anything on empty database");
  }
}

// TODO: tests for creating a Database with different pool sizes
