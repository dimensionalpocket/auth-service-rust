# Update dps-config to 0.10.2

## Overview

Update the `dps-config` dependency from `0.7.0` to `0.10.2` and integrate the two new session conversion function properties:

1. **`session_sub_to_user_id_fn`** - Converts a session `sub` string to an `i64` user ID. Returns `anyhow::Result<i64>`.
2. **`session_user_to_sub_fn`** - Extracts a `sub` string from a JSON record (`serde_json::Value`). Returns `anyhow::Result<String>`.

These functions replace hardcoded logic for:
- Extracting user ID from session `sub` (currently done in `session_context_sub_to_user_id`)
- Storing user ID in session `sub` (currently done in `create_session_for_user` and many test session payloads)

## Changelog Context

From [PR #27](https://github.com/dimensionalpocket/dps-config/pull/27):
- Added two function-typed properties to `DpsConfig` (Rust only)
- Default for `session_sub_to_user_id_fn`: parses string as `i64`, returns `anyhow::Result<i64>` (parse error on failure)
- Default for `session_user_to_sub_fn`: returns `id` property as string, `anyhow::Result<String>` (error on missing/invalid)
- Both have getters and setters for customization
- Uses `anyhow` crate for error handling

## Current Code Analysis

### Where user ID is saved to session `sub`:

1. **Production code**: `src/services/session/create_session_for_user.rs:9`
   - Uses `DpsAuthSession::create_payload(user.id.to_string(), None)`
   - The `sub` is set to `user.id.to_string()`

2. **Test code**: ~190+ places create `DpsAuthSessionPayload` or `ServiceSessionPayload` with:
   - `sub: user.id.to_string()` or `sub: admin_user_id.to_string()`
   - These are in orchestrator tests and resolver tests

### Where user ID is extracted from session `sub`:

1. **`src/utils/session_context_sub_to_user_id.rs`**:
   - Parses `payload.sub` as `i64`
   - Used by 15+ orchestrators and 1 service (`auth_get_current_user`)

2. **Integration tests**: `tests/integration_tests.rs` uses `session_context_sub_to_user_id`

## Implementation Plan

### Phase 1: Update Dependencies

**File: `Cargo.toml`**
- Change `dps-config` version from `tag = "0.7.0"` to `tag = "0.10.2"`
- Run `cargo build` to update `Cargo.lock`

### Phase 2: Update `DpsAuthApiConfig`

**File: `src/types/dps_auth_api_config.rs`**
- Add two new fields to store the session conversion functions extracted from `DpsConfig`:
  - `session_sub_to_user_id_fn: Arc<dyn Fn(&str) -> anyhow::Result<i64> + Send + Sync>`
  - `session_user_to_sub_fn: Arc<dyn Fn(&serde_json::Value) -> anyhow::Result<String> + Send + Sync>`
- **Important**: `DpsAuthApiConfig` derives `Clone` (used in GraphQL context at `dps_auth_api.rs:144`). Since `dps-config` uses `Box<dyn Fn...>` (not `Clone`), we wrap the extracted functions in `Arc` to maintain `Clone` compatibility.

**File: `src/dps_auth_api.rs`**
- In `DpsAuthApi::new()`, extract the functions from `dps_config` and wrap them in `Arc`:
```rust
let config = DpsAuthApiConfig {
  // ... existing fields ...
  session_sub_to_user_id_fn: Arc::from(dps_config.get_session_sub_to_user_id_fn()),
  session_user_to_sub_fn: Arc::from(dps_config.get_session_user_to_sub_fn()),
};
```
- `Arc::from(Box<T>)` is a zero-cost conversion that reuses the allocation.

### Phase 3: Update `session_context_sub_to_user_id`

**File: `src/utils/session_context_sub_to_user_id.rs`**
- Update the function to use `config.get_session_sub_to_user_id_fn()` instead of manual parsing
- Current logic: `payload.sub.parse::<i64>().map_err(...)`
- New logic: Call the config's function and propagate the `anyhow::Error`

**Updated implementation**:
```rust
pub fn session_context_sub_to_user_id(
  session_context: &SessionContext,
  config: &DpsAuthApiConfig,
) -> Result<i64, String> {
  let payload = session_context
    .payload
    .as_ref()
    .ok_or_else(|| "No valid session".to_string())?;

  let sub_to_user_id_fn = config.get_session_sub_to_user_id_fn();
  sub_to_user_id_fn(&payload.sub)
    .map_err(|e| e.to_string())
}
```

**Note**: Since `dps-config` now returns `anyhow::Result<i64>`, we can use `?` for error propagation and convert `anyhow::Error` to `String` via `.to_string()`. This preserves error context from the underlying parse failure.

**Note**: This requires `DpsAuthApiConfig` to have access to the function. Options:
- **Option A**: Store a reference to `DpsConfig` in `DpsAuthApiConfig`
- **Option B**: Store the function directly in `DpsAuthApiConfig` as a `Box<dyn Fn...>`
- **Option C**: Pass `DpsConfig` directly where needed instead of `DpsAuthApiConfig`

**Recommendation**: Option A - store a reference to `DpsConfig` or clone the function into `DpsAuthApiConfig`

### Phase 4: Update `create_session_for_user`

**File: `src/services/session/create_session_for_user.rs`**
- Currently: `DpsAuthSession::create_payload(user.id.to_string(), None)`
- The new `session_user_to_sub_fn` is for **extracting** sub from a JSON record, not for **creating** the sub value
- **Important**: The `session_user_to_sub_fn` is meant for the reverse direction (user object → sub string), typically used when storing session data
- The current `create_session_for_user` simply uses `user.id.to_string()` which is correct
- **No changes needed** to this file for the new functions

**Clarification**: The two new functions serve different purposes:
- `session_sub_to_user_id_fn`: sub string → user ID (used when reading session)
- `session_user_to_sub_fn`: user JSON record → sub string (used when session middleware decodes user info)

The `session_user_to_sub_fn` is likely used by the session middleware to extract the `sub` from a user record stored in the session payload, not by `create_session_for_user`.

### Phase 5: Update All Callers of `session_context_sub_to_user_id`

**Files affected** (all already pass config, just need internal function update):
- `src/orchestrators/auth/auth_change_password.rs`
- `src/orchestrators/role/get_role.rs`
- `src/orchestrators/role/get_role_permissions.rs`
- `src/orchestrators/role/get_roles.rs`
- `src/orchestrators/role/remove_role.rs`
- `src/orchestrators/role/set_default_role.rs`
- `src/orchestrators/role/update_role.rs`
- `src/orchestrators/role/add_role.rs`
- `src/orchestrators/site/add_site.rs`
- `src/orchestrators/site/update_site.rs`
- `src/orchestrators/site/remove_site.rs`
- `src/orchestrators/site/get_site.rs`
- `src/orchestrators/user/delete_user.rs`
- `src/orchestrators/user/get_user.rs`
- `src/orchestrators/user/get_users.rs`
- `src/orchestrators/user/update_user.rs`
- `src/services/auth/auth_get_current_user.rs`
- `tests/integration_tests.rs`

**No code changes needed** in these files - they already call `session_context_sub_to_user_id(&session_context, config)`. Only the utility function implementation changes.

### Phase 6: Update Tests

**File: `src/utils/session_context_sub_to_user_id.rs` (tests)**
- Update `create_test_config()` to include the two new `Arc` function fields (can use defaults from `DpsConfig::new()`)
- Test with default function (parses string as i64, returns anyhow::Result)
- Test error path: invalid sub string should produce an error message via `.map_err(|e| e.to_string())`

**File: `tests/integration_tests.rs`**
- Update tests that use `session_context_sub_to_user_id`
- Ensure config is properly initialized with `DpsConfig::new()`

**Note**: Any test that constructs `DpsAuthApiConfig` directly will need to add the two new `Arc` fields. Consider creating a helper function or extracting defaults from `DpsConfig::new()` to avoid repetition.

### Phase 7: Verify and Test

1. Run `cargo build` to ensure compilation succeeds
2. Run `cargo test --quiet` to ensure all tests pass
3. Run `cargo clippy --allow-dirty --fix && cargo fmt` to lint and format

## Files to Modify

1. **`Cargo.toml`** - Update `dps-config` version
2. **`src/types/dps_auth_api_config.rs`** - Add `Arc<dyn Fn...>` fields for session conversion functions
3. **`src/dps_auth_api.rs`** - Extract functions from `DpsConfig` and wrap in `Arc`
4. **`src/utils/session_context_sub_to_user_id.rs`** - Use config's `session_sub_to_user_id_fn` with anyhow error propagation
5. **`tests/integration_tests.rs`** - Update if needed for config changes

**Note**: `anyhow` will be available transitively through `dps-config`. No need to add it as a direct dependency unless we want to use `anyhow!` macros for custom errors.

## Order of Work

1. Update `Cargo.toml` to use `dps-config = { ..., tag = "0.10.2" }`
2. Run `cargo build` to see what breaks
3. Update `DpsAuthApiConfig` to store/access the session conversion functions
4. Update `session_context_sub_to_user_id` to use the config function
5. Fix any compilation errors
6. Run `cargo test --quiet`
7. Run linter

## Questions for Review

1. **Test session payloads**: The ~190 test session payloads with `sub: user.id.to_string()` don't need to change - they're creating test data, not using the conversion functions. The conversion functions are only used when **reading** session data, not when creating test payloads. Correct?
