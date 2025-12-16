use crate::models::User;
use sqlx::SqliteConnection;

pub struct GetUserByUuidQuery;

impl GetUserByUuidQuery {
  pub async fn run(conn: &mut SqliteConnection, uuid: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE uuid = ?"
    )
    .bind(uuid)
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
  async fn test_get_user_by_uuid_found() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Insert test role first
    let role = create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
    let role_id = role.id;

    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind(&user_uuid)
      .bind("Test User")
      .bind(role_id)
      .bind("hashed_password")
      .bind(None::<String>)
      .bind(1234567890)
      .bind(1234567890)
      .execute(&mut *conn)
      .await
      .unwrap();

    let user = GetUserByUuidQuery::run(&mut conn, &user_uuid)
      .await
      .unwrap();

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
    let mut conn = pool.acquire().await.unwrap();

    let user_uuid = Uuid::new_v4().to_string();
    let user = GetUserByUuidQuery::run(&mut conn, &user_uuid)
      .await
      .unwrap();

    assert!(user.is_none());
  }
}
