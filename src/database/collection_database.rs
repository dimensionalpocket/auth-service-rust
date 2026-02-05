use crate::database::sqlite_database::SqliteDatabaseCore;
use crate::types::Database;
use sqlx::migrate::Migrator;
use sqlx::SqlitePool;

static MIGRATOR: Migrator = sqlx::migrate!("./config/databases/collection/migrations");

pub struct CollectionDatabase {
  pub pool: SqlitePool,
}

impl CollectionDatabase {
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
    Ok(Self { pool: core.pool })
  }

  pub const SQLITE_PRAGMA_COMMANDS: &'static [&'static str] = &[
    "PRAGMA journal_mode = WAL;",
    "PRAGMA foreign_keys = ON;",
    "PRAGMA synchronous = NORMAL;",
    "PRAGMA cache_size = -2000;",
    "PRAGMA page_size = 4096;",
    "PRAGMA mmap_size = 1048576;",
    "PRAGMA journal_size_limit = 27103364;",
  ];
}

impl Database for CollectionDatabase {
  fn pool(&self) -> &SqlitePool {
    &self.pool
  }

  fn migrations_dir() -> &'static str {
    "config/databases/collection/migrations"
  }

  fn seeds_dir() -> &'static str {
    "config/databases/collection/seeds"
  }

  fn pragma_commands() -> &'static [&'static str] {
    Self::SQLITE_PRAGMA_COMMANDS
  }

  fn migrator() -> &'static Migrator {
    &MIGRATOR
  }

  async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
    SqliteDatabaseCore::migrate(&self.pool, Self::migrator()).await
  }

  async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
    SqliteDatabaseCore::seed(&self.pool, Self::seeds_dir()).await
  }

  async fn dump_schema_content(&self) -> Result<String, sqlx::Error> {
    SqliteDatabaseCore::dump_schema_content(&self.pool).await
  }

  async fn dump_schema_to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    SqliteDatabaseCore::dump_schema_to_file(&self.pool, file_path).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::NamedTempFile;

  #[tokio::test]
  async fn test_collection_database_create_and_dump_schema() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = CollectionDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create collection database");

    let schema_content = database
      .dump_schema_content()
      .await
      .expect("Failed to dump schema content");
    assert!(schema_content.contains("-- Database Schema Dump"));
  }

  #[tokio::test]
  async fn test_collection_database_migrate_noop_succeeds() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = CollectionDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create collection database");

    database
      .migrate()
      .await
      .expect("Collection migrations should succeed even if empty");
  }

  #[tokio::test]
  async fn test_collection_database_seed_noop_succeeds() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = CollectionDatabase::new(&sqlite_file_path)
      .await
      .expect("Failed to create collection database");

    database
      .seed()
      .await
      .expect("Collection seed should succeed even if empty");
  }
}
