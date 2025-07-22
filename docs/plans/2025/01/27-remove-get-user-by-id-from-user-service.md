# Remove get_user_by_id Method from UserService

**Date**: 2025-01-27@15:41  
**Task**: Remove the `get_user_by_id` method from UserService as it's not used in multiple places, is too small, and tests should call the original query directly.

## Analysis

After analyzing the codebase, I found that the `get_user_by_id` method in `UserService` is indeed a thin wrapper around `GetUserByIdQuery::run()` and has limited usage:

### Current Usage
1. **UserService itself**: The method is defined in `src/services/user_service.rs` (lines 111-116)
2. **UserService tests**: Two test methods use it:
   - `test_get_user_by_id_delegates_to_query()` (line 379)
   - `test_get_user_by_id_not_found()` (line 391)
3. **No external usage**: The method is not used by any GraphQL mutations, queries, or other services

### Method Implementation
The current implementation is a simple delegation:

```rust
pub async fn get_user_by_id(
  pool: &SqlitePool,
  user_id: i64,
) -> Result<Option<User>, sqlx::Error> {
  GetUserByIdQuery::run(pool, user_id).await
}
```

This confirms the assessment that the method is too small and doesn't add meaningful business logic.

## Plan

### Phase 1: Remove the Method from UserService

1. **Remove method definition** from `src/services/user_service.rs`:
   - Delete lines 101-116 (method documentation and implementation)

2. **Remove import** from `src/services/user_service.rs`:
   - Remove `GetUserByIdQuery` from the import statement on line 2

3. **Update module exports** in `src/queries/users/mod.rs`:
   - Keep `GetUserByIdQuery` export as it will be used directly by tests

### Phase 2: Update Tests

1. **Update UserService tests** in `src/services/user_service.rs`:
   - Replace `test_get_user_by_id_delegates_to_query()` with a test that directly calls `GetUserByIdQuery::run()`
   - Replace `test_get_user_by_id_not_found()` with a test that directly calls `GetUserByIdQuery::run()`
   - Add necessary imports for `GetUserByIdQuery`

### Phase 3: Verification

1. **Run tests** to ensure all functionality still works:
   - Unit tests for `GetUserByIdQuery`
   - Updated UserService tests
   - Integration tests (should be unaffected)

2. **Verify no regressions** in the GraphQL API or other services

## Files to be Modified

### 1. `src/services/user_service.rs`
- **Remove**: `get_user_by_id` method (lines 101-116)
- **Remove**: `GetUserByIdQuery` from imports (line 2)
- **Update**: Test methods to call `GetUserByIdQuery::run()` directly
- **Add**: Import for `GetUserByIdQuery` in test module

### 2. No other files need modification
- `src/queries/users/get_user_by_id.rs` - Keep as-is (contains the actual implementation)
- `src/queries/users/mod.rs` - Keep exports as-is (tests will use them directly)

## Code Samples

### Updated UserService imports (after removal):
```rust
use crate::models::User;
use crate::queries::users::{
  CreateUserData, CreateUserQuery, GetUserByNameQuery,
};
use crate::services::{PasswordError, PasswordService};
use sqlx::SqlitePool;
use uuid::Uuid;
```

### Updated test imports:
```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::queries::users::GetUserByIdQuery; // Add this import
```

### Updated test method (example):
```rust
#[tokio::test]
async fn test_get_user_by_id_query_delegates_correctly() {
  let (pool, _temp_file) = create_test_database().await;

  // Insert default role first
  sqlx::query(
    "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
  )
  .execute(&pool)
  .await
  .unwrap();

  // Create a user first
  let user = UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  // Call GetUserByIdQuery directly instead of through UserService
  let retrieved_user = GetUserByIdQuery::run(&pool, user.id).await.unwrap();

  assert!(retrieved_user.is_some());
  let retrieved_user = retrieved_user.unwrap();
  assert_eq!(retrieved_user.id, user.id);
  assert_eq!(retrieved_user.name, user.name);
}
```

## Benefits

1. **Reduced code complexity**: Eliminates unnecessary wrapper method
2. **Better separation of concerns**: Tests directly test the query layer
3. **Cleaner API**: UserService focuses on business logic rather than simple delegations
4. **Consistency**: Aligns with the project pattern where simple queries can be called directly

## Risks

- **Low risk**: The method has minimal usage and no external dependencies
- **Easy rollback**: If needed, the method can be easily re-added

## Testing Strategy

1. Run existing unit tests for `GetUserByIdQuery` to ensure core functionality works
2. Run updated UserService tests to verify they work with direct query calls
3. Run full integration test suite to ensure no regressions
4. Verify GraphQL API still functions correctly (though it doesn't use this method)