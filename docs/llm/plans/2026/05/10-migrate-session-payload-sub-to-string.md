# Migrate SessionPayload.sub from i64 to String

## Background

The `dps-auth-session` crate has been updated to 0.3.0 with a breaking change: the `sub` property in `DpsAuthSessionPayload` has changed from `i64` to `String`. The repository URL has also changed (removed `-rs` suffix).

The `Cargo.toml` has already been updated to point to the new URL and tag.

## Impact Summary

- **162 compilation errors** across **44 files** (lib + tests)
- All errors are type mismatches (`E0308`) related to `sub` being `String` instead of `i64`
- The database layer (`User.id`, queries) still uses `i64`, so we need to parse the string back to `i64` where needed

## Changes Required

### 1. Cargo.toml (DONE)

Already updated:
```toml
dps-auth-session = { git = "https://github.com/dimensionalpocket/dps-auth-session", tag = "0.3.0" }
```

### 2. Delete GraphQL Type: `src/graphql/types/session_payload.rs`

This file is dead code - it's exported from `src/graphql/types/mod.rs` but never imported or used anywhere in the codebase. All resolvers and tests use `DpsAuthSessionPayload` directly from the `dps_auth_session` crate.

**Delete:**
- `src/graphql/types/session_payload.rs`

**Update `src/graphql/types/mod.rs`:**
Remove the session_payload module and re-export:
```rust
// Remove these lines:
pub mod session_payload;
pub use session_payload::SessionPayload;
```

### 3. Move DpsAuthApiConfig to types module

Move `DpsAuthApiConfig` from `src/dps_auth_api.rs` to `src/types/dps_auth_api_config.rs`:

**Create `src/types/dps_auth_api_config.rs`:**
```rust
#[derive(Debug, Clone)]
pub struct DpsAuthApiConfig {
  pub port: u16,
  pub sqlite_main_file_path: String,
  pub sqlite_session_file_path: String,
  pub session_secret: Vec<u8>,
  pub cookie_domain: String,
  pub api_path: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
  pub sqlite_main_pool_size: u16,
  pub sqlite_session_pool_size: u16,
  pub session_ttl_seconds: u32,
}
```

**Update `src/types/mod.rs`:**
```rust
pub mod dps_auth_api_config;
// ... existing exports
pub use dps_auth_api_config::DpsAuthApiConfig;
```

**Update `src/dps_auth_api.rs`:**
- Remove the `DpsAuthApiConfig` struct definition
- Add `use crate::types::DpsAuthApiConfig;`

**Update `src/lib.rs`:**
- Change `pub use dps_auth_api::{DpsAuthApi, DpsAuthApiConfig, DpsAuthApiError};` to `pub use dps_auth_api::{DpsAuthApi, DpsAuthApiError};`
- `DpsAuthApiConfig` is now exported via `types` module

**Update all files that import `DpsAuthApiConfig`:**
- `src/test_utils/mod.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/orchestrators/auth/auth_login.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/orchestrators/auth/auth_register.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/graphql/resolvers/auth_login.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/graphql/resolvers/auth_register.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/graphql/resolvers/auth_logout.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/services/cookie/generate_logout_cookie.rs`: `use crate::types::DpsAuthApiConfig;`
- `src/services/cookie/generate_session_cookie.rs`: `use crate::types::DpsAuthApiConfig;`

### 4. Create utility function: `src/utils/session_context_sub_to_user_id.rs`

Create new `src/utils/` folder with a single utility function that extracts and parses the user ID from a session context.

**Update `src/utils/mod.rs`:**
```rust
pub mod session_context_sub_to_user_id;
pub mod user_to_session_sub;

pub use session_context_sub_to_user_id::session_context_sub_to_user_id;
pub use user_to_session_sub::user_to_session_sub;
```

