# Plan: Create Static Role Permissions Array

**Date**: 2025-12-10@07:41

## Overview

Create a static array of all role permissions to whitelist allowed permissions and enable compile-time validation. This will improve type safety and prevent permission typos.

## Current Permission Usage Analysis

Based on scanning the `src` folder, here are all existing permissions found:

### Core Permissions
- `is_admin` - Special admin permission that grants all other permissions
- `can_view_user_self` - View own user details

### User Management Permissions
- `can_list_users` - List all users (src/orchestrators/user_orchestrator.rs:30, src/graphql/resolvers/users.rs:34)
- `can_view_user_details` - View detailed user information (src/orchestrators/user_orchestrator.rs:77, src/graphql/resolvers/user.rs:36)
- `can_delete_user` - Delete users (src/orchestrators/user_orchestrator.rs:126, src/graphql/resolvers/delete_user.rs:18)
- `can_edit_user` - Edit user information (src/orchestrators/user_orchestrator.rs:189)

### Site Management Permissions
- `can_create_site` - Create new sites (src/orchestrators/site_orchestrator.rs:28, src/graphql/resolvers/add_site.rs:43)
- `can_delete_site` - Delete sites (src/orchestrators/site_orchestrator.rs:58, src/graphql/resolvers/remove_site.rs:42)
- `can_update_site` - Update existing sites (src/orchestrators/site_orchestrator.rs:89, src/graphql/resolvers/update_site.rs:43)
- `can_view_site_details` - View detailed site information (src/orchestrators/site_orchestrator.rs:119, src/graphql/resolvers/site.rs:42)

### New Permissions to Add
- `can_edit_user_role` - Edit user role assignments
- `can_manage_roles` - Manage role definitions and permissions
- `can_manage_admin_role_permission` - Manage admin role permissions (highly privileged)

## Implementation Phases

### Phase 1: Create Static Permissions Array

**File**: `src/models/user_role.rs`

Add a static array containing all valid permissions:

```rust
/// Static array of all valid role permissions
pub const ROLE_PERMISSIONS: &[&str] = &[
    "is_admin",
    "can_view_user_self",
    "can_list_users",
    "can_view_user_details",
    "can_delete_user",
    "can_edit_user",
    "can_create_site",
    "can_delete_site",
    "can_update_site",
    "can_view_site_details",
    "can_edit_user_role",
    "can_manage_roles",
    "can_manage_admin_role_permission",
];
```

Add a validation function:

```rust
/// Check if a permission string is valid
pub fn is_valid_role_permission(permission: &str) -> bool {
    ROLE_PERMISSIONS.contains(&permission)
}
```

### Phase 2: Update Permission Checking Method

**File**: `src/services/user_role_service.rs`

Modify the `check_user_permission` method to validate permissions at runtime:

```rust
pub async fn check_user_permission(
    pool: &SqlitePool,
    user: &User,
    permission: &str,
) -> Result<bool, sqlx::Error> {
    // Validate permission exists
    if !crate::models::user_role::is_valid_role_permission(permission) {
        log::warn!("Invalid permission checked: {}", permission);
        return Ok(false);
    }
    
    // ... rest of existing logic
}
```

### Phase 3: Create Role Permissions Resolver

**File**: `src/graphql/resolvers/role_permissions.rs`

Create a new query resolver to list all available permissions:

