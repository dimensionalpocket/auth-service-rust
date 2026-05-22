# Plan: Configure Session Cookie Expiration

(THIS PLAN WAS SCRAPPED IN FAVOR OF MOVING SESSION DURATION CONFIGURATION TO THE DpsConfig CRATE.)

## Purpose

Enable consumers of the `dps-auth-api` library to configure session expiration times while maintaining sensible defaults and proper JWT token/cookie synchronization.

## Current State Analysis

**Current Implementation:**
- [`DpsAuthSession::create_payload(user_id, expiration_seconds)`](https://github.com/dimensionalpocket/dps-auth-session-rs) accepts an optional `expiration_seconds` parameter
- Currently hardcoded as `None` in [`SessionService::create_session()`](src/services/session_service.rs:152)
- External crate default: **3 days** (259,200 seconds)
- Cookie `Max-Age` is hardcoded to **3 days** in [`create_session` resolver](src/graphql/resolvers/create_session.rs:55)

**Problem:**
- No way for library consumers to customize session duration
- Cookie expiration and JWT token expiration are independently hardcoded (maintenance burden)
- Configuration approach unclear for library usage

## Industry Standards Research

### Session Duration Best Practices

**Common defaults:**
- **Short-lived sessions (web apps):** 15-30 minutes with refresh tokens
- **Standard sessions:** 1-24 hours
- **Extended sessions ("remember me"):** 7-30 days
- **API tokens:** 1-90 days

**Popular frameworks:**
- **Express.js (express-session):** Default 14 days, configurable via `maxAge`
- **Django:** Default 2 weeks (`SESSION_COOKIE_AGE`)
- **Spring Security:** Default 30 minutes for stateful, configurable
- **Passport.js:** No default, requires explicit configuration

**Security recommendations:**
- OWASP: Session timeout between 2-5 minutes (idle) to 15-30 minutes (absolute) for high-security apps
- PCI DSS: Maximum 15 minutes of inactivity for payment applications
- General web apps: 30 minutes to 24 hours is common

**Our current 3-day default** is reasonable for a general-purpose auth API but should be configurable.

### Library Configuration Patterns in Rust

**Builder Pattern (Preferred for Libraries):**
```rust
let server = DpsAuthApi::new()
    .session_duration_seconds(3600) // 1 hour
    .build()?;
```
✅ Type-safe, compile-time checked
✅ Clear API contract
✅ No hidden environment dependencies
✅ Composable and testable

**Environment Variables (Acceptable for Runtime Override):**
```rust
// Used in run_local_server.rs binary, NOT in library code
let duration = env::var("DPS_AUTH_SESSION_DURATION_SECONDS")
    .ok()
    .and_then(|s| s.parse().ok());
```
✅ Good for deployment-time configuration
✅ Doesn't pollute library API
⚠️ Should be documented for binary usage only

**Industry consensus:** Libraries should use builder pattern for configuration. Environment variables are acceptable for binaries/examples that use the library.

## Design Decisions

### 1. Configuration Approach

**Primary: Builder Pattern**
- Add `session_duration_seconds(u64)` method to [`DpsAuthApiBuilder`](src/dps_auth_api_builder.rs)
- Store in config, pass to GraphQL context
- Available to resolvers and services

**Secondary: Environment Variable (Binary Only)**
- `DPS_AUTH_SESSION_DURATION_SECONDS` for [`run_local_server.rs`](scripts/run_local_server.rs)
- Not used within library code itself
- Documented as deployment convenience

### 2. Precedence Rules

**Single source of truth:** Builder configuration only
- If `session_duration_seconds()` is called → use that value
- If not called → use default (3 days, 259,200 seconds)
- No complex precedence chains

**Rationale:** Libraries should have predictable, explicit behavior. The consuming application can implement its own precedence logic if needed.

### 3. Cookie Max-Age Synchronization

**Problem:** Cookie expiration must match JWT token expiration

**Solution:** 
- Pass configured duration to `DpsAuthSession::create_payload()` as `Some(duration)`
- Use same duration for cookie `Max-Age`
- Single source of truth prevents drift

### 4. Validation Rules

**Minimum:** 60 seconds (1 minute)
- Prevents accidental ultra-short sessions
- Security: prevents some timing attacks

**Maximum:** 31,536,000 seconds (365 days)
- Prevents overflow issues
- Security: reasonable upper bound

**Invalid values:** Return clear error during `build()`

## Implementation Plan

### Files to Modify

1. **[`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs)** - Add session duration configuration
2. **[`src/dps_auth_api.rs`](src/dps_auth_api.rs)** - Store in config, add to GraphQL context
3. **[`src/services/session_service.rs`](src/services/session_service.rs)** - Accept and use duration parameter
4. **[`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs)** - Use configured duration for cookie
5. **[`scripts/run_local_server.rs`](scripts/run_local_server.rs)** - Add env var support
6. **[`README.md`](README.md)** - Document new configuration option

### Step-by-Step Implementation

#### Step 1: Add Builder Configuration

**File:** [`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs)

```rust
#[derive(Debug, Default)]
pub struct DpsAuthApiBuilder {
  port: Option<u16>,
  sqlite_file_path: Option<String>,
  session_secret: Option<Vec<u8>>,
  cookie_domain: Option<String>,
  insecure_cookie: Option<bool>,
  development_mode: Option<bool>,
  database_pool_size: Option<u32>,
  session_duration_seconds: Option<u64>, // NEW
}

impl DpsAuthApiBuilder {
  // ... existing methods ...

  /// Set the session duration in seconds.
  /// 
  /// The session duration determines how long a user's session remains valid
  /// after login. This affects both the JWT token expiration and the cookie Max-Age.
  ///
  /// # Arguments
  /// * `seconds` - Duration in seconds (min: 60, max: 31,536,000)
  ///
  /// # Default
  /// If not set, defaults to 259,200 seconds (3 days).
  ///
  /// # Examples
  /// ```
  /// use dps_auth_api::DpsAuthApi;
  /// 
  /// let server = DpsAuthApi::new()
  ///     .session_duration_seconds(3600) // 1 hour
  ///     .session_secret(secret)
  ///     .build()?;
  /// ```
  pub fn session_duration_seconds(mut self, seconds: u64) -> Self {
    self.session_duration_seconds = Some(seconds);
    self
  }

  pub fn build(self) -> Result<DpsAuthApi, DpsAuthApiError> {
    // Validate session duration if provided
    if let Some(duration) = self.session_duration_seconds {
      const MIN_DURATION: u64 = 60; // 1 minute
      const MAX_DURATION: u64 = 31_536_000; // 365 days
      
      if duration < MIN_DURATION {
        return Err(DpsAuthApiError::InvalidSessionDuration {
          value: duration,
          min: MIN_DURATION,
          max: MAX_DURATION,
        });
      }
      
      if duration > MAX_DURATION {
        return Err(DpsAuthApiError::InvalidSessionDuration {
          value: duration,
          min: MIN_DURATION,
          max: MAX_DURATION,
        });
      }
    }

    let config = ResolvedServerConfig {
      port: self.port.unwrap_or(3000),
      sqlite_file_path: self
        .sqlite_file_path
        .unwrap_or_else(|| "data/development.db".to_string()),
      session_secret: self
        .session_secret
        .ok_or(DpsAuthApiError::MissingRequiredConfig {
          field: "session_secret".to_string(),
        })?,
      cookie_domain: self
        .cookie_domain
        .unwrap_or_else(|| ".api.dps.localhost".to_string()),
      insecure_cookie: self.insecure_cookie.unwrap_or(false),
      development_mode: self.development_mode.unwrap_or(false),
      database_pool_size: self.database_pool_size,
      session_duration_seconds: self.session_duration_seconds.unwrap_or(259_200), // 3 days default
    };

    // Existing validation...
    if config.session_secret.len() != 32 {
      return Err(DpsAuthApiError::InvalidSecretLength {
        actual: config.session_secret.len(),
        expected: 32,
      });
    }

    Ok(DpsAuthApi { config })
  }
}
```

#### Step 2: Update Config Structure and Error Type

**File:** [`src/dps_auth_api.rs`](src/dps_auth_api.rs)

```rust
#[derive(Debug)]
pub struct ResolvedServerConfig {
  pub port: u16,
  pub sqlite_file_path: String,
  pub session_secret: Vec<u8>,
  pub cookie_domain: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
  pub database_pool_size: Option<u32>,
  pub session_duration_seconds: u64, // NEW
}

#[derive(Debug)]
pub enum DpsAuthApiError {
  MissingRequiredConfig { field: String },
  InvalidSecretLength { actual: usize, expected: usize },
  InvalidSessionDuration { value: u64, min: u64, max: u64 }, // NEW
  DatabaseError(String),
  ServerError(String),
}

impl fmt::Display for DpsAuthApiError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      DpsAuthApiError::MissingRequiredConfig { field } => {
        write!(f, "Missing required configuration: {}", field)
      }
      DpsAuthApiError::InvalidSecretLength { actual, expected } => {
        write!(
          f,
          "Invalid secret length: expected {} bytes, got {}",
          expected, actual
        )
      }
      DpsAuthApiError::InvalidSessionDuration { value, min, max } => {
        write!(
          f,
          "Invalid session duration: {} seconds (must be between {} and {})",
          value, min, max
        )
      }
      DpsAuthApiError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
      DpsAuthApiError::ServerError(msg) => write!(f, "Server error: {}", msg),
    }
  }
}
```

Add to GraphQL context setup in the `start()` method:

```rust
// In the start() method where GraphQL schema is built
let schema = Schema::build(MutationRoot, QueryRoot, EmptySubscription)
  .data(pool.clone())
  .data(self.config.session_secret.clone())
  .data(self.config.cookie_domain.clone())
  .data(self.config.insecure_cookie)
  .data(self.config.session_duration_seconds) // NEW
  .finish();
