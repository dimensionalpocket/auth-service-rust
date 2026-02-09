use crate::services::{GetUserByNameService, VerifyPasswordService};
use crate::types::SessionError;
use sqlx::SqliteConnection;

use super::CreateSessionForUserService;

pub struct CreateSessionService;

impl CreateSessionService {
  pub async fn run(
    main_conn: &mut SqliteConnection,
    username: &str,
    password: &str,
    secret: &[u8],
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
    let token = CreateSessionForUserService::run(&user, secret)?;

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
  use crate::test_utils::create_test_role_model_with_conn;
  use dps_auth_session::DpsAuthSession;

  // Test secret - 32 bytes for AES-256
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  #[dps_auth_db_test]
  async fn test_create_session_success() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    create_test_role_model_with_conn(&mut main_conn, "user", &["can_view_user_self"], true).await;
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();
    let token = CreateSessionService::run(&mut main_conn, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[dps_auth_db_test]
  async fn test_create_session_blank_username() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let result = CreateSessionService::run(&mut main_conn, "", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_whitespace_username() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let result = CreateSessionService::run(&mut main_conn, "   ", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_blank_password() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let result = CreateSessionService::run(&mut main_conn, "testuser", "", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password is blank"));
  }

  #[dps_auth_db_test]
  async fn test_create_session_user_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let result =
      CreateSessionService::run(&mut main_conn, "nonexistent", "password123", TEST_SECRET).await;

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
    let result =
      CreateSessionService::run(&mut main_conn, "testuser", "wrong_password", TEST_SECRET).await;

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
    let token = CreateSessionService::run(&mut main_conn, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }
}
