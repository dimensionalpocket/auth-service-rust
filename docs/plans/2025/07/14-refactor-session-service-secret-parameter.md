# Refactor SessionService to Accept Secret as Parameter

**Date**: 2025-07-14@06:14  
**Phase**: Refactoring - Session Service Secret Management

## Overview

Currently, the `SessionService` reads the secret key directly from the `DP_AUTH_SECRET_KEY` environment variable within its methods. This creates tight coupling to environment variables and makes testing more complex, requiring environment variable manipulation in tests.

This plan refactors the `SessionService` to accept the secret key as a method parameter instead, moving the responsibility of reading environment variables to the session middleware. This will improve testability and separation of concerns.

## Current State Analysis

### Current Implementation Issues

1. **Environment Variable Coupling**: `SessionService::encode_token()` and `SessionService::decode_token()` directly read from `DP_AUTH_SECRET_KEY`
2. **Complex Testing**: Tests require environment variable manipulation with mutexes to prevent race conditions
3. **Tight Coupling**: Service layer is tightly coupled to environment configuration
4. **Inconsistent Responsibility**: Session middleware should handle environment concerns, not the service

### Current Method Signatures

```rust
// Current signatures that read from environment internally
pub fn encode_token(payload: &SessionPayload) -> Result<String, SessionError>
pub fn decode_token(token: &str) -> Result<SessionPayload, SessionError>
pub async fn create_session(pool: &SqlitePool, username: &str, password: &str) -> Result<String, SessionError>
```

### Current Usage Points

1. **Session Middleware** (`src/middleware/session.rs`):
   - Lines 57, 66: Calls `SessionService::decode_token(&token)`

2. **Create Session Mutation** (`src/graphql/mutations/create_session.rs`):
   - Calls `SessionService::create_session(pool, username, password)`

3. **Tests**: Multiple test files manipulate environment variables

## Proposed Changes

### 1. Update SessionService Method Signatures

```rust
// New signatures that accept secret as parameter
pub fn encode_token(payload: &SessionPayload, secret: &[u8]) -> Result<String, SessionError>
pub fn decode_token(token: &str, secret: &[u8]) -> Result<SessionPayload, SessionError>
pub async fn create_session(
  pool: &SqlitePool, 
  username: &str, 
  password: &str, 
  secret: &[u8]
) -> Result<String, SessionError>
```

### 2. Create Secret Reading Utility

Create a reusable utility for reading and validating secrets from environment variables:

```rust
// In src/utils/get_secret_from_env.rs
use base64::{engine::general_purpose, Engine as _};

/// Error type for secret reading operations
#[derive(Debug, Clone, PartialEq)]
pub enum SecretError {
  /// Environment variable not found
  NotFound(String),
  /// Base64 decoding failed
  InvalidBase64(String),
  /// Secret too short for cryptographic use
  TooShort { actual: usize, minimum: usize },
  /// Secret cache already initialized
  AlreadyInitialized,
}

impl std::fmt::Display for SecretError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SecretError::NotFound(var_name) => {
        write!(f, "Environment variable '{}' not found", var_name)
      }
      SecretError::InvalidBase64(var_name) => {
        write!(f, "Environment variable '{}' contains invalid base64", var_name)
      }
      SecretError::TooShort { actual, minimum } => {
        write!(f, "Secret too short: {} bytes, need at least {}", actual, minimum)
      }
      SecretError::AlreadyInitialized => {
        write!(f, "Secret cache already initialized")
      }
    }
  }
}

impl std::error::Error for SecretError {}

/// Read and validate a base64-encoded secret from an environment variable
///
/// This function reads a secret from the specified environment variable,
/// decodes it from base64, and validates that it meets the minimum length
/// requirement for cryptographic use.
///
/// # Arguments
///
/// * `env_var_name` - Name of the environment variable to read
/// * `min_bytes` - Minimum number of bytes required (typically 32 for AES-256)
///
/// # Returns
///
/// Returns exactly `min_bytes` bytes from the decoded secret, or an error if:
/// - The environment variable is not set
/// - The value is not valid base64
/// - The decoded secret is shorter than `min_bytes`
///
/// # Examples
///
/// ```rust
/// use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
///
/// // Read a 32-byte AES-256 key
/// let secret = get_secret_from_env("MY_SECRET_KEY", 32)?;
/// assert_eq!(secret.len(), 32);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn get_secret_from_env(env_var_name: &str, min_bytes: usize) -> Result<Vec<u8>, SecretError> {
  // Read environment variable
  let key_str = std::env::var(env_var_name)
    .map_err(|_| SecretError::NotFound(env_var_name.to_string()))?;

  // Decode base64
  let key_bytes = general_purpose::STANDARD
    .decode(&key_str)
    .map_err(|_| SecretError::InvalidBase64(env_var_name.to_string()))?;

  // Validate minimum length
  if key_bytes.len() < min_bytes {
    return Err(SecretError::TooShort {
      actual: key_bytes.len(),
      minimum: min_bytes,
    });
  }

  // Return exactly the requested number of bytes
  Ok(key_bytes[..min_bytes].to_vec())
}
```

### 3. Create Shared Secret Cache

Create a shared secret cache that uses the utility function:

```rust
// In src/middleware/session.rs
use crate::utils::get_secret_from_env::{get_secret_from_env, SecretError};
use std::sync::OnceLock;