```

#### Step 3: Update Session Service

**File:** [`src/services/session_service.rs`](src/services/session_service.rs)

```rust
impl SessionService {
  /// Create a new session by authenticating user credentials
  ///
  /// # Arguments
  ///
  /// * `pool` - Database connection pool for user lookup
  /// * `username` - The username to authenticate
  /// * `password` - The plaintext password to verify
  /// * `secret` - The 32-byte secret key for token encryption
  /// * `duration_seconds` - Session duration in seconds (uses external crate default if None)
  ///
  /// # Returns
  ///
  /// Returns a session token string on successful authentication
  pub async fn create_session(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    secret: &[u8],
    duration_seconds: Option<u64>, // NEW parameter
  ) -> Result<String, SessionError> {
    // ... existing validation and authentication logic ...

    // Create session payload with configurable expiration
    let payload = DpsAuthSession::create_payload(user.id, duration_seconds);
    let token = DpsAuthSession::encode_token(&payload, secret)?;

    tracing::info!(
      username = username,
      user_id = user.id,
      duration_seconds = ?duration_seconds,
      "Authentication successful"
    );

    Ok(token)
  }
}
```

#### Step 4: Update Create Session Resolver

**File:** [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs)

```rust
#[Object]
impl CreateSessionResolver {
  #[instrument(skip(self, ctx, input), fields(username = %input.username))]
  async fn create_session(
    &self,
    ctx: &Context<'_>,
    input: CreateSessionInput,
  ) -> Result<CreateSessionResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_secret = ctx.data::<Vec<u8>>()?;
    let session_duration_seconds = *ctx.data::<u64>()?; // NEW