**Create `src/utils/session_context_sub_to_user_id.rs`:**
```rust
use crate::middleware::session::SessionContext;
use crate::types::DpsAuthApiConfig;

/// Extracts the user ID from a session context by parsing the string `sub` field.
///
/// # Arguments
/// * `session_context` - The session context containing the payload
/// * `_config` - The API config (reserved for future use, e.g., validation rules)
///
/// # Returns
/// * `Ok(i64)` - The parsed user ID
/// * `Err(String)` - Error message if session is missing or sub cannot be parsed
pub fn session_context_sub_to_user_id(
  session_context: &SessionContext,
  _config: &DpsAuthApiConfig,
) -> Result<i64, String> {
  let payload = session_context
    .payload
    .as_ref()
    .ok_or_else(|| "No valid session".to_string())?;

  payload
    .sub
    .parse::<i64>()
    .map_err(|e| format!("Invalid user ID in session: {e}"))
}
```

**Update `src/lib.rs`:**
```rust
pub mod utils;
```

**Tests for `session_context_sub_to_user_id`:**
```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionPayload;

  fn create_test_config() -> DpsAuthApiConfig {
    DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      sqlite_session_file_path: ":memory:".to_string(),
      session_secret: vec![0u8; 32],
      cookie_domain: ".test.com".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      sqlite_session_pool_size: 1,
      session_ttl_seconds: 3600,
    }
  }

  #[test]
  fn test_valid_session_returns_user_id() {
    let payload = SessionPayload {
      sub: "123".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(payload));
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert_eq!(result, Ok(123));
  }

  #[test]
  fn test_no_session_returns_error() {
    let session_context = SessionContext::new(None);
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No valid session"));
  }

  #[test]
  fn test_invalid_sub_returns_error() {
    let payload = SessionPayload {
      sub: "not_a_number".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(payload));
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid user ID"));
  }
}
```

### 5. Create utility function: `src/utils/user_to_session_sub.rs`

Create a second utility function that converts a `User` to the string `sub` value for session payloads.

**Create `src/utils/user_to_session_sub.rs`:**
```rust
use crate::models::User;
use crate::types::DpsAuthApiConfig;

/// Converts a user to the string `sub` value used in session payloads.
///
/// # Arguments
/// * `user` - The user to convert
/// * `_config` - The API config (reserved for future use)
///
/// # Returns
/// The string representation of the user ID for use as the session `sub`
pub fn user_to_session_sub(user: &User, _config: &DpsAuthApiConfig) -> String {
  user.id.to_string()
}
```

**Tests for `user_to_session_sub`:**
```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::User;

  fn create_test_config() -> DpsAuthApiConfig {
    DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      sqlite_session_file_path: ":memory:".to_string(),
      session_secret: vec![0u8; 32],
      cookie_domain: ".test.com".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      sqlite_session_pool_size: 1,
      session_ttl_seconds: 3600,
    }
  }

  fn create_test_user(id: i64) -> User {
    User {
      id,
      uuid: "test-uuid".to_string(),
      created_ts: 1000,
      updated_ts: 1000,
      name: "testuser".to_string(),
      role_id: 1,
      password_hash: "hash".to_string(),
      metadata_json: None,
    }
  }

  #[test]
  fn test_user_to_session_sub_returns_id_as_string() {
    let user = create_test_user(42);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, "42");
  }

  #[test]
  fn test_user_to_session_sub_zero_id() {
    let user = create_test_user(0);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, "0");
  }

  #[test]
  fn test_user_to_session_sub_large_id() {
    let user = create_test_user(i64::MAX);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, i64::MAX.to_string());
  }
}
```

### 6. Middleware: `src/middleware/session.rs`

**Remove the `user_id()` method from `SessionContext`:**
```rust
// DELETE this method:
// pub fn user_id(&self) -> Option<i64> {
//   self.payload.as_ref().map(|p| p.sub)
// }
```

### 7. Orchestrators and Services Using `session_payload.sub` Directly

