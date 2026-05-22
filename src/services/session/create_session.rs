use crate::services::{GetUserByNameService, VerifyPasswordService};
use crate::types::SessionError;
use dps_config::DpsConfig;
use sqlx::SqliteConnection;

use super::CreateSessionForUserService;

pub struct CreateSessionService;

impl CreateSessionService {
  pub async fn run(
    main_conn: &mut SqliteConnection,
    username: &str,
    password: &str,
    config: &DpsConfig,
  ) -> Result<String, SessionError> {
    // Input validation
    if username.trim().is_empty() {
      let error_msg = "User is blank";
      tracing::info!(username = "", error = error_msg, "Authentication failed");
      return Err(SessionError::AuthenticationError(error_msg.to_string()));
    }

    if password.is_empty() {
      let error_msg = "Password is blank";
      tracing::info!(
        username = username,
        error = error_msg,
        "Authentication failed"
      );
      return Err(SessionError::AuthenticationError(error_msg.to_string()));
    }

    // Retrieve user by username
    let user = match GetUserByNameService::run(main_conn, username).await {
      Ok(Some(user)) => user,
      Ok(None) => {
        let error_msg = "User not found";
        tracing::info!(
          username = username,
          error = error_msg,
          "Authentication failed"
        );
        return Err(SessionError::AuthenticationError(error_msg.to_string()));
      }
      Err(db_error) => {
        let error_msg = format!("Database error during user lookup: {db_error}");
        tracing::info!(username = username, error = %db_error, "Authentication failed");
        return Err(SessionError::DatabaseError(error_msg));
      }
    };

    // Verify password
    let password_matches = match VerifyPasswordService::run(password, &user.password_hash) {
      Ok(matches) => matches,
      Err(password_error) => {
        let error_msg = format!("Password verification error: {password_error}");
        tracing::info!(username = username, error = %password_error, "Authentication failed");
        return Err(SessionError::PasswordVerificationError(error_msg));
      }
    };

    if !password_matches {
      let error_msg = "Password does not match";
      tracing::info!(
        username = username,
        error = error_msg,
        "Authentication failed"
      );
      return Err(SessionError::AuthenticationError(error_msg.to_string()));
    }

    // Create session token for the authenticated user
    let token = CreateSessionForUserService::run(&user, config)?;

    tracing::info!(
      username = username,
      user_id = user.id,
      "Authentication successful"
    );

    Ok(token)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{create_test_dps_config, create_test_role_model_with_conn};
  use dps_auth_session::DpsAuthSession;

  #[dps_auth_db_test]
  async fn test_create_session_success() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    create_test_role_model_with_conn(&mut main_conn, "user", &["can_view_user_self"], true).await;
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();
    let config = create_test_dps_config();
    let token = CreateSessionService::run(&mut main_conn, "testuser", "password123", &config)
      .await
      .unwrap();

    let secret = config.get_auth_api_session_secret_bytes().unwrap();
    let payload = DpsAuthSession::decode_token(&token, &secret).unwrap();
    assert_eq!(payload.sub, user.id.to_string());
  }

  #[dps_auth_db_test]
  async fn test_create_session_blank_username() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let config = create_test_dps_config();
    let result = CreateSessionService::run(&mut main_conn, "", "password123", &config).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_whitespace_username() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let config = create_test_dps_config();
    let result = CreateSessionService::run(&mut main_conn, "   ", "password123", &config).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_blank_password() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let config = create_test_dps_config();
    let result = CreateSessionService::run(&mut main_conn, "testuser", "", &config).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_user_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let config = create_test_dps_config();
    let result =
      CreateSessionService::run(&mut main_conn, "nonexistent", "password123", &config).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User not found"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_wrong_password() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    create_test_role_model_with_conn(&mut main_conn, "user", &["can_view_user_self"], true).await;
    CreateUserService::run(&mut main_conn, "testuser", "correct_password")
      .await
      .unwrap();
    let config = create_test_dps_config();
    let result =
      CreateSessionService::run(&mut main_conn, "testuser", "wrong_password", &config).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password does not match"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_case_insensitive_username() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    create_test_role_model_with_conn(&mut main_conn, "user", &["can_view_user_self"], true).await;
    let user = CreateUserService::run(&mut main_conn, "TestUser", "password123")
      .await
      .unwrap();
    let config = create_test_dps_config();
    let token = CreateSessionService::run(&mut main_conn, "testuser", "password123", &config)
      .await
      .unwrap();

    let secret = config.get_auth_api_session_secret_bytes().unwrap();
    let payload = DpsAuthSession::decode_token(&token, &secret).unwrap();
    assert_eq!(payload.sub, user.id.to_string());
  }
}
