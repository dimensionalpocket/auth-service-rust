# Upgrade DpsConfig from 0.3.0 to 0.4.0

**Date:** 2025-11-25@14:10  
**Version:** DpsConfig 0.3.0 → 0.4.0

## Breaking Changes Overview

DpsConfig 0.4.0 introduces breaking changes that affect the dps-auth-api:

1. **Removed:** `api_subdomain` property, its getter and setter
2. **Added:** `api_path` property, with getter and setter
3. **Updated:** `get_auth_api_url()` now includes the `api_path` in the URL

## Analysis of Current Usage

### Current Affected Code

1. **Integration Tests** (`tests/integration_tests.rs:28`):
   ```rust
   config.set_api_subdomain("api");
   ```

2. **Cookie Path Configuration** (`src/graphql/resolvers/auth_login.rs:48`, `src/graphql/resolvers/auth_logout.rs:30`):
   ```rust
   // Currently hardcoded to Path=/ and SameSite=Strict
   "{}={}; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Max-Age={}",
   ```

3. **Cookie Domain Calculation** (`src/dps_auth_api.rs:99`):
   ```rust
   cookie_domain: format!(".{}", dps_config.get_api_domain()),
   ```
   - This will change from `.api.dps.localhost` to `.dps.localhost` with the new DpsConfig
   - Need to update to use `get_domain()` instead of `get_api_domain()`

### Additional Areas Requiring Updates

1. **Documentation** (`README.md:118`):
   - References to `api_subdomain` in cookie domain explanation
   - Need to update to reflect `api_path` usage

2. **Test Expectations** (`tests/integration_tests.rs:239`):
   - Test expects `Domain=.api.dps.localhost` 
   - Need to update to expect `Domain=.dps.localhost` with new DpsConfig

## Implementation Plan

### Phase 1: Update Dependencies

1. **Update Cargo.toml**:
   ```toml
   dps-config = { git = "https://github.com/dimensionalpocket/dps-config-rs", tag = "0.4.0" }
   ```

### Phase 2: Update Integration Tests

1. **Replace `api_subdomain` with `api_path`** in `tests/integration_tests.rs:28`:
   ```rust
   // OLD
   config.set_api_subdomain("api");
   
   // NEW  
   config.set_api_path("api");
   ```

### Phase 3: Update Cookie Path Configuration

