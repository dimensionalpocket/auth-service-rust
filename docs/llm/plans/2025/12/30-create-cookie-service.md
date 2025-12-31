# Create Cookie Service

## Date Created
2025-12-30@22:06

## Problem Statement

Cookie generation code is duplicated across multiple resolvers:
- `auth_login.rs` (lines 53-65)
- `auth_register.rs` (lines 78-90)
- `auth_logout.rs` (lines 27-34)

Each resolver manually formats the cookie string using `format!()` macro with similar logic. This creates:
- Code duplication
- Maintenance burden (cookie format changes need updates in 3 places)
- Reduced testability (cookie logic is embedded in resolvers)

## Current Cookie Patterns

### Session Cookie (Login/Register)
```rust
let cookie_value = format!(
  "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
  SESSION_COOKIE_NAME,
  session_token,
  cookie_domain,
  config.api_path,
  if insecure_cookie { "" } else { "; Secure" },
  config.session_ttl_seconds
);
```

### Logout Cookie
```rust
let cookie_value = format!(
  "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
  SESSION_COOKIE_NAME,
  cookie_domain,
  config.api_path,
  if insecure_cookie { "" } else { "; Secure" }
);
```

## Solution: Create CookieService

Create a dedicated service for cookie header generation. This service will:
- Centralize cookie formatting logic
- Provide clean API for resolvers
- Be easily testable in isolation
- Simplify resolver code

## Implementation Plan

### Phase 1: Create CookieService

**File**: `src/services/cookie_service.rs`

#### 1.1 Create CookieService
```rust
pub struct CookieService;

impl CookieService {
  /// Generate a session cookie header value for setting authentication
  ///
  /// # Arguments
  /// * `config` - DPS API configuration (contains cookie settings)
  /// * `session_token` - The session token to set in the cookie
  ///
  /// # Returns
  /// * Formatted cookie header value string
  pub fn generate_session_cookie(
    config: &DpsAuthApiConfig,
    session_token: &str,
  ) -> String {
    format!(
      "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
      crate::middleware::session::SESSION_COOKIE_NAME,
      session_token,
      config.cookie_domain,
      config.api_path,
      if config.insecure_cookie { "" } else { "; Secure" },
      config.session_ttl_seconds
    )
  }

  /// Generate a logout cookie header value for clearing the session
  ///
  /// This sets the cookie expiration date to the past to effectively delete it.
  ///
  /// # Arguments
  /// * `config` - DPS API configuration (contains cookie settings)
  ///
  /// # Returns
  /// * Formatted cookie header value string
  pub fn generate_logout_cookie(config: &DpsAuthApiConfig) -> String {
    format!(
      "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      crate::middleware::session::SESSION_COOKIE_NAME,
      config.cookie_domain,
      config.api_path,
      if config.insecure_cookie { "" } else { "; Secure" }
    )
  }
}
```

#### 1.2 Add to Services Module
**File**: `src/services/mod.rs`

Add module export:
```rust
pub mod cookie_service;
pub use cookie_service::CookieService;
```

### Phase 2: Update auth_login.rs Resolver

**File**: `src/graphql/resolvers/auth_login.rs`

**Changes**:
1. Remove `use crate::middleware::session::SESSION_COOKIE_NAME;` import
2. Add `use crate::services::CookieService;` import
3. Replace cookie generation code (lines 53-65)

**Before**:
```rust
let cookie_value = format!(
  "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
  SESSION_COOKIE_NAME,
  auth_result.session_token,
  cookie_domain,
  config.api_path,
  if insecure_cookie { "" } else { "; Secure" },
  config.session_ttl_seconds
);

let _ = ctx.append_http_header("set-cookie", cookie_value);
```

**After**:
```rust
let cookie_value = CookieService::generate_session_cookie(
  config,
  &auth_result.session_token,
);

let _ = ctx.append_http_header("set-cookie", cookie_value);
```

### Phase 3: Update auth_register.rs Resolver

**File**: `src/graphql/resolvers/auth_register.rs`

**Changes**: Same as auth_login.rs

1. Remove `use crate::middleware::session::SESSION_COOKIE_NAME;` import
2. Add `use crate::services::CookieService;` import
3. Replace cookie generation code (lines 78-90)

**After**:
```rust
let cookie_value = CookieService::generate_session_cookie(
  config,
  &register_result.session_token,
);

let _ = ctx.append_http_header("set-cookie", cookie_value);
```