    match SessionService::create_session(
      pool,
      &input.username,
      &input.password,
      session_secret,
      Some(session_duration_seconds), // NEW: Pass configured duration
    )
    .await
    {
      Ok(token) => {
        let default_domain = ".api.dps.localhost".to_string();
        let cookie_domain = ctx.data::<String>().unwrap_or(&default_domain);
        let insecure_cookie = *ctx.data::<bool>().unwrap_or(&false);

        let cookie_value = format!(
          "{}={}; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Max-Age={}",
          crate::middleware::session::SESSION_COOKIE_NAME,
          token,
          cookie_domain,
          if insecure_cookie { "" } else { "; Secure" },
          session_duration_seconds // Use configured duration
        );

        let _ = ctx.append_http_header("set-cookie", cookie_value);

        Ok(CreateSessionResponse {
          token,
          message: "Authentication successful".to_string(),
        })
      }
      Err(session_error) => {
        let user_message = map_session_error_to_user_message(&session_error);
        Err(async_graphql::Error::new(user_message))
      }
    }
  }
}
```

#### Step 5: Update Local Server Binary

**File:** [`scripts/run_local_server.rs`](scripts/run_local_server.rs)

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // ... existing environment variable loading ...

  // Load session duration from environment
  let session_duration_seconds = env::var("DPS_AUTH_SESSION_DURATION_SECONDS")
    .ok()
    .and_then(|s| s.parse::<u64>().ok());

  let mut builder = DpsAuthApi::new()
    .port(port)
    .sqlite_file_path(sqlite_file)
    .session_secret(secret_key)
    .cookie_domain(cookie_domain)
    .development_mode(is_development);

  if let Ok(insecure) = env::var("DPS_AUTH_INSECURE_COOKIE") {
    if !insecure.is_empty() {
      builder = builder.insecure_cookie(true);
    }
  }

  // NEW: Apply session duration if provided
  if let Some(duration) = session_duration_seconds {
    builder = builder.session_duration_seconds(duration);
  }

  let server = builder.build()?;
  server.start().await
}
```

