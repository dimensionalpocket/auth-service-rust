use sqlx::SqliteConnection;

pub struct DeleteUserByIdQuery;

impl DeleteUserByIdQuery {
  pub async fn run(main_conn: &mut SqliteConnection, user_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
      .bind(user_id)
      .execute(&mut *main_conn)
      .await?;

    Ok(result.rows_affected() > 0)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use sqlx::Row;
  use uuid::Uuid;

  #[dps_auth_db_test]
  async fn test_delete_user_by_id_success() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    // Insert test role first
    let role_result = sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&mut *main_conn)
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
        .execute(&mut *main_conn)
        .await
        .unwrap();

    let user_id = user_result.last_insert_rowid();

    // Verify user exists before deletion
    let user_exists = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(user_id)
      .fetch_one(&mut *main_conn)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(user_exists, 1);

    // Delete the user
    let deleted = DeleteUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .unwrap();
    assert!(deleted);

    // Verify user is deleted - reuse same connection
    let user_exists = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(user_id)
      .fetch_one(&mut *main_conn)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(user_exists, 0);
  }

  #[dps_auth_db_test]
  async fn test_delete_user_by_id_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    // Try to delete non-existent user
    let deleted = DeleteUserByIdQuery::run(&mut main_conn, 999).await.unwrap();
    assert!(!deleted);
  }

  #[dps_auth_db_test]
  async fn test_delete_user_by_id_multiple_deletes() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    // Insert test role first
    let role_result = sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&mut *main_conn)
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
        .execute(&mut *main_conn)
        .await
        .unwrap();

    let user_id = user_result.last_insert_rowid();

    // First deletion should succeed
    let deleted_first = DeleteUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .unwrap();
    assert!(deleted_first);

    // Second deletion should fail (user already deleted)
    let deleted_second = DeleteUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .unwrap();
    assert!(!deleted_second);
  }
}
