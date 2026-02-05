use sqlx::migrate::Migrator;
use sqlx::SqlitePool;

#[allow(async_fn_in_trait)]
pub trait Database {
  fn pool(&self) -> &SqlitePool;

  fn migrations_dir() -> &'static str;
  fn seeds_dir() -> &'static str;

  fn pragma_commands() -> &'static [&'static str];

  fn migrator() -> &'static Migrator;

  async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError>;
  async fn revert(&self, steps: usize) -> Result<usize, sqlx::migrate::MigrateError>;
  async fn seed(&self) -> Result<(), Box<dyn std::error::Error>>;
  async fn dump_schema_content(&self) -> Result<String, sqlx::Error>;
  async fn dump_schema_to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>>;
}
