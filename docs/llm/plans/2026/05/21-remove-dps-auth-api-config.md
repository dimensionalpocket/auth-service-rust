# Remove DpsAuthApiConfig and use DpsConfig directly

## Overview

Remove the `DpsAuthApiConfig` struct and replace it with `Arc<dps_config::DpsConfig>` throughout the codebase. This eliminates the redundant configuration layer and allows direct access to `DpsConfig`'s getters.

## Current State

`DpsAuthApiConfig` exists in `src/types/dps_auth_api_config.rs` with these fields:
- `port: u16`
- `sqlite_main_file_path: String`
- `sqlite_session_file_path: String`
- `session_secret: Vec<u8>`
- `cookie_domain: String` (derived: `.{domain}`)
- `api_path: String` (derived: `/{api_path}`)
- `insecure_cookie: bool`
- `development_mode: bool`
- `sqlite_main_pool_size: u16`
- `sqlite_session_pool_size: u16`
- `session_ttl_seconds: u32`

## Derived Values Mapping

| `DpsAuthApiConfig` field | `DpsConfig` equivalent |
|--------------------------|------------------------|
| `port` | `config.get_auth_api_port().unwrap_or(3000)` |
| `sqlite_main_file_path` | `config.get_auth_api_sqlite_main_file_path()` |
| `sqlite_session_file_path` | `config.get_auth_api_sqlite_session_file_path()` |
| `session_secret` | `config.get_auth_api_session_secret_bytes().unwrap_or_default()` |
| `cookie_domain` | `format!(".{}", config.get_domain())` |
| `api_path` | `format!("/{}", config.get_api_path())` |
| `insecure_cookie` | `config.get_auth_api_insecure_cookie()` |
| `development_mode` | `config.get_development_mode()` |
| `sqlite_main_pool_size` | `config.get_auth_api_sqlite_main_pool_size()` |
| `sqlite_session_pool_size` | `config.get_auth_api_sqlite_session_pool_size()` |
| `session_ttl_seconds` | `config.get_auth_api_session_ttl_seconds()` |

## Implementation Plan

### Phase 1: Update `DpsAuthApi` struct

**File: `src/dps_auth_api.rs`**
- Change `DpsAuthApi` to store `Arc<DpsConfig>` instead of `Arc<DpsAuthApiConfig>`:
```rust
pub struct DpsAuthApi {
  pub(crate) config: Arc<dps_config::DpsConfig>,
}
```
- Update `DpsAuthApi::new()` to:
  - Validate session secret (required, 32 bytes)
  - Store `Arc::new(dps_config)` directly
- Add validation method:
```rust
fn validate_config(config: &DpsConfig) -> Result<(), DpsAuthApiError> {
  let secret = config.get_auth_api_session_secret_bytes()
    .ok_or(DpsAuthApiError::MissingRequiredConfig { field: "auth_api_session_secret".to_string() })?;
  if secret.len() != 32 {
    return Err(DpsAuthApiError::InvalidSecretLength { actual: secret.len(), expected: 32 });
  }
  Ok(())
}
```
- Update `create_app()` to inject `Arc<DpsConfig>` into GraphQL context:
```rust
.data(self.config.clone()) // Arc<DpsConfig>
```
- Update router methods to use `DpsConfig` getters:
  - `self.config.get_api_path()` → format with `/` prefix inline
  - `self.config.get_auth_api_session_secret_bytes().unwrap()` for session middleware

### Phase 2: Delete `DpsAuthApiConfig`

**File: `src/types/dps_auth_api_config.rs`**
- Delete the file entirely

**File: `src/types/mod.rs`**
- Remove `mod dps_auth_api_config` and `pub use` export

### Phase 3: Update Orchestrators

**Files affected** (change `config: &DpsAuthApiConfig` to `config: &DpsConfig`):
- `src/orchestrators/auth/auth_change_password.rs`
- `src/orchestrators/auth/auth_login.rs`
- `src/orchestrators/auth/auth_me.rs`
- `src/orchestrators/auth/auth_register.rs`
- `src/orchestrators/role/add_role.rs`
- `src/orchestrators/role/get_role.rs`
- `src/orchestrators/role/get_role_permissions.rs`
- `src/orchestrators/role/get_roles.rs`
- `src/orchestrators/role/remove_role.rs`
- `src/orchestrators/role/set_default_role.rs`
- `src/orchestrators/role/update_role.rs`
- `src/orchestrators/site/add_site.rs`
- `src/orchestrators/site/get_site.rs`
- `src/orchestrators/site/remove_site.rs`
- `src/orchestrators/site/update_site.rs`
- `src/orchestrators/user/delete_user.rs`
- `src/orchestrators/user/get_user.rs`
- `src/orchestrators/user/get_users.rs`
- `src/orchestrators/user/update_user.rs`

**Changes per file**:
- Import: `use dps_config::DpsConfig;` instead of `use crate::types::DpsAuthApiConfig;`
- Parameter: `config: &DpsConfig`
- Access: `config.get_auth_api_session_secret_bytes()` instead of `config.session_secret`

### Phase 4: Update Resolvers

**Files affected** (change `ctx.data::<DpsAuthApiConfig>()` to `ctx.data::<Arc<DpsConfig>>()`):
- `src/graphql/resolvers/add_role.rs`
- `src/graphql/resolvers/add_site.rs`
- `src/graphql/resolvers/auth_change_password.rs`
- `src/graphql/resolvers/auth_login.rs`
- `src/graphql/resolvers/auth_me.rs`
- `src/graphql/resolvers/auth_register.rs`
- `src/graphql/resolvers/delete_user.rs`
- `src/graphql/resolvers/remove_role.rs`
- `src/graphql/resolvers/remove_site.rs`
- `src/graphql/resolvers/role.rs`
- `src/graphql/resolvers/role_permissions.rs`
- `src/graphql/resolvers/roles.rs`
- `src/graphql/resolvers/set_default_role.rs`
- `src/graphql/resolvers/site.rs`
- `src/graphql/resolvers/sites.rs`
- `src/graphql/resolvers/update_role.rs`
- `src/graphql/resolvers/update_site.rs`
- `src/graphql/resolvers/update_user.rs`
- `src/graphql/resolvers/user.rs`
- `src/graphql/resolvers/users.rs`

