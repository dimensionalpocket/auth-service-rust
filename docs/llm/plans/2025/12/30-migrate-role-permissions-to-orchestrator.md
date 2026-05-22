# Plan: Migrate role_permissions Resolver to Orchestrator Pattern

## Date
2025-12-30@16:30

## Problem
The `role_permissions.rs` resolver is the only resolver not using the orchestrator pattern. It contains TODO comment acknowledging this should be migrated. The resolver currently implements authentication, authorization, and business logic directly instead of delegating to an orchestrator method.

## Current Implementation
The resolver:
1. Extracts pool and session context
2. Gets current user from session
3. Checks if user has `can_manage_roles` permission
4. If they have `can_manage_roles`, also checks `can_manage_admin_role_permission`
5. Returns filtered permissions (excluding `is_admin` if user lacks admin management permission)
6. Admin users (with `is_admin`) bypass the `can_manage_roles` check and see all permissions

## Solution

### Phase 1: Add orchestrator method to RoleOrchestrator

**File:** `src/orchestrators/role_orchestrator.rs`

Add new method `get_all_role_permissions_with_permission_check`:

```rust
impl RoleOrchestrator {
  /// Get all role permissions with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission (OR user has "is_admin")
  /// - Checks if user has "can_manage_admin_role_permission" to include "is_admin" in results
  /// - Returns filtered list of permissions
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context for authentication/authorization
  ///
  /// # Returns
  /// * `Ok(Vec<String>)` - List of available role permissions (filtered by user's admin rights)
  /// * `Err(RoleError)` - Authentication or authorization error
  pub async fn get_all_role_permissions_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<String>, RoleError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await
      .map_err(RoleError::DatabaseError)?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let (can_manage_roles, can_manage_admin) = {
      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?;
      let can_manage_admin =
        RoleService::check_user_permission(&mut conn, &user, "can_manage_admin_role_permission").await?;

      (can_manage_roles, can_manage_admin)
    };

    // Users with can_manage_roles OR is_admin permission can access
    let is_admin = RoleService::check_user_permission(&mut conn, &user, "is_admin").await?;

    if !can_manage_roles && !is_admin {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Filter permissions based on user's admin management rights
    let permissions: Vec<String> = crate::models::ROLE_PERMISSIONS
      .iter()
      .filter(|&&perm| can_manage_admin || perm != "is_admin")
      .map(|s| s.to_string())
      .collect();

    Ok(permissions)
  }
}
```

**Note:** Need to add `use crate::models::role::ROLE_PERMISSIONS;` import at top of file.

### Phase 2: Update role_permissions.rs resolver

**File:** `src/graphql/resolvers/role_permissions.rs`

Simplify the resolver to call the orchestrator method:

```rust
use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use crate::services::role_service::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(Default, Debug)]
pub struct RolePermissionsResolver;

#[Object]
impl RolePermissionsResolver {
  /// Get all available role permissions
  /// Requires can_manage_roles permission
  #[graphql(name = "rolePermissions")]
  #[instrument(skip(self, ctx), fields())]
  async fn role_permissions(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = ctx.data::<SessionContext>()?;

    match RoleOrchestrator::get_all_role_permissions_with_permission_check(
      pool,
      session_context.clone(),
    )
    .await
    {
      Ok(permissions) => Ok(permissions),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to get role permissions: {}", err);
        Err(async_graphql::Error::new("Failed to retrieve role permissions"))
      }
    }
  }
}
```

Remove the following unused imports:
- `use crate::models::role::ROLE_PERMISSIONS;`
- `use crate::queries::users::GetUserByIdQuery;`
- `use crate::services::role_service::RoleService;`

Remove the TODO comment at the top of the file.

### Phase 3: Add orchestrator tests

**File:** `src/orchestrators/role_orchestrator.rs` (in the `#[cfg(test)]` module)

Add tests following the pattern of existing orchestrator tests:

```rust
#[tokio::test]
async fn test_get_all_role_permissions_with_permission_check_without_manage_roles() {
  let (pool, _temp_file) = create_test_database().await;

  // Create regular user role without can_manage_roles
  let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
  let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

  // Create session context for regular user
  let session_payload = DpsAuthSessionPayload {
    sub: regular_user.id,
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(session_payload));

  let result = RoleOrchestrator::get_all_role_permissions_with_permission_check(
    &pool,
    session_context,
  ).await;

  assert!(result.is_err());
  match result.unwrap_err() {
    RoleError::AuthorizationError(msg) => {
      assert!(msg.contains("Forbidden"));
    }
    _ => panic!("Expected AuthorizationError"),
  }
}

#[tokio::test]
async fn test_get_all_role_permissions_with_permission_check_role_manager() {
  let (pool, _temp_file) = create_test_database().await;

  // Create role manager with can_manage_roles but not can_manage_admin_role_permission
  let role_manager_id =
    create_test_role_with_pool(&pool, "role_manager", &["can_manage_roles"]).await;
  let role_manager_user =
    create_test_user_with_pool(&pool, "role_manager", role_manager_id).await;

  // Create session context for role manager
  let session_payload = DpsAuthSessionPayload {
    sub: role_manager_user.id,
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(session_payload));

  let result = RoleOrchestrator::get_all_role_permissions_with_permission_check(
    &pool,
    session_context,
  ).await;

  assert!(result.is_ok());
  let permissions = result.unwrap();
  assert!(!permissions.contains(&"is_admin".to_string()));
  assert!(permissions.contains(&"can_manage_roles".to_string()));
  assert_eq!(permissions.len(), crate::models::ROLE_PERMISSIONS.len() - 1);
}

#[tokio::test]
async fn test_get_all_role_permissions_with_permission_check_admin_manager() {
  let (pool, _temp_file) = create_test_database().await;

  // Create admin manager with both permissions
  let admin_manager_id = create_test_role_with_pool(
    &pool,
    "admin_manager",
    &["can_manage_roles", "can_manage_admin_role_permission"],
  ).await;
  let admin_manager_user =
    create_test_user_with_pool(&pool, "admin_manager", admin_manager_id).await;

  // Create session context for admin manager
  let session_payload = DpsAuthSessionPayload {
    sub: admin_manager_user.id,
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(session_payload));

  let result = RoleOrchestrator::get_all_role_permissions_with_permission_check(
    &pool,
    session_context,
  ).await;

  assert!(result.is_ok());
  let permissions = result.unwrap();
  assert!(permissions.contains(&"is_admin".to_string()));
  assert!(permissions.contains(&"can_manage_roles".to_string()));
  assert!(permissions.contains(&"can_manage_admin_role_permission".to_string()));
  assert_eq!(permissions.len(), crate::models::ROLE_PERMISSIONS.len());
}

#[tokio::test]
async fn test_get_all_role_permissions_with_permission_check_admin() {
  let (pool, _temp_file) = create_test_database().await;

  // Create admin user (with is_admin, bypasses can_manage_roles requirement)
  let admin_role_id = create_test_role_with_pool(&pool, "admin", &["is_admin"]).await;
  let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

  // Create session context for admin
  let session_payload = DpsAuthSessionPayload {
    sub: admin_user.id,
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(session_payload));

  let result = RoleOrchestrator::get_all_role_permissions_with_permission_check(
    &pool,
    session_context,
  ).await;

  assert!(result.is_ok());
  let permissions = result.unwrap();
  assert!(permissions.contains(&"is_admin".to_string()));
  assert_eq!(permissions.len(), crate::models::ROLE_PERMISSIONS.len());
}

#[tokio::test]
async fn test_get_all_role_permissions_with_permission_check_unauthenticated() {
  let (pool, _temp_file) = create_test_database().await;

  // Create session context without user (not authenticated)
  let session_context = SessionContext::new(None);

  let result = RoleOrchestrator::get_all_role_permissions_with_permission_check(
    &pool,
    session_context,
  ).await;

  assert!(result.is_err());
  match result.unwrap_err() {
    RoleError::AuthenticationError(msg) => {
      assert!(msg.contains("Authentication required"));
    }
    _ => panic!("Expected AuthenticationError"),
  }
}
```

## Summary

This plan follows the established pattern used by all other resolvers in the codebase:

1. **Authentication** - Orchestrator checks if user is logged in
2. **Authorization** - Orchestrator verifies user has appropriate permissions
3. **Business Logic** - Orchestrator filters permissions based on admin management rights
4. **Resolver** - Thin wrapper that calls orchestrator and maps errors to GraphQL errors

The resolver tests will continue to work as they test the GraphQL endpoint end-to-end, while the new orchestrator tests will verify the business logic at the orchestrator level.
