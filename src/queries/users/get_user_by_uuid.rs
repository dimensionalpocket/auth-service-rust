use crate::models::User;
use sqlx::SqlitePool;

pub struct GetUserByUuidQuery;

impl GetUserByUuidQuery {
  pub async fn run(pool: &SqlitePool, uuid: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE uuid = ?"
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use uuid::Uuid;

  #[tokio::test]
  async fn test_get_user_by_uuid_found() {
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
    sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'Test User', ?, 'hashed_password', NULL)"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();

    let user = GetUserByUuidQuery::run(&pool, &user_uuid).await.unwrap();

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test User");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.metadata_json, None);
  }

  #[tokio::test]
  async fn test_get_user_by_uuid_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let user_uuid = Uuid::new_v4().to_string();
    let user = GetUserByUuidQuery::run(&pool, &user_uuid).await.unwrap();

    assert!(user.is_none());
  }
}