1. **Add `api_path` to DpsAuthApiConfig** in `src/dps_auth_api.rs:14-22`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct DpsAuthApiConfig {
     pub port: u16,
     pub sqlite_main_file_path: String,
     pub session_secret: Vec<u8>,
     pub cookie_domain: String,
     pub api_path: String,  // NEW: Add api_path field
     pub insecure_cookie: bool,
     pub development_mode: bool,
     pub sqlite_main_pool_size: u16,
   }
   ```

2. **Update DpsAuthApiConfig construction** in `src/dps_auth_api.rs:95-103`:
   ```rust
   let config = DpsAuthApiConfig {
     port: dps_config.get_auth_api_port().unwrap_or(3000),
     sqlite_main_file_path: dps_config.get_auth_api_sqlite_main_file_path(),
     session_secret,
     cookie_domain: format!(".{}", dps_config.get_domain()),  // UPDATED: Use get_domain() instead of get_api_domain()
     api_path: format!("/{}", dps_config.get_api_path()),  // NEW: Add leading slash
     insecure_cookie: dps_config.get_auth_api_insecure_cookie(),
     development_mode: dps_config.get_development_mode(),
     sqlite_main_pool_size: dps_config.get_auth_api_sqlite_main_pool_size(),
   };
   ```

### Phase 4: Update Cookie Generation

1. **Update auth login resolver** in `src/graphql/resolvers/auth_login.rs:47-54`:
   ```rust
   let cookie_value = format!(
     "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
     SESSION_COOKIE_NAME,
     auth_result.session_token,
     cookie_domain,
     config.api_path,  // NEW: Use api_path instead of hardcoded "/"
     if insecure_cookie { "" } else { "; Secure" },
     3 * 24 * 60 * 60
   );
   ```

2. **Update auth logout resolver** in `src/graphql/resolvers/auth_logout.rs:29-34`:
   ```rust
   let cookie_value = format!(
     "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
     SESSION_COOKIE_NAME,
     cookie_domain,
     config.api_path,  // NEW: Use api_path instead of hardcoded "/"
     if insecure_cookie { "" } else { "; Secure" }
   );
   ```

### Phase 5: Update Test Configurations

1. **Update test configs** in `src/graphql/resolvers/auth_login.rs:117-125` and `src/graphql/resolvers/auth_login.rs:172-180`:
   ```rust
   let test_config = DpsAuthApiConfig {
     port: 0,
     sqlite_main_file_path: "test.db".to_string(),
     session_secret: TEST_SECRET.to_vec(),
     cookie_domain: ".dps.localhost".to_string(),  // UPDATED: Changed from .api.dps.localhost
     api_path: "/api".to_string(),  // NEW: Add api_path
     insecure_cookie: false,
     development_mode: true,
     sqlite_main_pool_size: 1,
   };
   ```

2. **Update test config** in `src/graphql/resolvers/auth_login.rs:212-220`:
   ```rust
   let test_config = DpsAuthApiConfig {
     port: 0,
     sqlite_main_file_path: "test.db".to_string(),
     session_secret: TEST_SECRET.to_vec(),
     cookie_domain: ".dps.localhost".to_string(),  // UPDATED: Changed from .api.dps.localhost
     api_path: "/api".to_string(),  // NEW: Add api_path
     insecure_cookie: false,
     development_mode: true,
     sqlite_main_pool_size: 1,
   };
   ```

### Phase 6: Update Documentation

1. **Update README.md** line 118:
   ```markdown
   <!-- OLD -->
   The cookie domain is automatically derived as `.{api_subdomain}.{domain}` (e.g., ".api.dps.localhost").
   
   <!-- NEW -->
   The cookie domain is automatically derived as `.{domain}` (e.g., ".dps.localhost").
   The cookie path is set to the `api_path` configuration with a leading slash (e.g., "/api").
   ```

### Phase 7: Verify URL Generation

1. **Confirm `get_auth_api_url()` behavior**:
   - The new DpsConfig version should return URLs like `https://auth.dps.localhost/api`
   - No changes needed in our code since we use this getter directly
   - Verify integration tests still pass with expected cookie domains

## Implementation Notes

### Key Changes

1. **Cookie Path**: Now uses `api_path` from DpsConfig instead of hardcoded "/"
2. **Cookie Domain**: Now uses `get_domain()` instead of `get_api_domain()`, changing from ".api.dps.localhost" to ".dps.localhost"
3. **Cookie SameSite**: Changed from `Strict` to `Lax` for better cross-site navigation behavior
4. **API Path**: Added to DpsAuthApiConfig to make it available to resolvers
5. **Leading Slash**: DpsConfig's `api_path` doesn't include leading slash, so we add it during config construction

### Backward Compatibility Considerations

- This is a breaking change as expected for pre-1.0.0 version
- Cookie paths will change from "/" to "/api" by default
- Cookie domains will change from ".api.dps.localhost" to ".dps.localhost" by default
- Cookie SameSite will change from "Strict" to "Lax" for better cross-site behavior
- Existing deployments will need to update environment variables:
  - Remove: `DPS_API_SUBDOMAIN=api`  
  - Add: `DPS_API_PATH=api`

### Testing Requirements

1. Verify integration tests pass with new cookie path
2. Confirm cookie domain change from ".api.dps.localhost" to ".dps.localhost" works correctly  
3. Verify SameSite change from "Strict" to "Lax" works as expected
4. Test that `get_auth_api_url()` returns expected URLs
5. Run password logging tests to ensure no regressions

## Files to Modify

1. `Cargo.toml` - Update dps-config version
2. `src/dps_auth_api.rs` - Add api_path to DpsAuthApiConfig
3. `src/graphql/resolvers/auth_login.rs` - Update cookie path and test configs
4. `src/graphql/resolvers/auth_logout.rs` - Update cookie path
5. `tests/integration_tests.rs` - Replace api_subdomain with api_path
6. `README.md` - Update documentation

