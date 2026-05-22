# Plan: Replace Hardcoded Cookie TTL with DpsConfig Method

## Overview
Replace the hardcoded cookie time-to-live (TTL) value in `auth_login.rs` with a configurable value from `DpsConfig.get_auth_api_session_ttl_seconds()` method.

## Current Issue
In `src/graphql/resolvers/auth_login.rs:54`, the cookie Max-Age is hardcoded as `3 * 24 * 60 * 60` (3 days). This should be configurable via DpsConfig.

## Implementation Details

### 1. Update DpsAuthApiConfig Structure
**File**: `src/dps_auth_api.rs`
- Add `session_ttl_seconds: u32` field to `DpsAuthApiConfig` struct
- Update the config resolution in `DpsAuthApi::new()` method to include:
  ```rust
  session_ttl_seconds: dps_config.get_auth_api_session_ttl_seconds(),
  ```

### 2. Update AuthLoginResolver
**File**: `src/graphql/resolvers/auth_login.rs`
- Replace hardcoded `3 * 24 * 60 * 60` with `config.session_ttl_seconds`
- Update the cookie format string to use the configurable value:
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
  ```

### 3. Update Test Configurations
**File**: `src/graphql/resolvers/auth_login.rs`
- Update test configurations in the test modules to include the new `session_ttl_seconds` field
- Set a reasonable test value (e.g., `3600` for 1 hour)

## Files to Modify
1. `src/dps_auth_api.rs` - Add session_ttl_seconds to DpsAuthApiConfig and config resolution
2. `src/graphql/resolvers/auth_login.rs` - Replace hardcoded TTL with config value and update tests

## Notes
- The `dps_config::DpsConfig.get_auth_api_session_ttl_seconds()` method already handles defaults, so no additional default handling is needed
- This change makes session expiration configurable while maintaining backward compatibility through the existing DpsConfig defaults
- All existing tests should continue to pass with the updated configuration structure