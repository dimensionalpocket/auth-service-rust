use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetDefaultUserRoleQuery;

impl GetDefaultUserRoleQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Option<UserRole>, sqlx::Error> {
    sqlx::query_as::<_, UserRole>(
      "SELECT id, name, created_ts, is_default, permissions_json FROM user_roles WHERE is_default = TRUE LIMIT 1"
    )
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_default_user_role_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test roles with one default
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)")
      .execute(&pool)
      .await
      .unwrap();
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert_eq!(role.is_default, true);
  }

  #[tokio::test]
  async fn test_get_default_user_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test roles with no default
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('moderator', 1234567891, FALSE)")
      .execute(&pool)
      .await
      .unwrap();
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_default_user_role_empty_table() {
    let (pool, _temp_file) = create_test_database().await;
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_none());
  }
}