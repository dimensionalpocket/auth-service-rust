use crate::models::User;
use crate::queries::users::{GetUserByIdQuery, UpdateUserPasswordData, UpdateUserPasswordQuery};
use crate::services::{GeneratePasswordHashService, VerifyPasswordService};
use crate::types::UserError;
use sqlx::SqliteConnection;

use super::ValidateUserPasswordService;

pub struct UpdateUserPasswordService;

impl UpdateUserPasswordService {
  pub async fn run(
    conn: &mut SqliteConnection,
    user_id: i64,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    ValidateUserPasswordService::run(new_password)?;

    if new_password != new_password_confirmation {
      return Err(UserError::ValidationError(
        "Passwords do not match".to_string(),
      ));
    }

    let user = GetUserByIdQuery::run(conn, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    let is_current_password_valid =
      VerifyPasswordService::run(current_password, &user.password_hash)
        .map_err(UserError::PasswordHashingFailed)?;

    if !is_current_password_valid {
      return Err(UserError::ValidationError(
        "Current password is incorrect".to_string(),
      ));
    }

    let new_password_hash = GeneratePasswordHashService::run(new_password)?;

    let update_data = UpdateUserPasswordData {
      password_hash: new_password_hash,
    };

    let updated_user = UpdateUserPasswordQuery::run(conn, user_id, update_data)
      .await
      .map_err(UserError::DatabaseError)?;

    Ok(updated_user)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::create_test_role_model_with_pool;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_password_success() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "oldpassword123")
      .await
      .unwrap();
    let updated_user = UpdateUserPasswordService::run(
      &mut conn,
      user.id,
      "oldpassword123",
      "newpassword456",
      "newpassword456",
    )
    .await
    .unwrap();

    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_ne!(updated_user.password_hash, user.password_hash);

    let is_new_password_valid =
      VerifyPasswordService::run("newpassword456", &updated_user.password_hash).unwrap();
    assert!(is_new_password_valid);

    let is_old_password_valid =
      VerifyPasswordService::run("oldpassword123", &updated_user.password_hash).unwrap();
    assert!(!is_old_password_valid);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_password_invalid_current_password() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "correctpassword")
      .await
      .unwrap();
    let result = UpdateUserPasswordService::run(
      &mut conn,
      user.id,
      "wrongpassword",
      "newpassword456",
      "newpassword456",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Current password is incorrect"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_password_password_confirmation_mismatch() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "currentpassword")
      .await
      .unwrap();
    let result = UpdateUserPasswordService::run(
      &mut conn,
      user.id,
      "currentpassword",
      "newpassword456",
      "differentpassword",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Passwords do not match"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_password_invalid_new_password() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "currentpassword")
      .await
      .unwrap();
    let result =
      UpdateUserPasswordService::run(&mut conn, user.id, "currentpassword", "123", "123").await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("at least 6 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_password_nonexistent_user() {
    let mut conn = pool.acquire().await.unwrap();
    let result = UpdateUserPasswordService::run(
      &mut conn,
      999,
      "anypassword",
      "newpassword456",
      "newpassword456",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }
}