#### Step 6: Update Tests

Add unit tests to [`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs):

```rust
#[cfg(test)]
mod session_duration_tests {
  use super::*;

  #[test]
  fn test_default_session_duration() {
    let secret = vec![1u8; 32];
    let server = DpsAuthApiBuilder::default()
      .session_secret(secret)
      .build()
      .unwrap();
    
    assert_eq!(server.config.session_duration_seconds, 259_200); // 3 days
  }

  #[test]
  fn test_custom_session_duration() {
    let secret = vec![1u8; 32];
    let server = DpsAuthApiBuilder::default()
      .session_duration_seconds(3600) // 1 hour
      .session_secret(secret)
      .build()
      .unwrap();
    
    assert_eq!(server.config.session_duration_seconds, 3600);
  }

  #[test]
  fn test_session_duration_too_short() {
    let secret = vec![1u8; 32];
    let result = DpsAuthApiBuilder::default()
      .session_duration_seconds(30) // 30 seconds - below minimum
      .session_secret(secret)
      .build();
    
    assert!(matches!(
      result,
      Err(DpsAuthApiError::InvalidSessionDuration { value: 30, min: 60, max: 31_536_000 })
    ));
  }

  #[test]
  fn test_session_duration_too_long() {
    let secret = vec![1u8; 32];
    let result = DpsAuthApiBuilder::default()
      .session_duration_seconds(40_000_000) // > 1 year
      .session_secret(secret)
      .build();
    
    assert!(matches!(
      result,
      Err(DpsAuthApiError::InvalidSessionDuration { .. })
    ));
  }

  #[test]
  fn test_session_duration_boundary_values() {
    let secret = vec![1u8; 32];
    
    // Minimum valid
    let server = DpsAuthApiBuilder::default()
      .session_duration_seconds(60)
      .session_secret(secret.clone())
      .build()
      .unwrap();
    assert_eq!(server.config.session_duration_seconds, 60);
    
    // Maximum valid
    let server = DpsAuthApiBuilder::default()
      .session_duration_seconds(31_536_000)
      .session_secret(secret)
      .build()
      .unwrap();
    assert_eq!(server.config.session_duration_seconds, 31_536_000);
  }
}
```

Update existing session service tests in [`src/services/session_service.rs`](src/services/session_service.rs):

```rust
#[tokio::test]
async fn test_create_session_with_custom_duration() {
  let (pool, _temp_file) = create_test_database().await;
  
  setup_default_role(&pool).await;
  let user = UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  // Test with 1 hour duration
  let token = SessionService::create_session(
    &pool,
    "testuser",
    "password123",
    TEST_SECRET,
    Some(3600), // 1 hour
  )
  .await
  .unwrap();

  let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
  assert_eq!(payload.sub, user.id);
  
  // Verify expiration is approximately 1 hour from now
  let expected_exp = chrono::Utc::now().timestamp() + 3600;
  assert!((payload.exp - expected_exp).abs() < 5); // Within 5 seconds tolerance
}

#[tokio::test]
async fn test_create_session_with_default_duration() {
  let (pool, _temp_file) = create_test_database().await;
  
  setup_default_role(&pool).await;
  let user = UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  // Test with None (should use external crate default of 3 days)
  let token = SessionService::create_session(
    &pool,
    "testuser",
    "password123",
    TEST_SECRET,
    None,
  )
  .await
  .unwrap();

  let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
  assert_eq!(payload.sub, user.id);
  
  // Verify expiration is approximately 3 days from now
  let expected_exp = chrono::Utc::now().timestamp() + (3 * 24 * 60 * 60);
  assert!((payload.exp - expected_exp).abs() < 5);
}
```

Add integration test file:

**New file:** `tests/session_duration_integration_tests.rs`

```rust
use dps_auth_api::DpsAuthApi;

#[tokio::test]
async fn test_session_duration_configuration() {
  let secret = vec![1u8; 32];
  
  // Test that custom duration is respected
  let server = DpsAuthApi::new()
    .session_duration_seconds(7200) // 2 hours
    .session_secret(secret)
    .sqlite_file_path(":memory:")
    .build()
    .unwrap();
  
  assert_eq!(server.config.session_duration_seconds, 7200);
}

#[tokio::test]
async fn test_cookie_max_age_matches_session_duration() {
  // This would require a full integration test with actual HTTP requests
  // Left as TODO for implementation phase
}
```

#### Step 7: Update Documentation

**File:** [`README.md`](README.md)

Add to Configuration section:

```markdown
## Configuration

The `DpsAuthApi` builder accepts the following configuration options:

- `port(u16)`: Server port (default: 3000)
- `sqlite_file_path(String)`: SQLite database file path (default: "data/development.db")
- `session_secret(Vec<u8>)`: 32-byte secret for session encryption (required)
- `session_duration_seconds(u64)`: Session duration in seconds (default: 259,200 = 3 days, min: 60, max: 31,536,000)
- `cookie_domain(String)`: Domain for session cookies (default: ".api.dps.localhost")
- `insecure_cookie(bool)`: Whether to use insecure cookies for development (default: false)
- `development_mode(bool)`: Enable development features like GraphQL playground (default: false)

### Session Duration

The session duration determines how long a user's session remains valid after login. This affects:
- The JWT token expiration (`exp` claim)
- The cookie `Max-Age` attribute

**Common durations:**
- 1 hour: `3600`
- 8 hours: `28800`
- 1 day: `86400`
- 3 days: `259200` (default)
- 7 days: `604800`
- 30 days: `2592000`

**Example:**
```rust
let server = DpsAuthApi::new()
    .session_duration_seconds(3600) // 1 hour sessions
    .session_secret(your_secret)
    .build()?;
```

**Validation:**
- Minimum: 60 seconds (1 minute)
- Maximum: 31,536,000 seconds (365 days)
- Invalid values will cause `build()` to return an error

### Environment Variables (for run_local_server binary)

The `run_local_server` binary supports environment variable configuration:

- `DPS_AUTH_SECRET_KEY`: Base64-encoded 32-byte secret (required)
- `DPS_AUTH_SQLITE_FILE`: SQLite database file path (optional)
- `PORT`: Server port (optional)
- `DPS_AUTH_COOKIE_DOMAIN`: Cookie domain (optional)
- `DPS_AUTH_INSECURE_COOKIE`: Set to enable insecure cookies (optional)
- `DPS_AUTH_ENV`: Set to "development" to enable development mode (optional)
- `DPS_AUTH_SESSION_DURATION_SECONDS`: Session duration in seconds (optional)

**Note:** When using the library directly in your code, use the builder pattern instead of environment variables.
```

## Summary of Changes

### Modified Files
1. [`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs) - Add `session_duration_seconds()` method
2. [`src/dps_auth_api.rs`](src/dps_auth_api.rs) - Add field to config, new error variant, add to GraphQL context
3. [`src/services/session_service.rs`](src/services/session_service.rs) - Accept duration parameter, pass to external crate
4. [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs) - Use configured duration for cookie Max-Age
5. [`scripts/run_local_server.rs`](scripts/run_local_server.rs) - Support env var for binary usage
6. [`README.md`](README.md) - Document new configuration option

