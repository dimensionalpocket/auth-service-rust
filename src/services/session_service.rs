use crate::services::{PasswordService, UserService};
use aes_gcm::{AeadCore, AeadInPlace, Aes256Gcm, Key, KeyInit, Nonce};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fmt;

/// Session payload structure containing user session information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionPayload {
  /// Subject - user ID
  pub sub: i64,
  /// Issued at timestamp (seconds since epoch)
  pub iat: i64,
  /// Expiration timestamp (seconds since epoch)
  pub exp: i64,
}

/// Custom error type for session operations
#[derive(Debug)]
pub enum SessionError {
  /// Secret key not configured
  SecretKeyNotSet,
  /// Invalid secret key format
  InvalidSecretKey(String),
  /// Token encoding failed
  EncodingError(String),
  /// Token decoding failed
  DecodingError(String),
  /// Token has expired
  TokenExpired,
  /// Invalid token format
  InvalidToken(String),
  /// JSON serialization/deserialization error
  JsonError(String),
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
      SessionError::SecretKeyNotSet => write!(f, "Secret key not configured"),
      SessionError::InvalidSecretKey(msg) => write!(f, "Invalid secret key: {msg}"),
      SessionError::EncodingError(msg) => write!(f, "Token encoding error: {msg}"),
      SessionError::DecodingError(msg) => write!(f, "Token decoding error: {msg}"),
      SessionError::TokenExpired => write!(f, "Token has expired"),
      SessionError::InvalidToken(msg) => write!(f, "Invalid token: {msg}"),
      SessionError::JsonError(msg) => write!(f, "JSON error: {msg}"),
      SessionError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SessionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      SessionError::PasswordVerificationError(msg) => {
        write!(f, "Password verification error: {msg}")
      }
    }
  }
}

impl std::error::Error for SessionError {}

/// Service for managing session tokens using custom encrypted format
pub struct SessionService;

impl SessionService {
  /// Default token expiration time (3 days in seconds)
  const DEFAULT_EXPIRATION_SECONDS: i64 = 3 * 24 * 60 * 60;

  /// Encode a session payload into an encrypted token.
  ///
  /// This method takes a session payload containing user information and creates
  /// an encrypted token that can be safely transmitted and stored. The token uses
  /// AES-256-GCM encryption with a random nonce for security.
  ///
  /// # Arguments
  ///
  /// * `payload` - The session payload containing user ID and timestamps
  ///
  /// # Returns
  ///
  /// Returns a base64-encoded encrypted token string on success, or a `SessionError` on failure.
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - The secret key is not configured (`SecretKeyNotSet`)
  /// - The secret key format is invalid (`InvalidSecretKey`)
  /// - JSON serialization fails (`JsonError`)
  /// - Encryption fails (`EncodingError`)
  ///
  /// # Examples
  ///
  /// ```rust
  /// use dp_auth_service::services::{SessionService, SessionPayload};
  ///
  /// // First set the environment variable with a valid 32-byte base64 key
  /// std::env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  ///
  /// let payload = SessionService::create_payload(123);
  /// let token = SessionService::encode_token(&payload)?;
  /// println!("Generated token: {}", token);
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn encode_token(payload: &SessionPayload) -> Result<String, SessionError> {
    // Get the secret key from environment
    let key_bytes = Self::get_secret_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Serialize payload to JSON
    let json_data =
      serde_json::to_vec(payload).map_err(|e| SessionError::JsonError(e.to_string()))?;

    // Generate a random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);

    // Encrypt the JSON data
    let mut buffer = json_data;
    cipher
      .encrypt_in_place(&nonce, b"", &mut buffer)
      .map_err(|e| SessionError::EncodingError(e.to_string()))?;

    // Combine nonce + encrypted data
    let mut result = nonce.to_vec();
    result.extend_from_slice(&buffer);

    // Base64 encode the result
    Ok(general_purpose::STANDARD.encode(result))
  }

