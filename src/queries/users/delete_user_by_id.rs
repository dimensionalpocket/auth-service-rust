use sqlx::SqlitePool;

pub struct DeleteUserByIdQuery;

impl DeleteUserByIdQuery {
  pub async fn run(pool: &SqlitePool, user_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
      .bind(user_id)
      .execute(pool)
      .await?;

    Ok(result.rows_affected() > 0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use sqlx::Row;
  use uuid::Uuid;

  #[tokio::test]
  async fn test_delete_user_by_id_success() {
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
    let user_result = sqlx::query(
            "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'Test User', ?, 'hashed_password', '{\"test\": true}')"
        )
        .bind(&user_uuid)
        .bind(role_id)
        .execute(&pool)
        .await
        .unwrap();

    let user_id = user_result.last_insert_rowid();

    // Verify user exists before deletion
    let user_exists = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(user_id)
      .fetch_one(&pool)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(user_exists, 1);

    // Delete the user
    let deleted = DeleteUserByIdQuery::run(&pool, user_id).await.unwrap();
    assert!(deleted);

    // Verify user is deleted
    let user_exists = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(user_id)
      .fetch_one(&pool)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(user_exists, 0);
  }

  #[tokio::test]
  async fn test_delete_user_by_id_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Try to delete non-existent user
    let deleted = DeleteUserByIdQuery::run(&pool, 999).await.unwrap();
    assert!(!deleted);
  }

  #[tokio::test]
  async fn test_delete_user_by_id_multiple_deletes() {
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
    let user_result = sqlx::query(
            "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'Test User', ?, 'hashed_password', '{\"test\": true}')"
        )
        .bind(&user_uuid)
        .bind(role_id)
        .execute(&pool)
        .await
        .unwrap();

    let user_id = user_result.last_insert_rowid();

    // First deletion should succeed
    let deleted_first = DeleteUserByIdQuery::run(&pool, user_id).await.unwrap();
    assert!(deleted_first);

    // Second deletion should fail (user already deleted)
    let deleted_second = DeleteUserByIdQuery::run(&pool, user_id).await.unwrap();
    assert!(!deleted_second);
  }
}
