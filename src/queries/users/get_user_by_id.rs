use crate::models::User;
use sqlx::SqliteConnection;

pub struct GetUserByIdQuery;

impl GetUserByIdQuery {
  pub async fn run(
    main_conn: &mut SqliteConnection,
    user_id: i64,
  ) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&mut *main_conn)
    .await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::create_test_role_model_with_databases;
  use uuid::Uuid;

  #[dps_auth_db_test]
  async fn test_get_user_by_id_found() {
    // Insert test role first
    let role =
      create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true)
        .await;
    let role_id = role.id;

    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    let user_id = sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind(&user_uuid)
      .bind("Test User")
      .bind(role_id)
      .bind("hashed_password")
      .bind("{\"test\": true}")
      .bind(1234567890)
      .bind(1234567890)
      .execute(&main_pool)
      .await
      .unwrap()
      .last_insert_rowid();
    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .unwrap();

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.id, user_id);
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test User");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.metadata_json, Some("{\"test\": true}".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_id_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = GetUserByIdQuery::run(&mut main_conn, 999).await.unwrap();

    assert!(user.is_none());
  }
}