Replace direct `session_payload.sub` access with the new utility function. Import at the top of the file:

```rust
use crate::utils::session_context_sub_to_user_id;
```

Then call the function directly:

```rust
// Before:
let user_id = session_payload.sub;

// After:
let user_id = session_context_sub_to_user_id(&session_context, &config)
  .map_err(|e| UserError::ValidationError(e))?;
```

Affected files (orchestrators/services):

- `src/orchestrators/auth/auth_change_password.rs` - line 24
- `src/services/auth/auth_get_current_user.rs` - line 23

For orchestrators, the error maps to `UserError::ValidationError`:
```rust
let user_id = session_context_sub_to_user_id(&session_context, &config)
  .map_err(|e| UserError::ValidationError(e))?;
```

For services, the error maps to `SessionError`:
```rust
let user_id = session_context_sub_to_user_id(&session_context, &config)
  .map_err(|e| SessionError::AuthenticationError(e))?;
```

Note: Orchestrators and services that don't already have access to `config` will need to accept it as a parameter. Check each orchestrator/service to see if `config` is already available or needs to be added to the function signature.

### 8. Session Creation: `src/services/session/create_session_for_user.rs`

Use the new `user_to_session_sub` utility function instead of directly converting `user.id` to string. Import at the top of the file:

```rust
use crate::utils::user_to_session_sub;
```

Then update the payload creation:

```rust
// Before:
let payload = DpsAuthSession::create_payload(user.id, None);

// After:
let payload = DpsAuthSession::create_payload(user_to_session_sub(&user, &config), None);
```

Note: This service will need to accept `config: &DpsAuthApiConfig` as a parameter if it doesn't already.

### 9. Test Files - SessionPayload Construction

All test files that construct `DpsAuthSessionPayload` need to convert `sub` to string:

```rust
// Before:
let session_payload = DpsAuthSessionPayload {
  sub: user.id,
  iat: 1000,
  exp: 2000,
};

// After:
let session_payload = DpsAuthSessionPayload {
  sub: user.id.to_string(),
  iat: 1000,
  exp: 2000,
};
```

Affected test files (all in `#[cfg(test)]` modules):

**Orchestrator tests:**
- `src/orchestrators/auth/auth_change_password.rs` (5 occurrences)
- `src/orchestrators/auth/auth_me.rs` (1 occurrence)
- `src/orchestrators/role/add_role.rs` (8 occurrences)
- `src/orchestrators/role/get_role.rs` (4 occurrences)
- `src/orchestrators/role/get_role_permissions.rs` (5 occurrences)
- `src/orchestrators/role/get_roles.rs` (2 occurrences)
- `src/orchestrators/role/remove_role.rs` (3 occurrences)
- `src/orchestrators/role/set_default_role.rs` (6 occurrences)
- `src/orchestrators/role/update_role.rs` (8 occurrences)
- `src/orchestrators/site/add_site.rs` (1 occurrence)
- `src/orchestrators/site/get_site.rs` (3 occurrences)
- `src/orchestrators/site/remove_site.rs` (2 occurrences)
- `src/orchestrators/site/update_site.rs` (2 occurrences)
- `src/orchestrators/user/delete_user.rs` (5 occurrences)
- `src/orchestrators/user/get_user.rs` (5 occurrences)
- `src/orchestrators/user/get_users.rs` (4 occurrences)
- `src/orchestrators/user/update_user.rs` (4 occurrences)

