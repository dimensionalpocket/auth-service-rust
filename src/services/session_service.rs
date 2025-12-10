use crate::models::user::User;
use crate::services::{PasswordService, UserService};
use dps_auth_session::{DpsAuthSession, DpsAuthSessionError};
use sqlx::SqlitePool;
use std::fmt;

// Re-export the session payload from the new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;

/// Custom error type for session operations
#[derive(Debug)]
pub enum SessionError {
  /// Token encoding/decoding errors from the auth session service
  AuthSessionError(DpsAuthSessionError),
  /// Authentication failed - user input validation
  AuthenticationError(String),
  /// Database operation failed during authentication
  DatabaseError(String),
  /// Password verification failed
  PasswordVerificationError(String),
}

impl fmt::Display for SessionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SessionError::AuthSessionError(err) => write!(f, "Auth session error: {err}"),
      SessionError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SessionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      SessionError::PasswordVerificationError(msg) => {
        write!(f, "Password verification error: {msg}")
      }
    }
  }
}

impl From<DpsAuthSessionError> for SessionError {
  fn from(err: DpsAuthSessionError) -> Self {
    SessionError::AuthSessionError(err)
  }
}

impl std::error::Error for SessionError {}

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
  /// * `pool` - Database connection pool for user lookup
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
  ///
  /// # Examples
  ///
  /// ```rust
  /// use dps_auth_api::services::SessionService;
  /// use sqlx::SqlitePool;
  ///
  /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
  /// let secret = &[0u8; 32]; // In practice, use a proper secret
  /// let token = SessionService::create_session(pool, "john_doe", "secure_password", secret).await?;
  /// println!("Session token: {}", token);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn create_session(
    pool: &SqlitePool,
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
    let user = match UserService::get_user_by_name(pool, username).await {
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
  use crate::database::test_utils::create_test_database;
  use crate::services::UserService;

  // Test secret - 32 bytes for AES-256 (base64-decoded from QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=)
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn test_create_session_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test: Create session
    let token = SessionService::create_session(&pool, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    // Verify: Token can be decoded and contains correct user ID
    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_blank_username() {
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_whitespace_username() {
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "   ", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_blank_password() {
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "testuser", "", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password is blank"));
  }

  #[tokio::test]
  async fn test_create_session_user_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let result =
      SessionService::create_session(&pool, "nonexistent", "password123", TEST_SECRET).await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User not found"));
  }

  #[tokio::test]
  async fn test_create_session_wrong_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "correct_password")
      .await
      .unwrap();

    // Test: Try with wrong password
    let result =
      SessionService::create_session(&pool, "testuser", "wrong_password", TEST_SECRET).await;

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
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "TestUser", "password123")
      .await
      .unwrap();

    // Test: Login with different case
    let token = SessionService::create_session(&pool, "testuser", "password123", TEST_SECRET)
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
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test: Create session for existing user (no password needed)
    let token = SessionService::create_session_for_user(&user, TEST_SECRET).unwrap();

    // Verify: Token is not empty and contains correct user ID
    assert!(!token.is_empty());

    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_for_user_different_users() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create two users
    setup_default_role(&pool).await;
    let user1 = UserService::create_user(&pool, "user1", "password123")
      .await
      .unwrap();
    let user2 = UserService::create_user(&pool, "user2", "password123")
      .await
      .unwrap();

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
