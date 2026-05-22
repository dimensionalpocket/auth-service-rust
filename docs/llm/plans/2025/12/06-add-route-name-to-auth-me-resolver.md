# Plan: Add `roleName` to authMe Resolver

**Date:** 2025-12-06@15:06  
**Status:** Planning

## Overview

Add a `roleName` field to the `authMe` GraphQL resolver that returns the name of the current user's role. This will be implemented by leveraging the existing `GetUserByIdWithRoleQuery` which already joins with the `user_roles` table to fetch the role name in a single query.

## Current State Analysis

The current `authMe` resolver:
- Uses `AuthService::get_current_user()` which calls `UserService::get_user_by_id()`
- Only fetches user data without role name (just `role_id`)
- Returns `AuthMeResponse` struct with `role_id` but no role name

Existing infrastructure that can be reused:
- `GetUserByIdWithRoleQuery` in `src/queries/users/get_user_by_id_with_role.rs` - already joins with `user_roles` table
- `UserWithRole` model in `src/models/user.rs` - contains both user data and `role_name` field
- The query returns `r.name as role_name` from the JOIN operation

## Implementation Plan

### Phase 1: Update Service Layer

**File:** `src/services/auth_service.rs`

1. Modify `AuthMeResult` struct to include `role_name: String`
2. Update `AuthService::get_current_user()` method:
   - Replace `UserService::get_user_by_id()` call with `GetUserByIdWithRoleQuery::run()`
   - Update the return mapping to include `role_name` from the `UserWithRole` result

### Phase 2: Update GraphQL Resolver

**File:** `src/graphql/resolvers/auth_me.rs`

1. Add `role_name` field to `AuthMeResponse` struct with `#[graphql(name = "routeName")]`
2. Update the resolver mapping to include the new field from `auth_me_result.role_name`
3. Update GraphQL documentation and examples to include the new field

### Phase 3: Update Tests

**Files:** `src/services/auth_service.rs` and `src/graphql/resolvers/auth_me.rs`

1. Update existing tests to verify `role_name` is correctly populated
2. Add test assertions for the new `roleName` field in GraphQL responses
3. Ensure test data includes role names for verification

### Phase 4: Update Documentation

**File:** `README.md`

1. Locate the GraphQL queries table in the README
2. Update the `authMe` query entry to include the new `roleName` field in the response columns
3. Ensure the documentation reflects the complete set of fields returned by the query

## Detailed Implementation

### Phase 1: Service Layer Changes

```rust
// In AuthMeResult struct
pub struct AuthMeResult {
  // ... existing fields ...
  pub role_name: String,  // New field
}

// In get_current_user method
pub async fn get_current_user(
  pool: &SqlitePool,
  session_context: &SessionContext,
) -> Result<AuthMeResult, SessionError> {
  let session_payload = session_context
    .payload
    .as_ref()
    .ok_or_else(|| SessionError::AuthenticationError("No valid session".to_string()))?;

  // Use existing query that joins with user_roles
  let user_with_role = GetUserByIdWithRoleQuery::run(pool, session_payload.sub)
    .await
    .map_err(|e| SessionError::DatabaseError(e.to_string()))?
    .ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

  Ok(AuthMeResult {
    user_id: user_with_role.user.id,
    username: user_with_role.user.name,
    uuid: user_with_role.user.uuid,
    role_id: user_with_role.user.role_id,
    role_name: user_with_role.role_name,  // New field
    created_ts: user_with_role.user.created_ts,
    updated_ts: user_with_role.user.updated_ts,
    session_iat: session_payload.iat,
    session_exp: session_payload.exp,
  })
}
```

### Phase 2: GraphQL Resolver Changes

```rust
// In AuthMeResponse struct
#[derive(async_graphql::SimpleObject)]
pub struct AuthMeResponse {
  // ... existing fields ...
  /// The name of the user's role
  #[graphql(name = "roleName")]
  pub role_name: String,
}

// In resolver mapping
Ok(Some(AuthMeResponse {
  // ... existing fields ...
  role_name: auth_me_result.role_name,
  // ... rest of fields ...
}))
```

### Phase 3: Test Updates

Update test GraphQL query to include the new field:
```graphql
{ authMe { userId uuid username roleId roleName createdTs updatedTs sessionIat sessionExp } }
```

Add test assertions:
```rust
assert_eq!(data["authMe"]["roleName"], "user");  // or appropriate role name
```

## Benefits of This Approach

1. **No additional queries**: Uses existing JOIN query that's already optimized
2. **Reuses existing infrastructure**: Leverages `UserWithRole` model and `GetUserByIdWithRoleQuery`
3. **Minimal changes**: Only 3 files need modification
4. **Consistent with existing patterns**: Follows the same service → resolver pattern
5. **Backward compatible**: Adds new field without breaking existing functionality

## Files to Modify

1. `src/services/auth_service.rs` - Update service to use JOIN query and include role name
2. `src/graphql/resolvers/auth_me.rs` - Add routeName field to response type
3. Tests in both files to verify the new functionality
4. `README.md` - Update GraphQL queries table to include roleName field

### Phase 4: Update Documentation

**File:** `README.md`

1. Locate the GraphQL queries table in the README
2. Update the `authMe` query entry to include the new `roleName` field in the response columns
3. Ensure the documentation reflects the complete set of fields returned by the query

