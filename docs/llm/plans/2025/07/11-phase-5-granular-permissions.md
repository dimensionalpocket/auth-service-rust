# Phase 5: Granular Permissions for User Roles

**Date**: 2025-07-11@13:04  
**Phase**: 5 - User Roles and Permissions  

## Current State Analysis

The project currently has:
- Basic `user_roles` table with `id`, `name`, `created_ts`, and `is_default` fields
- Two default roles: `admin` and `user` 
- Users are assigned to roles via `role_id` foreign key
- No granular permissions system yet

## Objective

Implement a granular permissions system where:
- Permissions are specific actions (e.g., `can_create_user`, `can_delete_user`)
- Permissions can be assigned to roles
- The system can check if a user has a specific permission through their role

## Selected Approach: JSON Permissions Field

**Database Structure:**
- Add `permissions_json` TEXT field to `user_roles` table
- Store permissions as JSON array (e.g., `["can_create_user", "can_delete_user"]`)

**Benefits:**
- Simpler database structure
- Faster queries (no JOINs needed)
- Easy to serialize/deserialize in Rust
- Fewer tables to manage
- Quick to implement

## Implementation Plan

### 1. Database Schema Changes

**Update Existing Migration**: `config/database/migrations/001_create_user_roles.sql`
```sql
-- Create user_roles table
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    permissions_json TEXT NOT NULL DEFAULT '[]'
);
```

### 2. Update Seeds

**Update**: `config/database/seeds/001_default_roles.sql`
```sql
-- Insert default user roles with permissions
INSERT OR IGNORE INTO user_roles (name, created_ts, is_default, permissions_json) VALUES 
    ('admin', strftime('%s', 'now'), FALSE, '["is_admin"]'),
    ('user', strftime('%s', 'now'), TRUE, '["can_view_user_self", "can_update_user_self", "can_create_email_self", "can_delete_email_self"]');
```

### 3. Update Rust Models

**Update**: `src/models/user_role.rs`
```rust
use sqlx::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
  pub is_default: bool,
  #[sqlx(rename = "permissions_json")]
  pub permissions: Vec<String>,
}

impl UserRole {
  /// Check if this role has a specific permission
  pub fn has_permission(&self, permission: &str) -> bool {
    self.permissions.contains(&permission.to_string())
  }
}
```

### 4. Create UserRoleService

**New File**: `src/services/user_role_service.rs`
```rust
use crate::models::user_role::UserRole;
use crate::queries::user_roles::{GetRoleByIdQuery, GetRoleByNameQuery};
use crate::queries::users::GetUserByIdQuery;
use sqlx::SqlitePool;

pub struct UserRoleService;

impl UserRoleService {
  /// Get a role by ID
  pub async fn get_role_by_id(pool: &SqlitePool, role_id: i64) -> Result<Option<UserRole>, sqlx::Error> {
    GetRoleByIdQuery::run(pool, role_id).await
  }

  /// Get a role by name
  pub async fn get_role_by_name(pool: &SqlitePool, name: &str) -> Result<Option<UserRole>, sqlx::Error> {
    GetRoleByNameQuery::run(pool, name).await
  }

  /// Check if a user has a specific permission
  /// 
  /// This method checks if the given user has the specified permission through their role.
  /// Special handling for admin users: if the user's role has the "is_admin" permission,
  /// this method will return true for ANY permission check, regardless of what specific
  /// permission is being requested.
  /// 
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user` - The user to check permissions for (required, not optional)
  /// * `permission` - The permission string to check for
  /// 
  /// # Returns
  /// * `Ok(true)` - User has the permission (or is admin)
  /// * `Ok(false)` - User does not have the permission
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn check_user_permission(pool: &SqlitePool, user: &User, permission: &str) -> Result<bool, sqlx::Error> {
    let role = GetRoleByIdQuery::run(pool, user.role_id).await?;
    
    match role {
      Some(role) => {
        // Check if user is admin first - admins have all permissions
        if role.has_permission("is_admin") {
          Ok(true)
        } else {
          // For non-admin users, check the specific permission
          Ok(role.has_permission(permission))
        }
      },
      None => Ok(false),
    }
  }
}
```

### 5. Update Service Module

**Update**: `src/services/mod.rs`
```rust
pub mod password_service;
pub mod server_service;
pub mod user_role_service;
```

### 6. Files to Create/Modify

**Files to Create:**
- `src/services/user_role_service.rs`

**Files to Modify:**
- `config/database/migrations/001_create_user_roles.sql` - Add permissions_json field
- `src/models/user_role.rs` - Add permissions field and has_permission method
- `config/database/seeds/001_default_roles.sql` - Add permissions to default roles
- `src/services/mod.rs` - Export new service
- `Cargo.toml` - Add serde dependency if not already present

### 7. Testing Strategy

**Unit Tests:**
- Test `UserRole::has_permission` method with various permission scenarios
- Test `UserRoleService::check_user_permission` method:
  - Regular user with specific permission (should return true)
  - Regular user without specific permission (should return false)
  - Admin user with any permission (should always return true due to is_admin)
  - User with no role assigned (should return false)
- Test JSON serialization/deserialization of permissions array
- Test `UserRoleService::get_role_by_id` and `get_role_by_name` methods

**Integration Tests:**
- Test migration runs successfully and adds permissions_json field
- Test seeds populate permissions correctly for both admin and user roles
- Test complete permission checking flow:
  - Create test users with admin and user roles
  - Verify admin can perform any action
  - Verify regular user can only perform self-service actions
  - Verify permission checks work end-to-end

**Test Files to Update:**
- Update existing user role query tests to handle new permissions field
- Update integration tests that create users to account for new permissions
- Add new test cases for permission checking scenarios

**Sample Test Cases:**
```rust
#[tokio::test]
async fn test_admin_has_all_permissions() {
    // Admin user should return true for any permission check
}

#[tokio::test]
async fn test_user_self_permissions() {
    // Regular user should have self-service permissions
}

#[tokio::test]
async fn test_user_lacks_admin_permissions() {
    // Regular user should not have admin-level permissions
}
```

### 8. Example Permissions for Auth Service

**Default Permissions:**
- `is_admin` - Admin role with full access (assigned to admin role)
- `can_view_user_self` - View own user information (assigned to user role)
- `can_update_user_self` - Update own user information (assigned to user role)
- `can_create_email_self` - Create email addresses for own account (assigned to user role)
- `can_delete_email_self` - Delete email addresses from own account (assigned to user role)

**Additional Permissions (for future use):**
- `can_create_user` - Create new users
- `can_delete_user` - Delete existing users
- `can_manage_roles` - Create/modify user roles
- `can_view_users` - View user information

### 9. Dependencies

**Required Cargo.toml additions:**
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

This implementation provides a simple, efficient permissions system that can be easily extended with new permissions as needed.