/// Global cached session secret, initialized once at startup
static SESSION_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

/// Initialize the session secret from environment variable
/// This should be called once during application startup
pub fn init_session_secret() -> Result<(), SecretError> {
  let secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)?;
  
  SESSION_SECRET.set(secret)
    .map_err(|_| SecretError::AlreadyInitialized)?;

  Ok(())
}

/// Get the cached session secret
/// Panics if the secret hasn't been initialized - this indicates a programming error
pub fn get_session_secret() -> &'static [u8] {
  SESSION_SECRET.get()
    .expect("Session secret not initialized - call init_session_secret() during startup")
}
```

### 4. Update Session Middleware

The session middleware will use the cached secret:

```rust
// In src/middleware/session.rs
fn extract_and_validate_session_sync(request: &Request) -> SessionContext {
  // Get the cached secret (no environment variable reading per request)
  let secret = get_session_secret();

  // Try header first
  if let Some(token) = extract_token_from_header(request) {
    if let Ok(payload) = SessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
    return SessionContext::new(None);
  }

  // Try cookie if no header
  if let Some(token) = extract_token_from_cookie(request) {
    if let Ok(payload) = SessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
  }

  SessionContext::new(None)
}
```

### 5. Update Create Session Mutation

The mutation will use the cached secret:

```rust
// In src/graphql/mutations/create_session.rs
use crate::middleware::session::get_session_secret;

impl CreateSessionMutation {
  pub async fn create_session(
    ctx: &Context<'_>,
    username: String,
    password: String,
  ) -> Result<CreateSessionPayload, async_graphql::Error> {
    let pool = ctx.data::<SqlitePool>()?;
    
    // Get the cached secret (no environment variable reading per request)
    let secret = get_session_secret();

    // Create session with secret parameter
    match SessionService::create_session(pool, &username, &password, secret).await {
      // ... rest of the implementation
    }
  }
}
```

### 6. Simplify Tests

Tests will no longer need environment variable manipulation:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  
  // Test secret - 32 bytes for AES-256 (base64-decoded)
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8
  ];

  #[test]
  fn test_encode_decode_roundtrip() {
    let payload = SessionService::create_payload(456);
    let token = SessionService::encode_token(&payload, TEST_SECRET).unwrap();
    let decoded_payload = SessionService::decode_token(&token, TEST_SECRET).unwrap();

    assert_eq!(payload, decoded_payload);
  }

  #[tokio::test]
  async fn test_create_session_success() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Setup: Create a user
    setup_default_role(&pool).await;
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Test: Create session with test secret
    let token = SessionService::create_session(&pool, "testuser", "password123", TEST_SECRET)
      .await
      .unwrap();

    // Verify: Token can be decoded and contains correct user ID
    let payload = SessionService::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }
}
```

