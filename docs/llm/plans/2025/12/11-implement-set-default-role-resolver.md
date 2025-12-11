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

### Phase 2: Service Method

**File:** `src/services/role_service.rs`

**New Method:** `set_default_role(pool: &SqlitePool, role_id: i64) -> Result<Role, RoleError>`

**Implementation Details:**
1. Call `SetDefaultRoleQuery` to handle atomic default role setting
2. Handle role not found case from query result
3. Return the updated role or appropriate error

**Error Handling:**
- `RoleNotFound` - if role_id doesn't exist (from query result)
- `DatabaseError` - for SQL operation failures (from query)

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

### Phase 3: Orchestrator Method

**File:** `src/orchestrators/role_orchestrator.rs`

**New Method:** `set_default_role_with_permission_check(pool: &SqlitePool, session_context: SessionContext, role_id: i64) -> Result<Role, RoleError>`

**Implementation Details:**
Follow the existing orchestrator pattern:
1. Authentication: Check if user is authenticated
2. Authorization: Get user and verify `can_manage_roles` permission
3. Business Logic: Call `RoleService::set_default_role`

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

### Phase 4: GraphQL Resolver

**File:** `src/graphql/resolvers/set_default_role.rs`

**New Resolver:** `SetDefaultRoleResolver`

**Implementation Details:**
Follow existing resolver patterns:
1. Define input/output types inline
2. Implement resolver method calling orchestrator
3. Handle error conversion to GraphQL errors
4. Add comprehensive documentation
5. Include tests

**Code Structure:**
```rust
use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use crate::services::role_service::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for set default role response
#[derive(async_graphql::SimpleObject)]
pub struct SetDefaultRoleResponse {
  /// The role's database ID
  pub id: i64,
  /// The role's name
  pub name: String,
  /// Whether this is the default role for new users
  #[graphql(name = "isDefault")]
  pub is_default: bool,
  /// The role's permissions as a string array
  pub permissions: Vec<String>,
  /// Timestamp when the role was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the role was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Set default role mutation resolver
#[derive(Default, Debug)]
pub struct SetDefaultRoleResolver;

#[Object]
impl SetDefaultRoleResolver {
  /// Sets a role as the default role for new users.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_manage_roles" permission
  /// - Validates that the role exists
  /// - Atomically sets the role as default while unsetting any existing default
  /// - Automatically updates the updated_ts timestamp
  /// - Returns the updated role information
  ///
  /// # Arguments
  /// * `role_id` - ID of the role to set as default
  ///
  /// # Returns
  /// * `SetDefaultRoleResponse` - The updated role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns GraphQL error if role is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(role_id = %role_id))]
  #[graphql(name = "setDefaultRole")]
  async fn set_default_role(
    &self,
    ctx: &Context<'_>,
    role_id: i64,
  ) -> Result<SetDefaultRoleResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match RoleOrchestrator::set_default_role_with_permission_check(
      pool,
      session_context.clone(),
      role_id,
    )
    .await
    {
      Ok(role) => Ok(SetDefaultRoleResponse {
        id: role.id,
        name: role.name,
        is_default: role.is_default,
        permissions: role.permissions(),
        created_ts: role.created_ts,
        updated_ts: role.updated_ts,
      }),
      Err(RoleError::RoleNotFound(id)) => Err(async_graphql::Error::new(format!(
        "Role with ID {id} not found"
      ))),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to set default role: {}", err);
        Err(async_graphql::Error::new("Failed to set default role"))
      }
    }
  }
}
```

**Tests:** Include comprehensive tests following existing patterns:
- Success case with admin user
- Unauthenticated user
- User without permissions
- Non-existent role
- Verify atomic behavior (only one default role exists)

### Phase 5: Update README

**File:** `README.md`

**Update:** Add new mutation to the mutations table:

```markdown
| `setDefaultRole` | Set a role as the default role. Input: roleId (Int!). Returns: id (Int), name (String), isDefault (Boolean), permissions ([String]), createdTs (Int), updatedTs (Int). Requires can_manage_roles permission. |
```

**Location:** Insert after the `updateRole` row in the mutations table to maintain alphabetical order.

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