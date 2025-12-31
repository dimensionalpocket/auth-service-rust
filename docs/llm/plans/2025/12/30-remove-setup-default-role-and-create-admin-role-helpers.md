# Plan: Remove setup_default_role and create_admin_role One-Liner Helper Methods

## Summary

Remove duplicate `setup_default_role` and `create_admin_role` methods from test files that are just one-liners wrapping test helpers. Use the test helpers directly instead.

## Problem

Multiple test files define their own `setup_default_role` and `create_admin_role` methods that simply call test helper functions:
- `setup_default_role()` → calls `create_test_role_model_with_pool(pool, "user", &["can_view_user_self"], true)`
- `create_admin_role()` → calls `create_test_role_with_pool(pool, "admin", &["is_admin"])` or similar

These one-liner wrapper methods add no value and increase maintenance burden when the test helpers need to be updated.

## Implementation

### Phase 1: src/services/user_service.rs

**Lines to modify:** 776, 815-816, 901, 943-944, 983-984, 1033, 1096, 1134, 1172, 1211, 1251, 1302, 1341, 1381, 1412-1418

**Actions:**
1. Remove lines 1412-1418 (both `setup_default_role` and `create_admin_role` methods)
2. Replace all `setup_default_role(&pool).await;` with:
   ```rust
   create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
   ```
3. Replace all `let admin_role_id = create_admin_role(&pool).await;` with:
   ```rust
   let admin_role_id = create_test_role_with_pool(&pool, "admin", &["is_admin"]).await;
   ```

### Phase 2: src/services/session_service.rs

**Lines to modify:** 208-210, 217, 281, 302, 322, 342

**Actions:**
1. Remove lines 208-210 (`setup_default_role` method)
2. Replace all `setup_default_role(&pool).await;` with:
   ```rust
   create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
   ```

### Phase 3: src/queries/users/update_user.rs

**Lines to modify:** 110-117, 124, 164, 205, 247, 289, 330, 392, 432, 476

**Actions:**
1. Remove lines 110-117 (`setup_default_role` and `setup_admin_role_for_update` methods)
2. Replace all `setup_default_role(&pool).await;` with:
   ```rust
   create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
   ```
3. Replace all `let admin_role_id = setup_admin_role_for_update(&pool).await;` with:
   ```rust
   let admin_role = create_test_role_model_with_pool(&pool, "admin", &["is_admin"], false).await;
   let admin_role_id = admin_role.id;
   ```

### Phase 4: src/queries/users/update_user_password.rs

**Lines to modify:** 66-68, 75, 127

**Actions:**
1. Remove lines 66-68 (`setup_default_role` method)
2. Replace all `setup_default_role(&pool).await;` with:
   ```rust
   create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
   ```

### Phase 5: src/orchestrators/user_orchestrator.rs

**Lines to modify:** 752, 926, 969, 1015, 1061, 1104-1110

**Actions:**
1. Remove lines 1104-1110 (both `create_admin_role` and `create_user_role` methods)
2. Replace all `let admin_role_id = create_admin_role(&pool).await;` with:
   ```rust
   let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
   ```
3. Replace all `let user_role_id = create_user_role(&pool).await;` with:
   ```rust
   let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
   ```

## Verification

After implementation:
1. Run `cargo test --quiet` to verify all tests still pass
2. Verify `grep -r "setup_default_role\|create_admin_role\|setup_admin_role_for_update\|create_user_role" src/` returns no results in test code (only matches should be in docs)
