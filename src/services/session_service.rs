use crate::models::user::User;
use crate::services::{PasswordService, UserService};
use crate::types::SessionError;
use dps_auth_session::DpsAuthSession;
use sqlx::SqliteConnection;

// Re-export the session payload from the new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;

/// Service for managing session tokens using custom encrypted format
pub struct SessionService;

impl SessionService {
  /// Create a new session by authenticating user credentials
  ///
  /// This method validates the provided username and password, retrieves the user
  /// from the database, verifies the password against the stored hash, and creates
  /// a new session token if authentication is successful.
  ///
  /// # Arguments
  ///
  /// * `conn` - Database connection for user lookup
  /// * `username` - The username to authenticate
  /// * `password` - The plaintext password to verify
  /// * `secret` - The 32-byte secret key for token encryption
  ///
  /// # Returns
  ///
  /// Returns a session token string on successful authentication, or a `SessionError` on failure.
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - Username is blank (`AuthenticationError`)
  /// - Password is blank (`AuthenticationError`)
  /// - User not found (`AuthenticationError`)
  /// - Password does not match (`AuthenticationError`)
  /// - Database operation fails (`DatabaseError`)
  /// - Token encoding fails (`EncodingError`)
  ///
  /// All authentication errors are logged at info level with the username for debugging.
  pub async fn create_session(
    conn: &mut SqliteConnection,
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
    let user = match UserService::get_user_by_name(conn, username).await {
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
    let password_matches = match PasswordService::verify(password, &user.password_hash) {
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
    let token = Self::create_session_for_user(&user, secret)?;

    tracing::info!(
      username = username,
      user_id = user.id,
      "Authentication successful"
    );

    Ok(token)
  }

  /// Create a session token for an existing user (no password verification)
  ///
  /// This method creates a session token for a user that has already been authenticated
  /// or created. It does not perform any password verification.
  ///
  /// # Arguments
  ///
  /// * `user` - The user object for which to create a session
  /// * `secret` - The 32-byte secret key for token encryption
  ///
  /// # Returns
  ///
  /// Returns a session token string on success, or a `SessionError` on failure.
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - Token encoding fails (`AuthSessionError`)
  pub fn create_session_for_user(user: &User, secret: &[u8]) -> Result<String, SessionError> {
    // Create session payload and encode token
    let payload = DpsAuthSession::create_payload(user.id, None);
    let token = DpsAuthSession::encode_token(&payload, secret)?;

    Ok(token)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::UserService;
  use crate::test_utils::{
    create_test_database, create_test_role_model, create_test_role_model_with_pool,
  };

  // Test secret - 32 bytes for AES-256 (base64-decoded from QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=)
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  #[tokio::test]
  async fn test_create_session_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    let user = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      UserService::create_user(&mut conn, "testuser", "password123")
        .await
        .unwrap()
    };

    // Test: Create session
    let mut conn = pool.acquire().await.unwrap();
    let token = SessionService::create_session(&mut conn, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    // Verify: Token can be decoded and contains correct user ID
    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_blank_username() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let result = SessionService::create_session(&mut conn, "", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_whitespace_username() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let result = SessionService::create_session(&mut conn, "   ", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_blank_password() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let result = SessionService::create_session(&mut conn, "testuser", "", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password is blank"));
  }

  #[tokio::test]
  async fn test_create_session_user_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let result =
      SessionService::create_session(&mut conn, "nonexistent", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User not found"));
  }

  #[tokio::test]
  async fn test_create_session_wrong_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      UserService::create_user(&mut conn, "testuser", "correct_password")
        .await
        .unwrap();
    };

    // Test: Try with wrong password
    let mut conn = pool.acquire().await.unwrap();
    let result =
      SessionService::create_session(&mut conn, "testuser", "wrong_password", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password does not match"));
  }

  #[tokio::test]
  async fn test_create_session_case_insensitive_username() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with mixed case
    let user = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      UserService::create_user(&mut conn, "TestUser", "password123")
        .await
        .unwrap()
    };

    // Test: Login with different case
    let mut conn = pool.acquire().await.unwrap();
    let token = SessionService::create_session(&mut conn, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    // Verify: Token contains correct user ID
    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_for_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    let user = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      UserService::create_user(&mut conn, "testuser", "password123")
        .await
        .unwrap()
    };

    // Test: Create session
    let mut conn = pool.acquire().await.unwrap();
    let token = SessionService::create_session(&mut conn, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    // Verify: Token is not empty and contains correct user ID
    assert!(!token.is_empty());

    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_for_user_different_users() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create two users
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let user1 = {
      let mut conn = pool.acquire().await.unwrap();
      UserService::create_user(&mut conn, "user1", "password123")
        .await
        .unwrap()
    };
    let user2 = {
      let mut conn = pool.acquire().await.unwrap();
      UserService::create_user(&mut conn, "user2", "password123")
        .await
        .unwrap()
    };

    // Test: Create sessions for both users
    let token1 = SessionService::create_session_for_user(&user1, TEST_SECRET).unwrap();
    let token2 = SessionService::create_session_for_user(&user2, TEST_SECRET).unwrap();

    // Verify: Tokens are different and contain correct user IDs
    assert_ne!(token1, token2);

    let payload1 = DpsAuthSession::decode_token(&token1, TEST_SECRET).unwrap();
    let payload2 = DpsAuthSession::decode_token(&token2, TEST_SECRET).unwrap();

    assert_eq!(payload1.sub, user1.id);
    assert_eq!(payload2.sub, user2.id);
  }
}