**GraphQL resolver tests:**
- `src/graphql/resolvers/add_role.rs` (6 occurrences)
- `src/graphql/resolvers/add_site.rs` (4 occurrences)
- `src/graphql/resolvers/auth_change_password.rs` (1 occurrence)
- `src/graphql/resolvers/auth_me.rs` (1 occurrence)
- `src/graphql/resolvers/delete_user.rs` (multiple occurrences)
- `src/graphql/resolvers/remove_role.rs` (multiple occurrences)
- `src/graphql/resolvers/remove_site.rs` (3 occurrences)
- `src/graphql/resolvers/role.rs` (multiple occurrences)
- `src/graphql/resolvers/role_permissions.rs` (multiple occurrences)
- `src/graphql/resolvers/roles.rs` (multiple occurrences)
- `src/graphql/resolvers/set_default_role.rs` (multiple occurrences)
- `src/graphql/resolvers/site.rs` (3 occurrences)
- `src/graphql/resolvers/update_role.rs` (multiple occurrences)
- `src/graphql/resolvers/update_site.rs` (4 occurrences)
- `src/graphql/resolvers/update_user.rs` (multiple occurrences)
- `src/graphql/resolvers/user.rs` (3 occurrences)
- `src/graphql/resolvers/users.rs` (multiple occurrences)

**Service tests:**
- `src/services/session/create_session.rs` (2 occurrences - `assert_eq!(payload.sub, user.id)` needs `assert_eq!(payload.sub, user.id.to_string())`)
- `src/services/session/create_session_for_user.rs` (3 occurrences - same pattern)

### 10. Service Tests - Assertion Updates

In session service tests, assertions comparing `payload.sub` to `user.id` need updating:

```rust
// Before:
assert_eq!(payload.sub, user.id);

// After:
assert_eq!(payload.sub, user.id.to_string());
```

Affected files:
- `src/services/session/create_session.rs` (lines 113, 187)
- `src/services/session/create_session_for_user.rs` (lines 47, 70, 71)

## Implementation Order

Following the AGENTS.md guidelines (queries first, then services, then orchestrators, then resolvers):

1. **Delete dead code** - `src/graphql/types/session_payload.rs` + update `src/graphql/types/mod.rs`
2. **Move DpsAuthApiConfig** - to `src/types/dps_auth_api_config.rs` + update all imports
3. **Create utility functions** - `src/utils/session_context_sub_to_user_id.rs` and `src/utils/user_to_session_sub.rs` + tests (implement and test in isolation first)
4. **Middleware** - remove `user_id()` from `SessionContext` in `src/middleware/session.rs`
5. **Services** - `src/services/session/create_session_for_user.rs` (use `user_to_session_sub`), `src/services/auth/auth_get_current_user.rs` (use `session_context_sub_to_user_id`)
6. **Orchestrators** - all orchestrator files (update to use utility functions)
7. **GraphQL resolvers** - all resolver files
8. **Tests** - all test modules (update payload construction and assertions)

## Files Affected (Complete List)

### Production Code (13+ files)
1. `src/graphql/types/session_payload.rs` (DELETE)
2. `src/graphql/types/mod.rs` (remove session_payload references)
3. `src/types/dps_auth_api_config.rs` (NEW - moved from dps_auth_api.rs)
4. `src/types/mod.rs` (add dps_auth_api_config module)
5. `src/dps_auth_api.rs` (remove DpsAuthApiConfig struct, add import)
6. `src/lib.rs` (add utils module, update exports)
7. `src/utils/mod.rs` (NEW)
8. `src/utils/session_context_sub_to_user_id.rs` (NEW)
9. `src/utils/user_to_session_sub.rs` (NEW)
10. `src/middleware/session.rs` (remove user_id method)
11. `src/services/session/create_session_for_user.rs`
12. `src/services/auth/auth_get_current_user.rs`
13. `src/orchestrators/auth/auth_change_password.rs`
14. `src/orchestrators/auth/auth_me.rs`
15. All files importing `DpsAuthApiConfig` (update import paths)

### Test Code (37 files with test modules)
All orchestrator and resolver test modules that construct `DpsAuthSessionPayload` or assert on `payload.sub`.

## Notes

- The database schema and queries remain unchanged - user IDs are still `i64` in SQLite
- The string parsing should be safe since the session library creates the string from a valid `i64`
- No new dependencies needed
- No migration files needed
