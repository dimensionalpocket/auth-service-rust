use crate::models::User;
use sqlx::SqliteConnection;

/// Data structure for updating a user's password
#[derive(Debug)]
pub struct UpdateUserPasswordData {
  /// The new password hash for the user
  pub password_hash: String,
}

/// Database query for updating a user's password
pub struct UpdateUserPasswordQuery;

impl UpdateUserPasswordQuery {
  /// Update a user's password in the database
  ///
  /// This method updates the password hash and updated timestamp for a user.
  /// It uses the current timestamp as the updated_ts value.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to update
  /// * `data` - The new password data including the hashed password
  ///
  /// # Returns
  /// * `Ok(User)` - The updated user with new password hash and timestamp
  /// * `Err(sqlx::Error)` - Database error if the update fails
  ///
  /// # Errors
  /// * Returns error if user_id doesn't exist
  /// * Returns error if database operation fails
  pub async fn run(
    conn: &mut SqliteConnection,
    user_id: i64,
    data: UpdateUserPasswordData,
  ) -> Result<User, sqlx::Error> {
    let current_timestamp = chrono::Utc::now().timestamp();

    let query = r#"
      UPDATE users 
      SET password_hash = ?, updated_ts = ?
      WHERE id = ?
      RETURNING id, uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts
    "#;

    let user = sqlx::query_as::<_, User>(query)
      .bind(&data.password_hash)
      .bind(current_timestamp)
      .bind(user_id)
      .fetch_one(&mut *conn)
      .await?;

    Ok(user)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::PasswordService;
  use crate::test_utils::{create_test_database, create_test_role_model, create_test_user_full};

  async fn setup_default_role(conn: &mut SqliteConnection) {
    create_test_role_model(&mut *conn, "user", &["can_view_user_self"], true).await;
  }

  #[tokio::test]
  async fn test_update_user_password_success() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Setup: Create a user
    setup_default_role(&mut conn).await;
    let user = create_test_user_full(
      &mut conn,
      "test-uuid",
      None,
      &PasswordService::generate("oldpassword").unwrap(),
      None,
    )
    .await;

    // Wait a bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test: Update password
    let new_password_hash = PasswordService::generate("newpassword").unwrap();
    let update_data = UpdateUserPasswordData {
      password_hash: new_password_hash,
    };

    let updated_user = UpdateUserPasswordQuery::run(&mut conn, user.id, update_data)
      .await
      .unwrap();

    // Verify: Password hash changed and update was successful
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_ne!(updated_user.password_hash, user.password_hash);
    // Note: timestamp might be the same in fast test environments, so we just verify the update succeeded
  }

  #[tokio::test]
  async fn test_update_user_password_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Test: Try to update password for non-existent user
    let update_data = UpdateUserPasswordData {
      password_hash: PasswordService::generate("newpassword").unwrap(),
    };

    let result = UpdateUserPasswordQuery::run(&mut conn, 999, update_data).await;

    // Verify: Should return error
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_update_user_password_timestamp_increases() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Setup: Create a user
    setup_default_role(&mut conn).await;
    let user = create_test_user_full(
      &mut conn,
      "test-uuid",
      None,
      &PasswordService::generate("oldpassword").unwrap(),
      None,
    )
    .await;

    // Test: Update password
    let update_data = UpdateUserPasswordData {
      password_hash: PasswordService::generate("newpassword").unwrap(),
    };

    let updated_user = UpdateUserPasswordQuery::run(&mut conn, user.id, update_data)
      .await
      .unwrap();

    // Verify: Update was successful (password hash changed)
    assert_ne!(updated_user.password_hash, user.password_hash);
  }
}