**Changes per file**:
- Import: `use dps_config::DpsConfig;` instead of `use crate::types::DpsAuthApiConfig;`
- Extract: `let config = ctx.data::<Arc<DpsConfig>>()?;`
- Pass to orchestrator: `&config` (deref from Arc)

### Phase 5: Update Services

**File: `src/services/auth/auth_get_current_user.rs`**
- Change `config: &DpsAuthApiConfig` to `config: &DpsConfig`
- Update import

**File: `src/services/cookie/generate_session_cookie.rs`**
- Update to compute derived values inline:
  - `config.get_domain()` → `format!(".{}", config.get_domain())`
  - `config.get_api_path()` → `format!("/{}", config.get_api_path())`
  - `config.get_auth_api_insecure_cookie()` instead of `config.insecure_cookie`
  - `config.get_auth_api_session_ttl_seconds()` instead of `config.session_ttl_seconds`

**File: `src/services/cookie/generate_logout_cookie.rs`**
- Same as above

**File: `src/services/session/create_session_for_user.rs`**
- No changes needed (doesn't use config)

### Phase 6: Update `session_context_sub_to_user_id`

**File: `src/utils/session_context_sub_to_user_id.rs`**
- Change parameter from `config: &DpsAuthApiConfig` to `config: &DpsConfig`
- Update import

**File: `src/utils/user_to_session_sub.rs`**
- Change parameter from `config: &DpsAuthApiConfig` to `config: &DpsConfig`
- Update import

### Phase 7: Update Test Utilities

**File: `src/test_utils/mod.rs`**
- Remove `create_test_config()` that returns `DpsAuthApiConfig`
- Add `create_test_dps_config()` that returns `DpsConfig`:
```rust
pub fn create_test_dps_config() -> DpsConfig {
  let mut config = DpsConfig::new();
  config.set_auth_api_session_secret("a".repeat(32).as_str());
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config
}
```
- Update `create_test_query_schema` and `create_test_mutation_schema` to accept `Option<DpsConfig>` instead of `Option<DpsAuthApiConfig>`

### Phase 8: Update Integration Tests

**File: `tests/integration_tests.rs`**
- Remove `create_test_config()` returning `DpsAuthApiConfig`
- Use `create_test_dps_config()` from test_utils
- Update `session_context_sub_to_user_id` tests to use `DpsConfig`

### Phase 9: Update Resolver Tests

**Files affected** (tests that construct `DpsAuthApiConfig` directly):
- `src/graphql/resolvers/auth_login.rs`
- `src/graphql/resolvers/auth_register.rs`

**Changes**:
- Replace `DpsAuthApiConfig { ... }` with `create_test_dps_config()` helper

### Phase 10: Verify and Test

1. Run `cargo build` to ensure compilation succeeds
2. Run `cargo test --quiet` to ensure all tests pass
3. Run `cargo clippy --allow-dirty --fix && cargo fmt` to lint and format

## Files to Modify

1. **`src/dps_auth_api.rs`** - Store `Arc<DpsConfig>`, update validation
2. **`src/types/dps_auth_api_config.rs`** - DELETE
3. **`src/types/mod.rs`** - Remove export
4. **18 orchestrator files** - Change parameter type
5. **20 resolver files** - Change context extraction
6. **3 service files** - Update config usage
7. **2 utility files** - Update parameter type
8. **`src/test_utils/mod.rs`** - New helper function
9. **`tests/integration_tests.rs`** - Update config usage
10. **Resolver test modules** - Update test config creation

## Order of Work

1. Update `DpsAuthApi` to use `Arc<DpsConfig>`
2. Delete `DpsAuthApiConfig` and remove exports
3. Update orchestrators (parameter types)
4. Update resolvers (context extraction)
5. Update services (derived values)
6. Update utilities
7. Update test utilities
8. Update integration tests
9. Update resolver tests
10. Run `cargo build`
11. Run `cargo test --quiet`
12. Run linter

## Q&A

1. **Validation in `DpsAuthApi::new()`**: Keep validation as-is (lines 74-88), no helper method needed. Validation happens before struct creation anyway.
2. **Context injection**: Use `ctx.data::<Arc<DpsConfig>>()` - Arc is needed for multithreading.
3. **Orchestrators that don't use config**: Keep config parameter for consistency; they may use it in the future.
4. **Cookie services**: Compute derived values inline (`format!(".{}", config.get_domain())`, `format!("/{}", config.get_api_path())`).
5. **Test config setter**: Use `Some()` wrapper - `config.set_auth_api_session_secret(Some("a".repeat(32).as_str()))`.
6. **Session conversion functions** (`session_context_sub_to_user_id`, `user_to_session_sub`): Leave unchanged - keep `&DpsAuthApiConfig` parameter.

## Notes

- `DpsConfig` is not `Clone` due to `Box<dyn Fn...>`, but `Arc<DpsConfig>` is `Clone`
- All GraphQL context injections will use `Arc<DpsConfig>`
- Derived values (`cookie_domain`, `api_path` with `/`) computed inline where needed
- No changes to session conversion functions - they remain in `DpsConfig` but are not integrated yet
