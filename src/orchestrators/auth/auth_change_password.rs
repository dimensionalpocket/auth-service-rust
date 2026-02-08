use crate::middleware::session::SessionContext;
use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use crate::services::UpdateUserPasswordService;
use crate::types::UserError;
use sqlx::SqlitePool;

pub struct AuthChangePasswordOrchestrator;

impl AuthChangePasswordOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    let session_payload = session_context
      .payload
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let user_id = session_payload.sub;

    let mut conn = pool.acquire().await?;

    let _user = GetUserByIdQuery::run(&mut conn, user_id)
      .await?
      .ok_or(UserError::UserNotFound(user_id))?;

    UpdateUserPasswordService::run(
      &mut conn,
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
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_success() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_unauthenticated() {
    let session_context = SessionContext::new(None);

    let result = AuthChangePasswordOrchestrator::run(
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_nonexistent_user() {
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_wrong_current_password() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_password_mismatch() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_change_authenticated_user_password_invalid_new_password() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      AuthChangePasswordOrchestrator::run(&pool, session_context, "password123", "123", "123")
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
