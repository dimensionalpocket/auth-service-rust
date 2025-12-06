use crate::middleware::session::SessionContext;
use crate::queries::users::get_user_by_id_with_role::GetUserByIdWithRoleQuery;
use crate::services::{SessionError, SessionService, UserError, UserService};
use sqlx::SqlitePool;
use tracing::instrument;

/// Result type for authentication operations containing both user and session information
#[derive(Debug, Clone)]
pub struct AuthResult {
  pub user_id: i64,
  pub username: String,
  pub session_token: String,
}

/// Result type for user registration operations containing user information
#[derive(Debug, Clone)]
pub struct RegisterResult {
  pub user_id: i64,
  pub username: String,
  pub uuid: String,
  pub role_id: i64,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub session_token: String,
}

/// Result type for getting current authenticated user information
#[derive(Debug, Clone)]
pub struct AuthMeResult {
  pub user_id: i64,
  pub username: String,
  pub uuid: String,
  pub role_id: i64,
  pub role_name: String,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub session_iat: i64,
  pub session_exp: i64,
}

/// Orchestration service for authentication operations
pub struct AuthService;

impl AuthService {
  /// Authenticate a user and create a session
  ///
  /// This method orchestrates the login process by:
  /// 1. Finding the user by username
  /// 2. Verifying credentials and creating a session
  /// 3. Returning combined authentication result
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `username` - User's username
  /// * `password` - User's password
  /// * `session_secret` - Secret for session token signing
  ///
  /// # Returns
  /// * `AuthResult` containing user info and session token
  ///
  /// # Errors
  /// * `UserError` - If user not found or other user-related errors
  /// * `SessionError` - If authentication fails or session creation fails
  #[instrument(skip(pool, session_secret, password), fields(username = %username))]
  pub async fn login(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    session_secret: &[u8],
  ) -> Result<AuthResult, SessionError> {
    // Find user by username first to provide better error handling
    let user = UserService::get_user_by_name(pool, username)
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    // Check if user exists
    let user =
      user.ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

    // Create session which includes password verification
    let session_token =
      SessionService::create_session(pool, username, password, session_secret).await?;

    Ok(AuthResult {
      user_id: user.id,
      username: user.name,
      session_token,
    })
  }

  /// Register a new user account
  ///
  /// This method orchestrates the user registration process by:
  /// 1. Validating password confirmation matches
  /// 2. Creating a new user with validation
  /// 3. Returning user registration result
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `username` - Desired username for the new user
  /// * `password` - Password for the new user
  /// * `password_confirmation` - Password confirmation to ensure correctness
  ///
  /// # Returns
  /// * `RegisterResult` containing user information
  ///
  /// # Errors
  /// * `UserError` - If password confirmation doesn't match or user creation fails due to validation, uniqueness, or database error
  #[instrument(skip(pool, session_secret), fields(username = %username))]
  pub async fn register(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    password_confirmation: &str,
    session_secret: &[u8],
  ) -> Result<RegisterResult, UserError> {
    // Validate password confirmation matches
    if password != password_confirmation {
      return Err(UserError::ValidationError(
        "Passwords do not match".to_string(),
      ));
    }

    // Create user which includes validation and password hashing
    let user = UserService::create_user(pool, username, password).await?;

    // Create session for the newly created user
    let session_token = SessionService::create_session_for_user(&user, session_secret)
      .map_err(|e| UserError::SessionError(e.to_string()))?;

    Ok(RegisterResult {
      user_id: user.id,
      username: user.name,
      uuid: user.uuid,
      role_id: user.role_id,
      created_ts: user.created_ts,
      updated_ts: user.updated_ts,
      session_token,
    })
  }

  /// Get current authenticated user information
  ///
  /// This method orchestrates getting current user information by:
  /// 1. Extracting session from context
  /// 2. Fetching user details from database
  /// 3. Combining user and session information
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context containing authentication information
  ///
  /// # Returns
  /// * `AuthMeResult` containing user and session information
  ///
  /// # Errors
  /// * `SessionError` - If no valid session exists or user not found
  #[instrument(skip(pool))]
  pub async fn get_current_user(
    pool: &SqlitePool,
    session_context: &SessionContext,
  ) -> Result<AuthMeResult, SessionError> {
    let session_payload = session_context
      .payload
      .as_ref()
      .ok_or_else(|| SessionError::AuthenticationError("No valid session".to_string()))?;

    // Get user details from database with role information
    let user_with_role = GetUserByIdWithRoleQuery::run(pool, session_payload.sub)
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?
      .ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

    Ok(AuthMeResult {
      user_id: user_with_role.user.id,
      username: user_with_role.user.name,
      uuid: user_with_role.user.uuid,
      role_id: user_with_role.user.role_id,
      role_name: user_with_role.role_name,
      created_ts: user_with_role.user.created_ts,
      updated_ts: user_with_role.user.updated_ts,
      session_iat: session_payload.iat,
      session_exp: session_payload.exp,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::services::UserService;
  use dps_auth_session::DpsAuthSessionPayload;

  // Test secret - 32 bytes for AES-256
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn test_auth_login_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test successful login
    let result = AuthService::login(&pool, "testuser", "password123", TEST_SECRET).await;

    assert!(result.is_ok());
    let auth_result = result.unwrap();
    assert_eq!(auth_result.username, "testuser");
    assert!(!auth_result.session_token.is_empty());
    assert!(auth_result.user_id > 0);
  }

  #[tokio::test]
  async fn test_auth_login_invalid_credentials() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test login with wrong password
    let result = AuthService::login(&pool, "testuser", "wrongpassword", TEST_SECRET).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_auth_login_user_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Test login with non-existent user
    let result = AuthService::login(&pool, "nonexistent", "password123", TEST_SECRET).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_current_user_with_role() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a role and user
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'testuser', 1, 'hashed_password', NULL)"
    )
    .bind("550e8400-e29b-41d4-a716-446655440000")
    .execute(&pool)
    .await
    .unwrap();
    let user_id = user_result.last_insert_rowid();

    let payload = DpsAuthSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload));

    let result = AuthService::get_current_user(&pool, &session_context).await;

    assert!(result.is_ok());
    let auth_me_result = result.unwrap();
    assert_eq!(auth_me_result.user_id, user_id);
    assert_eq!(auth_me_result.username, "testuser");
    assert_eq!(auth_me_result.role_name, "admin");
    assert_eq!(auth_me_result.session_iat, 1706356800);
    assert_eq!(auth_me_result.session_exp, 1706616000);
  }
}
