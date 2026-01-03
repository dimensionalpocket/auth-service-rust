use crate::middleware::session::SessionContext;
use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{AuthService, UserService};
use crate::types::{SessionError, UserError};
use sqlx::SqlitePool;

pub struct AuthOrchestrator;

impl AuthOrchestrator {
  pub async fn get_authenticated_user(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Option<crate::services::AuthMeResult>, SessionError> {
    match &session_context.payload {
      Some(_payload) => {
        let mut conn = pool
          .acquire()
          .await
          .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        AuthService::get_current_user(&mut conn, &session_context)
          .await
          .map(Some)
      }
      None => Ok(None),
    }
  }

  pub async fn change_authenticated_user_password(
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

    UserService::update_password(
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
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_full_with_pool,
    create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_get_authenticated_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    let _user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user =
      create_test_user_full_with_pool(&pool, "testuser", Some(1), "password123", None).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthOrchestrator::get_authenticated_user(&pool, session_context).await;

    assert!(result.is_ok());
    let auth_me_result = result.unwrap();
    assert!(auth_me_result.is_some());
    let result_data = auth_me_result.unwrap();
    assert_eq!(result_data.user_id, user.id);
    assert_eq!(result_data.username, "testuser");
    assert_eq!(result_data.session_iat, 1000);
    assert_eq!(result_data.session_exp, 2000);
  }

  #[tokio::test]
  async fn test_get_authenticated_user_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    let session_context = SessionContext::new(None);

    let result = AuthOrchestrator::get_authenticated_user(&pool, session_context).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }

  #[tokio::test]
  async fn test_change_authenticated_user_password_success() {
    let (pool, _temp_file) = create_test_database().await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

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

    let session_context = SessionContext::new(None);

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

    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

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

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

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

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

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

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user = create_test_user_with_pool(&pool, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

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