## Implementation Steps

### Step 1: Create Secret Reading Utility
- Create `src/utils/mod.rs` if it doesn't exist
- Create `src/utils/get_secret_from_env.rs` with the utility function
- Implement `get_secret_from_env()` function with proper error handling
- Add comprehensive tests for the utility function
- Test various error conditions (missing env var, invalid base64, too short)

### Step 2: Update SessionService Core Methods
- Remove `get_secret_key()` private method
- Remove ALL environment variable knowledge from SessionService
- Update `encode_token()` to accept secret parameter
- Update `decode_token()` to accept secret parameter
- Update `create_session()` to accept secret parameter
- Remove `SecretKeyNotSet` and `InvalidSecretKey` error variants (no longer needed)
- Keep only cryptographic and business logic errors: `EncodingError`, `DecodingError`, `TokenExpired`, `InvalidToken`, `JsonError`, `AuthenticationError`, `DatabaseError`, `PasswordVerificationError`

### Step 3: Create Shared Secret Cache
- Add imports: `use crate::utils::get_secret_from_env::{get_secret_from_env, SecretError};` and `use std::sync::OnceLock;`
- Add `init_session_secret()` function using the utility
- Add `get_session_secret()` function to access cached secret
- Update `extract_and_validate_session_sync()` to use cached secret
- Remove per-request environment variable reading
- Add tests for cache initialization and retrieval functions

### Step 4: Update Create Session Mutation
- Add import: `use crate::middleware::session::get_session_secret;`
- Use cached secret in the mutation resolver
- Pass secret to `create_session()` call
- Remove per-request environment variable reading

### Step 5: Update Application Startup
- Call `init_session_secret()` during application startup in `main.rs`
- Application should crash (panic or exit) if secret initialization fails
- Ensure secret is initialized before starting the server

Example implementation:
```rust
// In main.rs
use dp_auth_service::middleware::session::init_session_secret;

#[tokio::main]
async fn main() {
    // Initialize session secret - crash if this fails
    init_session_secret().expect("Failed to initialize session secret");
    
    // ... rest of application startup
}
```

### Step 6: Update Tests
- Remove environment variable manipulation from SessionService tests
- Use constant test secret in tests
- Remove `ENV_MUTEX` and related complexity
- Add cache initialization in integration test setup
- Update integration tests to use new SessionService signatures
- Add tests for cache functions in middleware tests
- Integration tests should work automatically with `DP_AUTH_SECRET_KEY` from environment

### Step 7: Update Docker Build Workflow
- Add `DP_AUTH_SECRET_KEY` environment variable to Docker container startup
- This prevents Docker build tests from failing due to missing environment variable
- Use the same test value as used in the codebase

Update the container startup command in `.github/workflows/docker-build-test.yml`:
```yaml
- name: Start container and test API
  run: |
    # Start container in background with database migration and secret key
    docker run -d --name test-container -p 3000:3000 \
      -e DP_AUTH_SECRET_KEY="QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=" \
      dp-auth-service:test sh -c "./migrate_and_dump && ./dp-auth-service"
```

### Step 8: Update Documentation
- Update method documentation to reflect new parameters
- Update examples in doc comments
- Document the secret initialization requirement
- Update any README sections if needed

## Files to Modify

