use crate::models::User;
use sqlx::SqliteConnection;

pub struct GetUserByNameQuery;

impl GetUserByNameQuery {
  pub async fn run(conn: &mut SqliteConnection, name: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE name = ? COLLATE NOCASE"
    )
    .bind(name)
    .fetch_optional(&mut *conn)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role_model};
  use uuid::Uuid;

  #[tokio::test]
  async fn test_get_user_by_name_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role = create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;
    let role_id = role.id;

    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind(&user_uuid)
      .bind("TestUser")
      .bind(role_id)
      .bind("hashed_password")
      .bind(None::<String>)
      .bind(1234567890)
      .bind(1234567890)
      .execute(&pool)
      .await
      .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let user = GetUserByNameQuery::run(&mut conn, "TestUser")
      .await
      .unwrap();

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
    let role = create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;
    let role_id = role.id;

    // Insert test user with mixed case
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind(&user_uuid)
      .bind("TestUser")
      .bind(role_id)
      .bind("hashed_password")
      .bind(None::<String>)
      .bind(1234567890)
      .bind(1234567890)
      .execute(&pool)
      .await
      .unwrap();

    // Should find user regardless of case
    let mut conn = pool.acquire().await.unwrap();
    let user_lower = GetUserByNameQuery::run(&mut conn, "testuser")
      .await
      .unwrap();
    let user_upper = GetUserByNameQuery::run(&mut conn, "TESTUSER")
      .await
      .unwrap();
    let user_mixed = GetUserByNameQuery::run(&mut conn, "tEsTuSeR")
      .await
      .unwrap();

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

    let mut conn = pool.acquire().await.unwrap();
    let user = GetUserByNameQuery::run(&mut conn, "NonExistentUser")
      .await
      .unwrap();

    assert!(user.is_none());
  }
}