  /// Decode an encrypted token and retrieve the session payload.
  ///
  /// This method takes an encrypted token string and decrypts it to retrieve
  /// the original session payload. It also validates that the token has not expired.
  ///
  /// # Arguments
  ///
  /// * `token` - The base64-encoded encrypted token string
  ///
  /// # Returns
  ///
  /// Returns the decrypted session payload on success, or a `SessionError` on failure.
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - The secret key is not configured (`SecretKeyNotSet`)
  /// - The secret key format is invalid (`InvalidSecretKey`)
  /// - The token format is invalid (`InvalidToken`)
  /// - Decryption fails (`DecodingError`)
  /// - JSON deserialization fails (`JsonError`)
  /// - The token has expired (`TokenExpired`)
  ///
  /// # Examples
  ///
  /// ```rust
  /// use dp_auth_service::services::{SessionService, SessionPayload};
  ///
  /// // First set the environment variable with a valid 32-byte base64 key
  /// std::env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  ///
  /// // Create and encode a token first
  /// let payload = SessionService::create_payload(123);
  /// let token = SessionService::encode_token(&payload)?;
  ///
  /// // Then decode it
  /// match SessionService::decode_token(&token) {
  ///     Ok(decoded_payload) => println!("User ID: {}", decoded_payload.sub),
  ///     Err(e) => println!("Token validation failed: {}", e),
  /// }
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn decode_token(token: &str) -> Result<SessionPayload, SessionError> {
    // Get the secret key from environment
    let key_bytes = Self::get_secret_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Base64 decode the token
    let encrypted_data = general_purpose::STANDARD
      .decode(token)
      .map_err(|e| SessionError::InvalidToken(format!("Base64 decode error: {e}")))?;

    // Check minimum length (nonce + some encrypted data)
    if encrypted_data.len() < 12 {
      return Err(SessionError::InvalidToken("Token too short".to_string()));
    }

    // Split nonce and encrypted payload
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt the data
    let mut buffer = ciphertext.to_vec();
    cipher
      .decrypt_in_place(nonce, b"", &mut buffer)
      .map_err(|e| SessionError::DecodingError(format!("Decryption failed: {e}")))?;

    // Deserialize JSON to payload
    let payload: SessionPayload =
      serde_json::from_slice(&buffer).map_err(|e| SessionError::JsonError(e.to_string()))?;

    // Check if token has expired
    let current_time = chrono::Utc::now().timestamp();
    if payload.exp < current_time {
      return Err(SessionError::TokenExpired);
    }

    Ok(payload)
  }

  /// Create a new session payload for a user.
  ///
  /// This method creates a new session payload with the current timestamp as the
  /// issued time and sets the expiration to 3 days from now.
  ///
  /// # Arguments
  ///
  /// * `user_id` - The unique identifier of the user for this session
  ///
  /// # Returns
  ///
  /// Returns a new `SessionPayload` with the user ID and appropriate timestamps.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use dp_auth_service::services::SessionService;
  ///
  /// let payload = SessionService::create_payload(123);
  /// assert_eq!(payload.sub, 123);
  /// assert!(payload.exp > payload.iat);
  /// ```
  pub fn create_payload(user_id: i64) -> SessionPayload {
    let current_time = chrono::Utc::now().timestamp();

    SessionPayload {
      sub: user_id,
      iat: current_time,
      exp: current_time + Self::DEFAULT_EXPIRATION_SECONDS,
    }
  }

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
  /// use dp_auth_service::services::SessionService;
  /// use sqlx::SqlitePool;
  ///
  /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
  /// let token = SessionService::create_session(pool, "john_doe", "secure_password").await?;
  /// println!("Session token: {}", token);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn create_session(
    pool: &SqlitePool,
    username: &str,
    password: &str,
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

    // Create session payload and encode token
    let payload = Self::create_payload(user.id);
    let token = Self::encode_token(&payload)?;

    tracing::info!(
      username = username,
      user_id = user.id,
      "Authentication successful"
    );