1. **`src/utils/mod.rs`** (create if doesn't exist)
   - Add module declaration: `pub mod get_secret_from_env;`

2. **`src/lib.rs`**
   - Add module declaration: `pub mod utils;` to make utils publicly available

3. **`src/utils/get_secret_from_env.rs`** (new file)
   - Implement `get_secret_from_env()` function
   - Define `SecretError` enum
   - Add comprehensive unit tests

4. **`src/services/session_service.rs`**
   - Update method signatures to accept secret parameter
   - Remove `get_secret_key()` private method entirely
   - Remove ALL environment variable knowledge and related error types
   - Remove imports: `std::env` (if present)
   - Keep only cryptographic and business logic error variants in `SessionError`
   - Update all tests to use test secret parameter
   - Remove environment variable manipulation in tests

5. **`src/middleware/session.rs`**
   - Add imports: `use crate::utils::get_secret_from_env::{get_secret_from_env, SecretError};` and `use std::sync::OnceLock;`
   - Add `init_session_secret()` function for startup initialization
   - Add `get_session_secret()` function for accessing cached secret
   - Update `extract_and_validate_session_sync()` to use cached secret
   - Remove per-request environment variable reading
   - Add tests for cache functions

6. **`src/graphql/mutations/create_session.rs`**
   - Add import: `use crate::middleware::session::get_session_secret;`
   - Use cached secret in mutation resolver
   - Pass secret to `create_session()` call

7. **`src/main.rs`**
   - Add import: `use dp_auth_service::middleware::session::init_session_secret;`
   - Add secret initialization during application startup
   - Handle initialization errors appropriately

8. **`tests/session_middleware_tests.rs`**
   - Add cache initialization in test setup (tests will use `DP_AUTH_SECRET_KEY` from environment)
   - Update tests to work with new middleware implementation
   - Add tests for cache functions (`init_session_secret`, `get_session_secret`)

9. **`tests/integration_tests.rs`**
   - Add cache initialization in test setup (tests will use `DP_AUTH_SECRET_KEY` from environment)
   - Update integration tests to use new SessionService signatures
   - Tests should work automatically since `DP_AUTH_SECRET_KEY` is available in test environment

10. **`.github/workflows/docker-build-test.yml`**
   - Add `DP_AUTH_SECRET_KEY` environment variable to Docker container startup
   - Use the same value as in tests to ensure Docker build tests pass

## Benefits

1. **Complete Decoupling**: SessionService has zero knowledge of environment variables
2. **Reusable Utility**: Secret reading logic can be used for other secrets in the application
3. **Improved Performance**: Secret is read once at startup, not on every request
4. **Better Caching**: Shared secret cache eliminates redundant environment variable parsing
5. **Improved Testability**: Utility function has its own comprehensive tests
6. **Better Separation of Concerns**: Service layer focuses purely on business logic
7. **Reduced Coupling**: Service is completely independent of configuration sources
8. **Cleaner Tests**: No more mutex synchronization for environment variables
9. **More Flexible**: Secret can come from any source, not just environment variables
10. **Cleaner Architecture**: Clear responsibility boundaries between layers
11. **Fail-Fast**: Secret validation happens at startup, not during request processing
12. **Better Error Handling**: Specific error types for different failure modes

## Backward Compatibility

This is a breaking change to the `SessionService` API, but since this is an internal service within the application, it only affects internal callers. The external GraphQL API remains unchanged.

## Testing Strategy

1. **Utility Tests**: Comprehensive tests for `get_secret_from_env()` function
   - Test successful secret reading with valid base64
   - Test missing environment variable error
   - Test invalid base64 error
   - Test secret too short error
   - Test exact minimum length handling
2. **Cache Tests**: Tests for cache functions in middleware
   - Test successful cache initialization
   - Test cache already initialized error
   - Test cache retrieval after initialization
   - Test panic when accessing uninitialized cache
3. **Unit Tests**: Test all SessionService methods with various secret values
4. **Integration Tests**: Ensure middleware and mutations work with new implementation
5. **Error Handling**: Test behavior when secret reading fails
6. **Regression Tests**: Ensure all existing functionality still works

## Risk Assessment

- **Low Risk**: Changes are internal to the application
- **Well Isolated**: Changes are contained to specific service and middleware layers
- **Testable**: New implementation is easier to test than current approach
- **Reversible**: Changes can be reverted if issues arise

## Success Criteria

1. All existing tests pass with new implementation
2. No environment variable manipulation needed in SessionService tests
3. Session middleware successfully reads secret and passes to service
4. Create session mutation works with new service signature
5. All integration tests continue to pass