### New Files
1. `tests/session_duration_integration_tests.rs` - Integration tests for session duration

### Key Features
- ✅ Builder pattern for type-safe configuration
- ✅ Sensible default (3 days) matches existing behavior
- ✅ Validation (60s - 365 days)
- ✅ Cookie Max-Age synchronized with JWT expiration
- ✅ Environment variable support for binary (not library)
- ✅ Comprehensive tests
- ✅ Clear documentation

## Testing Strategy

### Unit Tests
- Default duration uses 3 days
- Custom duration is applied correctly
- Validation rejects too-short durations (< 60s)
- Validation rejects too-long durations (> 365 days)
- Boundary values (60s, 31,536,000s) are accepted
- Session service creates tokens with correct expiration

### Integration Tests
- Full HTTP flow with custom duration
- Cookie Max-Age matches token expiration
- Environment variable works in binary

### Manual Testing
```bash
# Test with default (3 days)
cargo run --bin run_local_server

# Test with 1 hour
DPS_AUTH_SESSION_DURATION_SECONDS=3600 cargo run --bin run_local_server

# Test validation
DPS_AUTH_SESSION_DURATION_SECONDS=30 cargo run --bin run_local_server  # Should error
```

## Migration Guide

### For Existing Users

**No breaking changes** - default behavior remains identical (3 day sessions).