    Ok(token)
  }

  /// Get the secret key from environment variable
  fn get_secret_key() -> Result<Vec<u8>, SessionError> {
    let key_str = std::env::var("DP_AUTH_SECRET_KEY").map_err(|_| SessionError::SecretKeyNotSet)?;

    // Decode base64 key
    let key_bytes = general_purpose::STANDARD
      .decode(&key_str)
      .map_err(|e| SessionError::InvalidSecretKey(format!("Base64 decode error: {e}")))?;

    // Ensure key is at least 32 bytes for AES-256
    if key_bytes.len() < 32 {
      return Err(SessionError::InvalidSecretKey(format!(
        "Key too short: {} bytes, need at least 32",
        key_bytes.len()
      )));
    }

    // Use exactly 32 bytes for AES-256
    Ok(key_bytes[..32].to_vec())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::services::UserService;
  use std::env;
  use std::sync::Mutex;

  // Mutex to serialize tests that modify environment variables
  static ENV_MUTEX: Mutex<()> = Mutex::new(());

  fn setup_test_key() {
    // Use a proper 32-byte key for testing (generated with openssl rand -base64 32)
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
    );
  }

  #[test]
  fn test_create_payload() {
    let user_id = 123;
    let payload = SessionService::create_payload(user_id);

    assert_eq!(payload.sub, user_id);
    assert!(payload.iat > 0);
    assert_eq!(
      payload.exp,
      payload.iat + SessionService::DEFAULT_EXPIRATION_SECONDS
    );
  }

  #[test]
  fn test_encode_decode_roundtrip() {
    setup_test_key();

    let payload = SessionService::create_payload(456);
    let token = SessionService::encode_token(&payload).unwrap();
    let decoded_payload = SessionService::decode_token(&token).unwrap();

    assert_eq!(payload, decoded_payload);
  }

  #[test]
  fn test_decode_expired_token() {
    setup_test_key();

    // Create payload with past expiration
    let expired_payload = SessionPayload {
      sub: 789,
      iat: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
      exp: chrono::Utc::now().timestamp() - 1800, // 30 minutes ago (expired)
    };

    let token = SessionService::encode_token(&expired_payload).unwrap();
    let result = SessionService::decode_token(&token);

    assert!(matches!(result, Err(SessionError::TokenExpired)));
  }

  #[test]
  fn test_decode_invalid_token() {
    setup_test_key();

    // Test empty token
    let result = SessionService::decode_token("");
    assert!(matches!(result, Err(SessionError::InvalidToken(_))));

    // Test invalid base64
    let result = SessionService::decode_token("invalid-base64!");
    assert!(matches!(result, Err(SessionError::InvalidToken(_))));

    // Test too short token
    let result = SessionService::decode_token("dGVzdA=="); // "test" in base64 (too short)
    assert!(matches!(result, Err(SessionError::InvalidToken(_))));
  }

  #[test]
  fn test_missing_secret_key() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_key = env::var("DP_AUTH_SECRET_KEY").ok();
    env::remove_var("DP_AUTH_SECRET_KEY");

    let payload = SessionService::create_payload(123);
    let result = SessionService::encode_token(&payload);

    // Restore original key if it existed
    if let Some(key) = original_key {
      env::set_var("DP_AUTH_SECRET_KEY", key);
    } else {
      env::remove_var("DP_AUTH_SECRET_KEY");
    }

    assert!(matches!(result, Err(SessionError::SecretKeyNotSet)));
  }

  #[test]
  fn test_invalid_secret_key() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_key = env::var("DP_AUTH_SECRET_KEY").ok();

    // Set a key that's too short
    env::set_var("DP_AUTH_SECRET_KEY", "c2hvcnQ="); // "short" in base64 (too short)

    let payload = SessionService::create_payload(123);
    let result = SessionService::encode_token(&payload);

    // Restore original key if it existed
    if let Some(key) = original_key {
      env::set_var("DP_AUTH_SECRET_KEY", key);
    } else {
      env::remove_var("DP_AUTH_SECRET_KEY");
    }

    assert!(matches!(result, Err(SessionError::InvalidSecretKey(_))));
  }

  #[test]
  fn test_encode_decode_with_different_keys() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_key = env::var("DP_AUTH_SECRET_KEY").ok();

    // Encode with one key (32 bytes)
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
    );
    let payload = SessionService::create_payload(123);
    let token = SessionService::encode_token(&payload).unwrap();

    // Try to decode with different key (also 32 bytes)
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "465c4b/KryoNecAbP6TsNqO5L8CYb28bID+MgiK5Y+o=",
    );
    let result = SessionService::decode_token(&token);

    // Restore original key if it existed
    if let Some(key) = original_key {
      env::set_var("DP_AUTH_SECRET_KEY", key);
    } else {
      env::remove_var("DP_AUTH_SECRET_KEY");
    }

    assert!(matches!(result, Err(SessionError::DecodingError(_))));
  }

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn test_create_session_success() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test: Create session
    let token = SessionService::create_session(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Verify: Token can be decoded and contains correct user ID
    let payload = SessionService::decode_token(&token).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_blank_username() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "", "password123").await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_whitespace_username() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "   ", "password123").await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User is blank"));
  }

  #[tokio::test]
  async fn test_create_session_blank_password() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "testuser", "").await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password is blank"));
  }

  #[tokio::test]
  async fn test_create_session_user_not_found() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    let result = SessionService::create_session(&pool, "nonexistent", "password123").await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("User not found"));
  }

  #[tokio::test]
  async fn test_create_session_wrong_password() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "correct_password")
      .await
      .unwrap();

    // Test: Try with wrong password
    let result = SessionService::create_session(&pool, "testuser", "wrong_password").await;

    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Password does not match"));
  }

  #[tokio::test]
  async fn test_create_session_case_insensitive_username() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with mixed case
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "TestUser", "password123")
      .await
      .unwrap();

    // Test: Login with different case
    let token = SessionService::create_session(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Verify: Token contains correct user ID
    let payload = SessionService::decode_token(&token).unwrap();
    assert_eq!(payload.sub, user.id);
  }
}
