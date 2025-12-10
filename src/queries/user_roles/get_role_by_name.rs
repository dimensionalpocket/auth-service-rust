use crate::models::UserRole;
use sqlx::SqlitePool;

pub struct GetRoleByNameQuery;

impl GetRoleByNameQuery {
  pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<UserRole>, sqlx::Error> {
    sqlx::query_as::<_, UserRole>(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM user_roles WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_role_by_name_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let role = GetRoleByNameQuery::run(&pool, "admin").await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert_eq!(role.created_ts, 1234567890);
    assert!(!role.is_default);
  }

  #[tokio::test]
  async fn test_get_role_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let role = GetRoleByNameQuery::run(&pool, "nonexistent").await.unwrap();

    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_role_by_name_case_sensitive() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Should not find with different case
    let role = GetRoleByNameQuery::run(&pool, "ADMIN").await.unwrap();
    assert!(role.is_none());
  }
}