**To customize session duration:**

```rust
// Before
let server = DpsAuthApi::new()
    .session_secret(secret)
    .build()?;

// After (optional customization)
let server = DpsAuthApi::new()
    .session_duration_seconds(3600) // 1 hour
    .session_secret(secret)
    .build()?;
```

## Security Considerations

1. **Validation**: Prevents both too-short (timing attack risk) and too-long (security risk) sessions
2. **Synchronization**: Cookie and JWT expiration always match
3. **No logging**: Duration values logged at INFO level only, never tokens
4. **Sensible default**: 3 days balances security and UX

## Alternative Approaches Considered

### ❌ Per-Request Duration Override
**Rejected:** Adds complexity, no clear use case for varying duration per login

### ❌ Complex Precedence Chain (env > config > default)
**Rejected:** Libraries should be explicit, not magic. Let consumers handle precedence.

### ❌ Separate Cookie and Token Duration
**Rejected:** Creates synchronization issues and confusion

### ✅ Chosen: Simple Builder Pattern
**Accepted:** Clear, type-safe, predictable

## Common Session Duration Examples

```rust
// Very short (development/testing)
.session_duration_seconds(300) // 5 minutes

// Short-lived (high security)
.session_duration_seconds(1800) // 30 minutes

// Standard (most web apps)
.session_duration_seconds(28800) // 8 hours

// Default (current behavior)
.session_duration_seconds(259200) // 3 days

// Extended ("remember me")
.session_duration_seconds(2592000) // 30 days
```

## Next Steps

Ready for implementation. This plan:
1. ✅ Uses industry-standard builder pattern
2. ✅ Maintains backward compatibility
3. ✅ Synchronizes cookie and JWT expiration
4. ✅ Includes comprehensive validation
5. ✅ Has clear documentation
6. ✅ Includes thorough test coverage