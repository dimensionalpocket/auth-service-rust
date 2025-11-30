use crate::middleware::session::SessionContext;
use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{UserError, UserService};
use sqlx::SqlitePool;

pub struct AuthOrchestrator;

impl AuthOrchestrator {
  pub async fn change_authenticated_user_password(
    pool: &SqlitePool,
    session_context: SessionContext,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    // Authentication: Check if user is authenticated
    let session_payload = session_context
      .payload
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let user_id = session_payload.sub;

    // Authorization: Verify user exists
    let _user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(UserError::UserNotFound(user_id))?;

    // Business logic: Update password
    UserService::update_password(
      pool,
      user_id,
      current_password,
      new_password,
      new_password_confirmation,
    )
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::PasswordService;
  use dps_auth_session::DpsAuthSessionPayload;
  use sqlx::SqlitePool;
  use uuid::Uuid;

  async fn create_test_user(pool: &SqlitePool, username: &str, role_id: i64) -> User {
    let password_hash = PasswordService::generate("password123").unwrap();
    let create_data = CreateUserData {
      uuid: Uuid::new_v4().to_string(),
      name: username.to_string(),
      role_id: Some(role_id),
      password_hash,
      metadata_json: None,
    };
    CreateUserQuery::run(pool, create_data).await.unwrap()
  }

  async fn create_test_role(pool: &SqlitePool, name: &str, permissions: &[&str]) -> i64 {
    let permissions_json = serde_json::json!(permissions);
    let result = sqlx::query(
      r#"
      INSERT INTO user_roles (name, created_ts, permissions_json, is_default)
      VALUES (?, ?, ?, FALSE)
      "#,
    )
    .bind(name)
    .bind(1234567890i64)
    .bind(permissions_json)
    .execute(pool)
    .await
    .unwrap();

    result.last_insert_rowid()
  }

  #[tokio::test]
  async fn test_change_authenticated_user_password_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let user = create_test_user(&pool, "testuser", user_role_id).await;

    // Create session context for user
    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test password change
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
      "password123",
      "newpassword456",
      "newpassword456",
    )
    .await;

    assert!(result.is_ok());
    let updated_user = result.unwrap();
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_ne!(updated_user.password_hash, user.password_hash);
  }

  #[tokio::test]
  async fn test_change_authenticated_user_password_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    // Test password change
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
      "password123",
      "newpassword456",
      "newpassword456",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_change_authenticated_user_password_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test password change
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
      "password123",
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

  #[tokio::test]
  async fn test_change_authenticated_user_password_wrong_current_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let user = create_test_user(&pool, "testuser", user_role_id).await;

    // Create session context for user
    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test password change with wrong current password
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
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

  #[tokio::test]
  async fn test_change_authenticated_user_password_password_mismatch() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let user = create_test_user(&pool, "testuser", user_role_id).await;

    // Create session context for user
    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test password change with mismatched confirmation
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
      "password123",
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

  #[tokio::test]
  async fn test_change_authenticated_user_password_invalid_new_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let user = create_test_user(&pool, "testuser", user_role_id).await;

    // Create session context for user
    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test password change with invalid new password (too short)
    let result = AuthOrchestrator::change_authenticated_user_password(
      &pool,
      session_context,
      "password123",
      "123",
      "123",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("at least 6 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }
}
