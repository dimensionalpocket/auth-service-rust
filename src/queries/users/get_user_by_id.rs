use crate::models::User;
use sqlx::SqlitePool;

pub struct GetUserByIdQuery;

impl GetUserByIdQuery {
  pub async fn run(pool: &SqlitePool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::test_utils::create_test_database;
  use uuid::Uuid;

  #[tokio::test]
  async fn test_get_user_by_id_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role_result = sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let role_id = role_result.last_insert_rowid();

    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    let user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'Test User', ?, 'hashed_password', '{\"test\": true}')"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();

    let user_id = user_result.last_insert_rowid();
    let user = GetUserByIdQuery::run(&pool, user_id).await.unwrap();

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.id, user_id);
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test User");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.metadata_json, Some("{\"test\": true}".to_string()));
  }

  #[tokio::test]
  async fn test_get_user_by_id_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let user = GetUserByIdQuery::run(&pool, 999).await.unwrap();

    assert!(user.is_none());
  }
}