```rust
use async_graphql::{Context, Object, Result};
use crate::models::user_role::ROLE_PERMISSIONS;
use crate::orchestrators::auth_orchestrator::AuthOrchestrator;
use crate::services::user_role_service::UserRoleService;
use sqlx::SqlitePool;

pub struct RolePermissionsResolver;

#[Object]
impl RolePermissionsResolver {
    /// Get all available role permissions
    /// Requires can_manage_roles permission
    #[graphql(name = "rolePermissions")]
    async fn role_permissions(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let pool = ctx.data::<SqlitePool>()?;
        let session = ctx.data::<crate::graphql::types::SessionPayload>()?;
        
        // Get current user
        let user = AuthOrchestrator::get_current_user(pool, &session.uuid).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get current user: {}", e)))?;
        
        // Check base permission
        let allowed = UserRoleService::check_user_permission(pool, &user, "can_manage_roles").await
            .map_err(|e| async_graphql::Error::new(format!("Permission check failed: {}", e)))?;
        
        if !allowed {
            return Err(async_graphql::Error::new("Forbidden: Insufficient permissions"));
        }
        
        // Check if user can manage admin role permissions
        let can_manage_admin = UserRoleService::check_user_permission(pool, &user, "can_manage_admin_role_permission").await
            .map_err(|e| async_graphql::Error::new(format!("Permission check failed: {}", e)))?;
        
        // Filter permissions based on user's admin management rights
        let permissions: Vec<String> = ROLE_PERMISSIONS
            .iter()
            .filter(|&&perm| can_manage_admin || perm != "is_admin")
            .map(|s| s.to_string())
            .collect();
        
        Ok(permissions)
    }
}
```

**File**: `src/graphql/resolvers/mod.rs`

Add the new resolver to the module exports:

```rust
pub mod role_permissions;
// ... other imports
pub use role_permissions::RolePermissionsResolver;
```

**File**: `src/graphql/schema.rs`

Add the new resolver to the imports and Query struct:

```rust
use crate::graphql::resolvers::{
  AddSiteResolver, AuthChangePasswordResolver, AuthLoginResolver, AuthLogoutResolver,
  AuthMeResolver, AuthRegisterResolver, DeleteUserResolver, GetServerTimestampResolver,
  RemoveSiteResolver, RolePermissionsResolver, SiteResolver, SitesResolver, UpdateSiteResolver, 
  UserResolver, UsersResolver,
};
```

Add `RolePermissionsResolver` to the Query struct:

```rust
#[derive(MergedObject, Default)]
pub struct Query(
  GetServerTimestampResolver,
  AuthMeResolver,
  SitesResolver,
  SiteResolver,
  UsersResolver,
  UserResolver,
  RolePermissionsResolver,
);
```

Update the `Query::new()` method to include the new resolver:

```rust
impl Query {
  pub fn new() -> Self {
    Self(
      GetServerTimestampResolver,
      AuthMeResolver,
      SitesResolver,
      SiteResolver,
      UsersResolver,
      UserResolver,
      RolePermissionsResolver,
    )
  }
}
```

### Phase 4: Update README

**File**: `README.md`

Add the new resolver to the Queries table:

```markdown
| `rolePermissions` | List all available role permissions. Returns array of permission strings. Requires can_manage_roles permission. |
```

### Phase 5: Update AGENTS.md

**File**: `AGENTS.md`

Add a new section after the Database section about role permission usage:

```markdown
## Role Permission Usage

### Static Permissions Array
All valid role permissions are defined in a static array at `src/models/user_role.rs:ROLE_PERMISSIONS`. This array serves as the whitelist of all allowed permissions in the system.

### Adding/Removing Permissions
When adding new permissions or removing existing ones:
1. Update the `ROLE_PERMISSIONS` array in `src/models/user_role.rs`
2. The `is_valid_role_permission()` function will automatically validate against the updated array
3. All permission checks throughout the codebase use this validation

### Permission Checking
Permission checks are performed through `UserRoleService::check_user_permission()` which validates that the permission exists in the static array before checking the user's role.
```

## Testing Considerations

- Add tests for the new `is_valid_role_permission()` function
- Add tests for the `rolePermissions` resolver with different permission levels
- Verify existing permission checks still work with validation
- Test that invalid permissions return false

## Benefits

1. **Type Safety**: Prevents permission typos at runtime
2. **Centralized Management**: All permissions in one location
3. **Discoverability**: New resolver allows frontend to fetch all permissions
4. **Maintainability**: Easy to add/remove permissions
5. **Security**: Whitelist approach prevents unauthorized permissions