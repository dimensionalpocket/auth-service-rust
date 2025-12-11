/*
 * Shared test utilities for creating test data.
 * 
 * This module provides centralized functions for creating test users, roles, and other entities.
 * All helpers use existing Query objects to ensure consistency with application behavior.
 * 
 * Do NOT create local create_test_* functions in test modules - use these shared utilities instead.
 */

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
  use crate::database::Database;
  use sqlx::SqlitePool;
  use tempfile::NamedTempFile;

  pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    create_test_database_with_config(true).await
  }

  pub async fn create_test_database_with_pool_size(pool_size: u32) -> (SqlitePool, NamedTempFile) {
    create_test_database_with_config_and_pool_size(true, pool_size).await
  }

  pub async fn create_test_database_with_config(
    configure_sqlite: bool,
  ) -> (SqlitePool, NamedTempFile) {
    create_test_database_with_config_and_pool_size(configure_sqlite, 1).await
  }

  pub async fn create_test_database_with_config_and_pool_size(
    configure_sqlite: bool,
    pool_size: u32,
  ) -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new_with_pool_size(&sqlite_file_path, Some(pool_size))
      .await
      .expect("Failed to create test database");
    let pool = database.pool;

    // Optionally configure SQLite settings
    if configure_sqlite {
      Database::configure_sqlite(&pool)
        .await
        .expect("Failed to configure SQLite");
    }

    // Run migrations
    let database = Database { pool: pool.clone() };
    database.migrate().await.expect("Failed to run migrations");

    (pool, temp_file)
  }
}