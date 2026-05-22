# Phase 10: SessionService::create_session Method

## Overview

Implement the `create_session` method in `SessionService` to handle user authentication by validating username/password credentials and creating session tokens. This method will serve as the foundation for the upcoming `createSession` GraphQL mutation.

## Requirements from README

- Accept username and password as input
- Call `UserService::get_user_by_name` to retrieve the user by username
- Call `PasswordService::verify` to check the password against the stored hash
- If valid, call `SessionService::encode_token` to create a session token
- Return the session token, or an error if the credentials are invalid
- Error messages should be specific (e.g., "User is blank" or "User not found" or "Password is blank" or "Password does not match", etc)
- Errors should be logged using the existing logging system and must contain the given username (not the password) for debugging
- Errors should be logged at `info` level, not `warn` or `error` -- those errors should not raise alarms in our logs, they're just informational
- Verify existing CORS configuration for 'with_credentials' support
- Unit tests for the `create_session` method
- Document the `create_session` method via Rust doc comments

## Current State Analysis

### Existing Components
- **SessionService**: Already has `encode_token`, `decode_token`, and `create_payload` methods
- **UserService**: Has `get_user_by_name` method that returns `Result<Option<User>, sqlx::Error>`
- **PasswordService**: Has `verify` method that returns `Result<bool, PasswordError>`
- **Logging**: Uses `tracing::info!` macro for logging
- **CORS**: Currently configured with `CorsLayer::permissive()` in main.rs

### Database Pool Access
The method will need access to `SqlitePool` to call `UserService::get_user_by_name`. Following the existing pattern, the pool will be passed as a parameter.

## Implementation Plan

### 1. Define New Error Types

Add new error variants to `SessionError` enum in `src/services/session_service.rs`:

```rust
pub enum SessionError {
  // ... existing variants ...
  /// Authentication failed - user input validation
  AuthenticationError(String),
  /// Database operation failed during authentication
  DatabaseError(String),
  /// Password verification failed
  PasswordVerificationError(String),
}
```

### 2. Implement `create_session` Method

Add the new method to `SessionService` in `src/services/session_service.rs`:

```rust
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
      tracing::info!(username = username, error = error_msg, "Authentication failed");
      return Err(SessionError::AuthenticationError(error_msg.to_string()));
    }

    // Retrieve user by username
    let user = match UserService::get_user_by_name(pool, username).await {
      Ok(Some(user)) => user,
      Ok(None) => {
        let error_msg = "User not found";
        tracing::info!(username = username, error = error_msg, "Authentication failed");
        return Err(SessionError::AuthenticationError(error_msg.to_string()));
      }
      Err(db_error) => {
        let error_msg = format!("Database error during user lookup: {}", db_error);
        tracing::info!(username = username, error = %db_error, "Authentication failed");
        return Err(SessionError::DatabaseError(error_msg));
      }
    };

    // Verify password
    let password_matches = match PasswordService::verify(password, &user.password_hash) {
      Ok(matches) => matches,
      Err(password_error) => {
        let error_msg = format!("Password verification error: {}", password_error);
        tracing::info!(username = username, error = %password_error, "Authentication failed");
        return Err(SessionError::PasswordVerificationError(error_msg));
      }
    };

    if !password_matches {
      let error_msg = "Password does not match";
      tracing::info!(username = username, error = error_msg, "Authentication failed");
      return Err(SessionError::AuthenticationError(error_msg.to_string()));
    }

    // Create session payload and encode token
    let payload = Self::create_payload(user.id);
    let token = Self::encode_token(&payload)?;

    tracing::info!(username = username, user_id = user.id, "Authentication successful");

    Ok(token)
  }
}
```

### 3. Update Error Display Implementation

Update the `Display` implementation for `SessionError` to handle new variants:

```rust
impl fmt::Display for SessionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      // ... existing variants ...
      SessionError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SessionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      SessionError::PasswordVerificationError(msg) => write!(f, "Password verification error: {msg}"),
    }
  }
}
```

### 4. Add Required Imports

Add necessary imports to `src/services/session_service.rs`:

```rust
use crate::services::{UserService, PasswordService};
use sqlx::SqlitePool;
use tracing;
```

### 5. Update Module Exports

No changes needed to `src/services/mod.rs` as `SessionError` is already exported.

### 6. CORS Configuration Verification

The current CORS configuration uses `CorsLayer::permissive()` which allows:
- All origins
- All methods
- All headers
- Credentials (cookies)

This configuration already supports 'with_credentials' for cookie-based authentication. No changes needed.

### 7. Comprehensive Unit Tests

Add extensive unit tests in the `tests` module of `src/services/session_service.rs`:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  // ... existing test imports ...
  use crate::database::test_utils::create_test_database;
  use crate::services::UserService;

  #[tokio::test]
  async fn test_create_session_success() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;
    
    // Setup: Create a user
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "testuser", "password123").await.unwrap();
    
    // Test: Create session
    let token = SessionService::create_session(&pool, "testuser", "password123").await.unwrap();
    
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
    assert!(result.unwrap_err().to_string().contains("Password is blank"));
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
    UserService::create_user(&pool, "testuser", "correct_password").await.unwrap();
    
    // Test: Try with wrong password
    let result = SessionService::create_session(&pool, "testuser", "wrong_password").await;
    
    assert!(matches!(result, Err(SessionError::AuthenticationError(_))));
    assert!(result.unwrap_err().to_string().contains("Password does not match"));
  }

  #[tokio::test]
  async fn test_create_session_case_insensitive_username() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;
    
    // Setup: Create a user with mixed case
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "TestUser", "password123").await.unwrap();
    
    // Test: Login with different case
    let token = SessionService::create_session(&pool, "testuser", "password123").await.unwrap();
    
    // Verify: Token contains correct user ID
    let payload = SessionService::decode_token(&token).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(pool)
      .await
      .unwrap();
  }
}
```

## Files to be Modified

1. **src/services/session_service.rs**
   - Add new error variants to `SessionError` enum
   - Add `create_session` method implementation
   - Update `Display` implementation for new error variants
   - Add comprehensive unit tests
   - Add required imports

## Dependencies

- Existing `UserService::get_user_by_name` method
- Existing `PasswordService::verify` method
- Existing `SessionService::create_payload` and `encode_token` methods
- Existing logging infrastructure (`tracing::info!`)
- Existing test utilities (`create_test_database`)

## Testing Strategy

- **Unit Tests**: Cover all error conditions and success scenarios
- **Integration Tests**: Will be added in Phase 11 with the GraphQL mutation
- **Logging Verification**: Ensure all authentication attempts are logged appropriately
- **Security Testing**: Verify timing consistency and no information leakage

## Success Criteria

- [ ] `create_session` method successfully authenticates valid credentials
- [ ] Method returns appropriate error messages for all invalid input scenarios
- [ ] All authentication attempts are logged at info level with username
- [ ] Unit tests achieve 100% code coverage for the new method
- [ ] CORS configuration verified to support credentials
- [ ] Documentation follows existing patterns and is comprehensive

## Next Steps

After Phase 10 completion, Phase 11 will implement the `createSession` GraphQL mutation that uses this new method to provide authentication endpoints for client applications.