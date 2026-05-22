# Implement setDefaultRole Resolver

**Date:** 2025-12-11@17:17  
**Resolver Name:** `setDefaultRole`  
**Permission Required:** `can_manage_roles`

## Overview

This plan implements a new GraphQL resolver `setDefaultRole` that sets a role as the default role in the system. The resolver will:

1. Take a `role_id` as input
2. Update the specified role's `is_default` field to `true`
3. Ensure no other role has `is_default` set to `true` (atomic operation)
4. Return the updated role information

## Implementation Phases

### Phase 1: Queries ✅ COMPLETED

**New Query File:** `src/queries/roles/set_default_role.rs` ✅

**New Query:** `SetDefaultRoleQuery` ✅

**Implementation Details:**
✅ Created a single atomic query that:
1. First verifies the role exists
2. Sets all roles' `is_default` to `false`
3. Sets target role's `is_default` to `true` 
4. Updates the `updated_ts` timestamp
5. Returns the updated role

**Module Update:** ✅ Added to `src/queries/roles/mod.rs`:
```rust
pub mod set_default_role;
pub use set_default_role::SetDefaultRoleQuery;
```

**Tests:** ✅ All comprehensive tests implemented and passing:
- Test successful default role setting
- Test atomic behavior (only one default exists)
- Test role not found scenario
- Test timestamp update verification

### Phase 2: Service Method ✅ COMPLETED

**File:** `src/services/role_service.rs`

**New Method:** `set_default_role(pool: &SqlitePool, role_id: i64) -> Result<Role, RoleError>` ✅

**Implementation Details:**
✅ Call `SetDefaultRoleQuery` to handle atomic default role setting
✅ Handle role not found case from query result
✅ Return the updated role or appropriate error

**Error Handling:**
✅ `RoleNotFound` - if role_id doesn't exist (from query result)
✅ `DatabaseError` - for SQL operation failures (from query)

**Code Structure:**
```rust
pub async fn set_default_role(
  pool: &SqlitePool,
  role_id: i64,
) -> Result<Role, RoleError> {
  match SetDefaultRoleQuery::run(pool, role_id).await {
    Ok(Some(role)) => Ok(role),
    Ok(None) => Err(RoleError::RoleNotFound(role_id)),
    Err(err) => Err(RoleError::DatabaseError(err)),
  }
}
```

**Tests:** ✅ All comprehensive tests implemented and passing:
- Test successful default role setting
- Test atomic behavior (only one default exists)
- Test role not found scenario
- Test timestamp update verification

### Phase 3: Orchestrator Method ✅ COMPLETED

**File:** `src/orchestrators/role_orchestrator.rs` ✅

**New Method:** `set_default_role_with_permission_check(pool: &SqlitePool, session_context: SessionContext, role_id: i64) -> Result<Role, RoleError>` ✅

**Implementation Details:**
✅ Follow the existing orchestrator pattern:
✅ Authentication: Check if user is authenticated
✅ Authorization: Get user and verify `can_manage_roles` permission
✅ Business Logic: Call `RoleService::set_default_role`

**Code Structure:**
```rust
/// Set a role as default with permission check
///
/// Validates that user has `can_manage_roles` permission
/// before setting the specified role as the default.
pub async fn set_default_role_with_permission_check(
  pool: &SqlitePool,
  session_context: SessionContext,
  role_id: i64,
) -> Result<Role, RoleError> {
  // Authentication: Check if user is authenticated
  let user_id = session_context
    .user_id()
    .ok_or(RoleError::AuthenticationError(
      "Authentication required".to_string(),
    ))?;

  // Authorization: Get user and check permissions
  let user = GetUserByIdQuery::run(pool, user_id)
    .await?
    .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

  let can_manage_roles =
    RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

  if !can_manage_roles {
    return Err(RoleError::AuthorizationError("Forbidden".to_string()));
  }

  // Business logic: Set default role
  RoleService::set_default_role(pool, role_id).await
}
```

**Tests:** ✅ All comprehensive tests implemented and passing:
- Success case with admin user
- Unauthenticated user
- User without permissions
- Non-existent role
- Verify atomic behavior (only one default role exists)
- Verify timestamp update

### Phase 4: GraphQL Resolver ✅ COMPLETED

**File:** `src/graphql/resolvers/set_default_role.rs` ✅

**New Resolver:** `SetDefaultRoleResolver` ✅

**Implementation Details:**
✅ Followed existing resolver patterns:
✅ Defined input/output types inline
✅ Implemented resolver method calling orchestrator
✅ Handled error conversion to GraphQL errors
✅ Added comprehensive documentation
✅ Included comprehensive tests

**Code Structure:** ✅ Implemented exactly as planned with proper error handling and logging

**Tests:** ✅ All comprehensive tests implemented and passing:
- Success case with admin user
- Unauthenticated user
- User without permissions
- Non-existent role
- Verify atomic behavior (only one default role exists)
- Used proper test utilities (no raw SQL queries)

### Phase 5: Update README ✅ COMPLETED

**File:** `README.md` ✅

**Update:** ✅ Added new mutation to the mutations table:

```markdown
| `setDefaultRole` | Set a role as the default role. Input: roleId (Int!). Returns: id (Int), name (String), isDefault (Boolean), permissions ([String]), createdTs (Int), updatedTs (Int). Requires can_manage_roles permission. |
```

**Location:** ✅ Inserted after the `updateRole` row in the mutations table to maintain alphabetical order.

## Additional Implementation Details

### Schema Registration

**File:** `src/graphql/schema.rs`

**Update:** Add new resolver to the `Mutation` struct:

```rust
#[derive(Default)]
pub struct Mutation {
  // ... existing fields
  pub set_default_role: SetDefaultRoleResolver,
}
```

### Module Registration

**File:** `src/graphql/resolvers/mod.rs`

**Update:** Add new module:

```rust
pub mod set_default_role;
```

### Module Registration

**File:** `src/graphql/resolvers/mod.rs`

**Update:** Add the new module:

```rust
pub mod set_default_role;
```

### Utility Function

**File:** `src/utils/mod.rs` (if not exists, create it)

**Add:** Timestamp utility function if not already available:

```rust
pub fn get_current_timestamp() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64
}
```

## Testing Strategy

### Unit Tests
- Service method: Test atomic default role setting
- Service method: Test role not found error
- Service method: Test database error handling

### Integration Tests  
- Orchestrator: Test permission check success/failure
- Orchestrator: Test authentication scenarios
- Resolver: Test full GraphQL mutation flow
- Resolver: Test error conversion to GraphQL errors

### Database Tests
- Verify atomic operation (only one default role exists)
- Test transaction rollback on errors
- Verify timestamp updates

## Security Considerations

1. **Permission Check:** Strict `can_manage_roles` requirement
2. **Input Validation:** Role ID validation through database query
3. **Atomic Operation:** Use database transaction to prevent multiple defaults
4. **Audit Trail:** Updated timestamp tracks when default was changed
5. **Error Handling:** No sensitive information leaked in error messages

## Performance Considerations

1. **Transaction Scope:** Minimal transaction time
2. **Index Usage:** Ensure `id` and `is_default` columns are indexed
3. **Query Efficiency:** Single transaction for both updates
4. **Caching:** No caching needed for this operation

## Rollback Plan

If issues arise:
1. Remove resolver from schema
2. Remove orchestrator method
3. Remove service method
4. Revert README changes
5. All changes are isolated and don't affect existing functionality

## Dependencies

No new external dependencies required. Uses existing:
- `sqlx` for database operations
- `async-graphql` for GraphQL
- `tracing` for logging
- Existing service and orchestrator patterns