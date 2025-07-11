use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetRoleByIdQuery;

impl GetRoleByIdQuery {
  pub async fn run(pool: &SqlitePool, role_id: i64) -> Result<Option<UserRole>, sqlx::Error> {
    sqlx::query_as::<_, UserRole>(
      "SELECT id, name, created_ts, is_default, permissions_json FROM user_roles WHERE id = ?"
    )
    .bind(role_id)
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_role_by_id_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test role
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE)")
      .execute(&pool)
      .await
      .unwrap();
    
    let role_id = result.last_insert_rowid();
    let role = GetRoleByIdQuery::run(&pool, role_id).await.unwrap();
    
    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.id, role_id);
    assert_eq!(role.name, "admin");
    assert_eq!(role.created_ts, 1234567890);
    assert_eq!(role.is_default, false);
  }

  #[tokio::test]
  async fn test_get_role_by_id_not_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    let role = GetRoleByIdQuery::run(&pool, 999).await.unwrap();
    
    assert!(role.is_none());
  }
}