### Phase 4: Update auth_logout.rs Resolver

**File**: `src/graphql/resolvers/auth_logout.rs`

**Changes**:
1. Remove `use crate::middleware::session::SESSION_COOKIE_NAME;` import
2. Add `use crate::services::CookieService;` import
3. Replace cookie generation code (lines 27-34)

**Before**:
```rust
let cookie_value = format!(
  "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
  SESSION_COOKIE_NAME,
  cookie_domain,
  config.api_path,
  if insecure_cookie { "" } else { "; Secure" }
);

let _ = ctx.append_http_header("set-cookie", cookie_value);
```

**After**:
```rust
let cookie_value = CookieService::generate_logout_cookie(config);

let _ = ctx.append_http_header("set-cookie", cookie_value);
```

### Phase 5: Add CookieService Tests

**File**: `src/services/cookie_service.rs`

Add comprehensive tests:

```rust
#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_config() -> DpsAuthApiConfig {
    DpsAuthApiConfig {
      port: 8080,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: vec
![0u8; 32],
      cookie_domain: ".example.com".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: false,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    }
  }

  #[test]
  fn test_generate_session_cookie_secure() {
    let config = create_test_config();
    let cookie = CookieService::generate_session_cookie(&config, "test-token");

    assert!(cookie.starts_with("DpsAuthSession=test-token;"));
    assert!(cookie.contains("Domain=.example.com"));
    assert!(cookie.contains("Path=/api"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("Max-Age=3600"));
  }

  #[test]
  fn test_generate_session_cookie_insecure() {
    let mut config = create_test_config();
    config.insecure_cookie = true;

    let cookie = CookieService::generate_session_cookie(&config, "test-token");

    assert!(cookie.contains("DpsAuthSession=test-token;"));
    assert!(!cookie.contains("Secure"));
  }

  #[test]
  fn test_generate_logout_cookie_secure() {
    let config = create_test_config();
    let cookie = CookieService::generate_logout_cookie(&config);

    assert!(cookie.starts_with("DpsAuthSession=;"));
    assert!(cookie.contains("Domain=.example.com"));
    assert!(cookie.contains("Path=/api"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
  }

  #[test]
  fn test_generate_logout_cookie_insecure() {
    let mut config = create_test_config();
    config.insecure_cookie = true;

    let cookie = CookieService::generate_logout_cookie(&config);

    assert!(!cookie.contains("Secure"));
  }
}
```

### Phase 6: Update Resolver Tests

Update existing resolver tests to verify they still pass with the new CookieService:

**Files**:
- `src/graphql/resolvers/auth_login.rs` - Tests in lines 112-296
- `src/graphql/resolvers/auth_register.rs` - Tests in lines 121-422
- `src/graphql/resolvers/auth_logout.rs` - Tests in lines 45-241

All tests should pass with minimal changes since cookie output format is identical.

## Benefits of This Approach

1. **Single Source of Truth**: All cookie formatting logic lives in one place
2. **No Abstraction Bloat**: Uses `DpsAuthApiConfig` directly, no intermediate config struct
3. **Testability**: CookieService can be unit tested independently
4. **Maintainability**: Changes to cookie format only require updates in one file
5. **Consistency**: No risk of divergent cookie formats across resolvers
6. **Clarity**: Resolvers focus on GraphQL concerns, not cookie formatting

## Summary

| Phase | Description | Files Created | Files Modified |
|-------|-------------|----------------|----------------|
| 1 | Create CookieService with cookie generation methods | `src/services/cookie_service.rs` | `src/services/mod.rs` |
| 2 | Update auth_login.rs to use CookieService | - | `src/graphql/resolvers/auth_login.rs` |
| 3 | Update auth_register.rs to use CookieService | - | `src/graphql/resolvers/auth_register.rs` |
| 4 | Update auth_logout.rs to use CookieService | - | `src/graphql/resolvers/auth_logout.rs` |
| 5 | Add comprehensive CookieService tests | - | `src/services/cookie_service.rs` |
| 6 | Verify all resolver tests still pass | - | All test files |

## Notes

- The cookie format (attributes, order, spacing) is preserved exactly as-is to maintain compatibility
- SESSION_COOKIE_NAME constant is accessed directly in CookieService from middleware module
- Uses `&DpsAuthApiConfig` directly - no intermediate config struct, no bloat
- No breaking changes to existing behavior - cookie strings will be identical
- CookieService is stateless and has no side effects, making it perfect for unit testing
