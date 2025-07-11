use crate::models::User;
use sqlx::SqlitePool;

pub struct GetUserByNameQuery;

impl GetUserByNameQuery {
  pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE name = ? COLLATE NOCASE"
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
  use uuid::Uuid;

  #[tokio::test]
  async fn test_get_user_by_name_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role_result = sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let role_id = role_result.last_insert_rowid();

    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'TestUser', ?, 'hashed_password', NULL)"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();

    let user = GetUserByNameQuery::run(&pool, "TestUser").await.unwrap();

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "TestUser");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.metadata_json, None);
  }

  #[tokio::test]
  async fn test_get_user_by_name_case_insensitive() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role_result = sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let role_id = role_result.last_insert_rowid();

    // Insert test user with mixed case
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'TestUser', ?, 'hashed_password', NULL)"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();

    // Should find user regardless of case
    let user_lower = GetUserByNameQuery::run(&pool, "testuser").await.unwrap();
    let user_upper = GetUserByNameQuery::run(&pool, "TESTUSER").await.unwrap();
    let user_mixed = GetUserByNameQuery::run(&pool, "tEsTuSeR").await.unwrap();

    assert!(user_lower.is_some());
    assert!(user_upper.is_some());
    assert!(user_mixed.is_some());

    assert_eq!(user_lower.unwrap().name, "TestUser");
    assert_eq!(user_upper.unwrap().name, "TestUser");
    assert_eq!(user_mixed.unwrap().name, "TestUser");
  }

  #[tokio::test]
  async fn test_get_user_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let user = GetUserByNameQuery::run(&pool, "NonExistentUser")
      .await
      .unwrap();

    assert!(user.is_none());
  }
}
