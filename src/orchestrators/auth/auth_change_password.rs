use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use crate::services::UpdateUserPasswordService;
use crate::types::{DpsAuthApiConfig, UserError};
use crate::utils::session_context_sub_to_user_id;

pub struct AuthChangePasswordOrchestrator;

impl AuthChangePasswordOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsAuthApiConfig,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    let user_id = session_context_sub_to_user_id(&session_context, config)
      .map_err(UserError::AuthenticationError)?;

    let main_pool = databases.main();
    let mut main_conn = main_pool.acquire().await?;

    let _user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await?
      .ok_or(UserError::UserNotFound(user_id))?;

    UpdateUserPasswordService::run(
      &mut main_conn,
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
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_success() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let user = create_test_user_with_databases(&databases, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
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

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_unauthenticated() {
    let session_context = SessionContext::new(None);

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      "password123",
      "newpassword456",
      "newpassword456",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_nonexistent_user() {
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
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

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_wrong_current_password() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let user = create_test_user_with_databases(&databases, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
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

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_password_mismatch() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let user = create_test_user_with_databases(&databases, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
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

  #[dps_auth_db_test]
  async fn test_change_authenticated_user_password_invalid_new_password() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let user = create_test_user_with_databases(&databases, "testuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthChangePasswordOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
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
