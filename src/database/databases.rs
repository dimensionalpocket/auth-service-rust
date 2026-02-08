use sqlx::SqlitePool;

/// Application database pools.
///
/// This is injected into the async-graphql schema context so all resolvers
/// can access all databases.
#[derive(Clone)]
pub struct Databases {
  main: SqlitePool,
  session: SqlitePool,
}

impl Databases {
  pub fn new(main: SqlitePool, session: SqlitePool) -> Self {
    Self { main, session }
  }

  pub fn main(&self) -> &SqlitePool {
    &self.main
  }

  pub fn session(&self) -> &SqlitePool {
    &self.session
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::{MainDatabase, SessionDatabase};
  use tempfile::NamedTempFile;

  #[tokio::test]
  async fn test_databases_creation_and_accessors() {
    let main_file = NamedTempFile::new().expect("Failed to create temp file");
    let session_file = NamedTempFile::new().expect("Failed to create temp file");

    let main_db = MainDatabase::new(&main_file.path().display().to_string())
      .await
      .expect("Failed to create main database");
    let session_db = SessionDatabase::new(&session_file.path().display().to_string())
      .await
      .expect("Failed to create session database");

    let databases = Databases::new(main_db.pool, session_db.pool);

    // Basic smoke checks that pools are usable.
    sqlx::query("SELECT 1")
      .execute(databases.main())
      .await
      .expect("Main pool should execute query");
    sqlx::query("SELECT 1")
      .execute(databases.session())
      .await
      .expect("Session pool should execute query");
  }

  #[tokio::test]
  async fn test_databases_pools_are_separate() {
    let main_file = NamedTempFile::new().expect("Failed to create temp file");
    let session_file = NamedTempFile::new().expect("Failed to create temp file");

    let main_db = MainDatabase::new(&main_file.path().display().to_string())
      .await
      .expect("Failed to create main database");
    let session_db = SessionDatabase::new(&session_file.path().display().to_string())
      .await
      .expect("Failed to create session database");

    let databases = Databases::new(main_db.pool, session_db.pool);

    sqlx::query("CREATE TABLE only_in_main (id INTEGER PRIMARY KEY)")
      .execute(databases.main())
      .await
      .expect("Failed to create table in main");

    // Table should not exist in session.
    let err = sqlx::query("SELECT * FROM only_in_main")
      .execute(databases.session())
      .await
      .expect_err("Expected table to be absent in session database");
    // Provide a small assertion to avoid unused variable warnings and make intent clear.
    let _ = err;
  }

  #[tokio::test]
  async fn test_databases_clone_and_concurrent_access() {
    let main_file = NamedTempFile::new().expect("Failed to create temp file");
    let session_file = NamedTempFile::new().expect("Failed to create temp file");

    let main_db = MainDatabase::new(&main_file.path().display().to_string())
      .await
      .expect("Failed to create main database");
    let session_db = SessionDatabase::new(&session_file.path().display().to_string())
      .await
      .expect("Failed to create session database");

    let databases = Databases::new(main_db.pool, session_db.pool);
    let databases_2 = databases.clone();

    let handle_1 = tokio::spawn(async move {
      for _ in 0..10 {
        sqlx::query("SELECT 1")
          .execute(databases.main())
          .await
          .expect("Main query should succeed");
      }
    });

    let handle_2 = tokio::spawn(async move {
      for _ in 0..10 {
        sqlx::query("SELECT 1")
          .execute(databases_2.session())
          .await
          .expect("Session query should succeed");
      }
    });

    handle_1.await.expect("Task 1 should complete");
    handle_2.await.expect("Task 2 should complete");
  }
}
