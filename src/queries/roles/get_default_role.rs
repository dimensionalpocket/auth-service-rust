use crate::models::Role;
use sqlx::SqlitePool;

pub struct GetDefaultRoleQuery;

impl GetDefaultRoleQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE is_default = TRUE LIMIT 1"
    )
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_default_role_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with one default
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE), ('user', 1234567891, 1234567891, TRUE)")
      .execute(&pool)
      .await
      .unwrap();

    let role = GetDefaultRoleQuery::run(&pool).await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.is_default);
  }

  #[tokio::test]
  async fn test_get_default_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with no default
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE), ('moderator', 1234567891, 1234567891, FALSE)")
      .execute(&pool)
      .await
      .unwrap();

    let role = GetDefaultRoleQuery::run(&pool).await.unwrap();

    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_default_role_empty_table() {
    let (pool, _temp_file) = create_test_database().await;

    let role = GetDefaultRoleQuery::run(&pool).await.unwrap();

    assert!(role.is_none());
  }
}
