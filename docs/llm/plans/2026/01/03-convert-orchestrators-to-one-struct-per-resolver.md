# Plan: Convert Orchestrators to One-Struct-One-Run-Method Pattern

## Overview

Convert all orchestrators from the current "one struct with multiple methods" pattern to the "one orchestrator per resolver with a single `run()` method" pattern as outlined in AGENTS.md.

**Current State:**
- 4 orchestrator files with 17 total methods
- Files: `auth_orchestrator.rs`, `user_orchestrator.rs`, `role_orchestrator.rs`, `site_orchestrator.rs`
- Each file has a struct with multiple methods
- All methods include extensive tests in the same file
- This makes files unwieldy and harder to work with

**Target State:**
- 17 individual orchestrator files, one per resolver
- Each file has a single struct with a single `run()` method
- Tests remain with the orchestrator code
- Old orchestrator files are deleted after all methods are migrated
- File structure: `src/orchestrators/<domain>/<resolver_name>.rs`

## Orchestrator Conversion Map

### AuthOrchestrator (2 methods)
1. `get_authenticated_user` → `src/orchestrators/auth/auth_me.rs` → `AuthMeOrchestrator`
   - Called by: `AuthMeResolver`
2. `change_authenticated_user_password` → `src/orchestrators/auth/auth_change_password.rs` → `AuthChangePasswordOrchestrator`
   - Called by: `AuthChangePasswordResolver`

### UserOrchestrator (4 methods)
3. `list_users_with_permission_check` → `src/orchestrators/user/users.rs` → `UsersOrchestrator`
   - Called by: `UsersResolver`
4. `get_user_details_with_permission_check` → `src/orchestrators/user/user.rs` → `UserOrchestrator`
   - Called by: `UserResolver`
5. `delete_user_with_permission_check` → `src/orchestrators/user/delete_user.rs` → `DeleteUserOrchestrator`
   - Called by: `DeleteUserResolver`
6. `update_user_with_permission_check` → `src/orchestrators/user/update_user.rs` → `UpdateUserOrchestrator`
   - Called by: `UpdateUserResolver`

### RoleOrchestrator (6 methods)
7. `get_all_roles_with_permission_check` → `src/orchestrators/role/roles.rs` → `RolesOrchestrator`
   - Called by: `RolesResolver`
8. `get_role_by_id_with_permission_check` → `src/orchestrators/role/role.rs` → `RoleOrchestrator`
   - Called by: `RoleResolver`
9. `update_role_with_permission_check` → `src/orchestrators/role/update_role.rs` → `UpdateRoleOrchestrator`
   - Called by: `UpdateRoleResolver`
10. `create_role_with_permission_check` → `src/orchestrators/role/add_role.rs` → `AddRoleOrchestrator`
    - Called by: `AddRoleResolver`
11. `delete_role_with_permission_check` → `src/orchestrators/role/remove_role.rs` → `RemoveRoleOrchestrator`
    - Called by: `RemoveRoleResolver`
12. `set_default_role_with_permission_check` → `src/orchestrators/role/set_default_role.rs` → `SetDefaultRoleOrchestrator`
    - Called by: `SetDefaultRoleResolver`
13. `get_all_role_permissions_with_permission_check` → `src/orchestrators/role/role_permissions.rs` → `RolePermissionsOrchestrator`
    - Called by: `RolePermissionsResolver`

### SiteOrchestrator (4 methods)
14. `create_site_with_permission_check` → `src/orchestrators/site/add_site.rs` → `AddSiteOrchestrator`
    - Called by: `AddSiteResolver`
15. `remove_site_with_permission_check` → `src/orchestrators/site/remove_site.rs` → `RemoveSiteOrchestrator`
    - Called by: `RemoveSiteResolver`
16. `update_site_with_permission_check` → `src/orchestrators/site/update_site.rs` → `UpdateSiteOrchestrator`
    - Called by: `UpdateSiteResolver`
17. `get_site_details_with_permission_check` → `src/orchestrators/site/site.rs` → `SiteOrchestrator`
    - Called by: `SiteResolver`

## New Orchestrator File Template

Each new orchestrator file should follow this structure:

```rust
use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::RoleService;
use crate::types::UserError;
use sqlx::SqlitePool;

pub struct UsersOrchestrator;

impl UsersOrchestrator {
  /// Static method (associated function) - struct is only used as namespace
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<UserWithRole>, UserError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    let allowed = RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all users
    GetAllUsersWithRolesQuery::run(&mut conn)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  // All existing tests moved here, adapted to call UsersOrchestrator::run()
}
```

**Note:** The `run` method is a static/associated function (no `self` parameter). The struct has no fields and is only used as a namespace for organizing code. Called as `UsersOrchestrator::run(pool, session_context)`.

## Implementation Phases

### Phase 1: Convert Auth Orchestrator Methods

#### 1.1 Create `src/orchestrators/auth/` directory
#### 1.2 Create `src/orchestrators/auth/auth_me.rs`
- Move `AuthOrchestrator::get_authenticated_user()` logic to `AuthMeOrchestrator::run()`
- Move and adapt all tests from `test_get_authenticated_user_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `auth::AuthMeOrchestrator`
- Update `AuthMeResolver` to call `AuthMeOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 1.3 Create `src/orchestrators/auth/auth_change_password.rs`
- Move `AuthOrchestrator::change_authenticated_user_password()` logic to `AuthChangePasswordOrchestrator::run()`
- Move and adapt all tests from `test_change_authenticated_user_password_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `auth::AuthChangePasswordOrchestrator`
- Update `AuthChangePasswordResolver` to call `AuthChangePasswordOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 1.4 Delete `src/orchestrators/auth_orchestrator.rs`
- Verify no resolvers call it anymore
- Remove from `src/orchestrators/mod.rs`

### Phase 2: Convert User Orchestrator Methods

#### 2.1 Create `src/orchestrators/user/` directory
#### 2.2 Create `src/orchestrators/user/users.rs`
- Move `UserOrchestrator::list_users_with_permission_check()` logic to `UsersOrchestrator::run()`
- Move and adapt all tests from `test_list_users_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `user::UsersOrchestrator`
- Update `UsersResolver` to call `UsersOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 2.3 Create `src/orchestrators/user/user.rs`
- Move `UserOrchestrator::get_user_details_with_permission_check()` logic to `UserOrchestrator::run()`
- Move and adapt all tests from `test_get_user_details_with_permission_check_*`
- Keep existing comments and documentation
- Update `UserResolver` to import `use crate::orchestrators::user::UserOrchestrator;` and call `UserOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

**Note:** See "Naming Conflicts During Migration" section for how to handle the `UserOrchestrator` naming conflict.

#### 2.4 Create `src/orchestrators/user/delete_user.rs`
- Move `UserOrchestrator::delete_user_with_permission_check()` logic to `DeleteUserOrchestrator::run()`
- Move and adapt all tests from `test_delete_user_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `user::DeleteUserOrchestrator`
- Update `DeleteUserResolver` to call `DeleteUserOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 2.5 Create `src/orchestrators/user/update_user.rs`
- Move `UserOrchestrator::update_user_with_permission_check()` logic to `UpdateUserOrchestrator::run()`
- Move and adapt all tests from `test_update_user_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `user::UpdateUserOrchestrator`
- Update `UpdateUserResolver` to call `UpdateUserOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 2.6 Delete `src/orchestrators/user_orchestrator.rs`
- Verify no resolvers call it anymore
- Remove from `src/orchestrators/mod.rs`

### Phase 3: Convert Role Orchestrator Methods

#### 3.1 Create `src/orchestrators/role/` directory
#### 3.2 Create `src/orchestrators/role/roles.rs`
- Move `RoleOrchestrator::get_all_roles_with_permission_check()` logic to `RolesOrchestrator::run()`
- Move and adapt all tests from `test_get_all_roles_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::RolesOrchestrator`
- Update `RolesResolver` to call `RolesOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.3 Create `src/orchestrators/role/role.rs`
- Move `RoleOrchestrator::get_role_by_id_with_permission_check()` logic to `RoleOrchestrator::run()`
- Move and adapt all tests from `test_get_role_by_id_with_permission_check_*`
- Keep existing comments and documentation
- Update `RoleResolver` to import `use crate::orchestrators::role::RoleOrchestrator;` and call `RoleOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

**Note:** See "Naming Conflicts During Migration" section for how to handle `RoleOrchestrator` naming conflict.

#### 3.4 Create `src/orchestrators/role/update_role.rs`
- Move `RoleOrchestrator::update_role_with_permission_check()` logic to `UpdateRoleOrchestrator::run()`
- Move and adapt all tests from `test_update_role_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::UpdateRoleOrchestrator`
- Update `UpdateRoleResolver` to call `UpdateRoleOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.5 Create `src/orchestrators/role/add_role.rs`
- Move `RoleOrchestrator::create_role_with_permission_check()` logic to `AddRoleOrchestrator::run()`
- Move and adapt all tests from `test_create_role_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::AddRoleOrchestrator`
- Update `AddRoleResolver` to call `AddRoleOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.6 Create `src/orchestrators/role/remove_role.rs`
- Move `RoleOrchestrator::delete_role_with_permission_check()` logic to `RemoveRoleOrchestrator::run()`
- Move and adapt all tests from `test_delete_role_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::RemoveRoleOrchestrator`
- Update `RemoveRoleResolver` to call `RemoveRoleOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.7 Create `src/orchestrators/role/set_default_role.rs`
- Move `RoleOrchestrator::set_default_role_with_permission_check()` logic to `SetDefaultRoleOrchestrator::run()`
- Move and adapt all tests from `test_set_default_role_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::SetDefaultRoleOrchestrator`
- Update `SetDefaultRoleResolver` to call `SetDefaultRoleOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.8 Create `src/orchestrators/role/role_permissions.rs`
- Move `RoleOrchestrator::get_all_role_permissions_with_permission_check()` logic to `RolePermissionsOrchestrator::run()`
- Move and adapt all tests from `test_get_all_role_permissions_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `role::RolePermissionsOrchestrator`
- Update `RolePermissionsResolver` to call `RolePermissionsOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 3.9 Delete `src/orchestrators/role_orchestrator.rs`
- Verify no resolvers call it anymore
- Remove from `src/orchestrators/mod.rs`

### Phase 4: Convert Site Orchestrator Methods

#### 4.1 Create `src/orchestrators/site/` directory
#### 4.2 Create `src/orchestrators/site/add_site.rs`
- Move `SiteOrchestrator::create_site_with_permission_check()` logic to `AddSiteOrchestrator::run()`
- Move and adapt all tests from `test_create_site_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `site::AddSiteOrchestrator`
- Update `AddSiteResolver` to call `AddSiteOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 4.3 Create `src/orchestrators/site/remove_site.rs`
- Move `SiteOrchestrator::remove_site_with_permission_check()` logic to `RemoveSiteOrchestrator::run()`
- Move and adapt all tests from `test_remove_site_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `site::RemoveSiteOrchestrator`
- Update `RemoveSiteResolver` to call `RemoveSiteOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 4.4 Create `src/orchestrators/site/update_site.rs`
- Move `SiteOrchestrator::update_site_with_permission_check()` logic to `UpdateSiteOrchestrator::run()`
- Move and adapt all tests from `test_update_site_with_permission_check_*`
- Keep existing comments and documentation
- Update `src/orchestrators/mod.rs` to export `site::UpdateSiteOrchestrator`
- Update `UpdateSiteResolver` to call `UpdateSiteOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

#### 4.5 Create `src/orchestrators/site/site.rs`
- Move `SiteOrchestrator::get_site_details_with_permission_check()` logic to `SiteOrchestrator::run()`
- Move and adapt all tests from `test_get_site_details_with_permission_check_*`
- Keep existing comments and documentation
- Update `SiteResolver` to import `use crate::orchestrators::site::SiteOrchestrator;` and call `SiteOrchestrator::run()`
- Run tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

**Note:** See "Naming Conflicts During Migration" section for how to handle `SiteOrchestrator` naming conflict.

#### 4.6 Delete `src/orchestrators/site_orchestrator.rs`
- Verify no resolvers call it anymore
- Remove from `src/orchestrators/mod.rs`

### Phase 5: Final Verification

#### 5.1 Final mod.rs structure
After all old orchestrator files are deleted, update `src/orchestrators/mod.rs` to:
```rust
pub mod auth;
pub mod user;
pub mod role;
pub mod site;

// Full re-export - safe now that old files are deleted
pub use auth::*;
pub use user::*;
pub use role::*;
pub use site::*;
```

**Note:** During migration phases (2.3, 3.3, 4.5), resolvers imported directly from submodule paths (e.g., `crate::orchestrators::user::UserOrchestrator`) to avoid conflicts. After old files are deleted in phases 2.6, 3.9, 4.6, we can safely do full re-exports.

#### 5.2 Create domain mod.rs files
Create the following mod.rs files in each domain directory:
- `src/orchestrators/auth/mod.rs`
- `src/orchestrators/user/mod.rs`
- `src/orchestrators/role/mod.rs`
- `src/orchestrators/site/mod.rs`

Each should export all orchestrators in that domain:
```rust
pub mod auth_me;
pub mod auth_change_password;

pub use auth_me::AuthMeOrchestrator;
pub use auth_change_password::AuthChangePasswordOrchestrator;
```

#### 5.3 Final test run
- Run all tests: `cargo test --quiet`
- Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`
- Verify build passes: `cargo build`

## Key Implementation Notes

### Naming Conflicts During Migration

During migration, there will be naming conflicts between new and old orchestrators:

- **Phase 2.3:** New `UserOrchestrator` (get one user) vs old `UserOrchestrator` (multiple methods)
- **Phase 3.3:** New `RoleOrchestrator` (get one role) vs old `RoleOrchestrator` (multiple methods)
- **Phase 4.5:** New `SiteOrchestrator` (get one site) vs old `SiteOrchestrator` (multiple methods)

**Solution for each conflict:**
1. Import directly from submodule path: `use crate::orchestrators::user::UserOrchestrator;`
2. Do NOT re-export at top level in `orchestrators/mod.rs` yet
3. After old file is deleted in subsequent phase, full re-exports are safe

This works because modules don't conflict - only re-exports at same level cause name collisions.

1. **Naming Convention**: Orchestrator structs are named after the resolver they support with `Orchestrator` suffix (e.g., `AuthMeResolver` → `AuthMeOrchestrator`)

2. **Single run() Method**: Each orchestrator has only one public method: `async fn run(...)` - it's a static/associated function (no `self` parameter) called as `NameOrchestrator::run(...)`

3. **Input Structure**: The `run()` method accepts:
   - `&SqlitePool` (database pool)
   - `SessionContext` (for authentication/authorization)
   - Additional resolver-specific parameters (e.g., `target_user_id`, `update_data`, etc.)

4. **Preserve All Comments**: All existing documentation and code comments must be preserved during migration

5. **Tests Move with Logic**: All test code moves with orchestrator logic to the new file

6. **Resolver Updates**: Update each resolver to call `<Name>Orchestrator::run()` instead of `<Domain>Orchestrator::<method>()`

7. **One Phase Per Method**: Each orchestrator method migration is a separate phase with:
   - Creating new orchestrator file
   - Moving logic and tests
   - Updating mod.rs
   - Updating resolver
   - Running tests
   - Running linter

8. **Delete Old Files**: Only delete old orchestrator files after all their methods have been migrated and no resolvers call them

1. **Naming Convention**: Orchestrator structs are named after the resolver they support with `Orchestrator` suffix (e.g., `AuthMeResolver` → `AuthMeOrchestrator`)

2. **Single run() Method**: Each orchestrator has only one public method: `async fn run(...)`

3. **Input Structure**: The `run()` method accepts:
   - `&SqlitePool` (database pool)
   - `SessionContext` (for authentication/authorization)
   - Additional resolver-specific parameters (e.g., `target_user_id`, `update_data`, etc.)

4. **Preserve All Comments**: All existing documentation and code comments must be preserved during migration

5. **Tests Move with Logic**: All test code moves with the orchestrator logic to the new file

6. **Resolver Updates**: Update each resolver to call `<Name>Orchestrator::run()` instead of `<Domain>Orchestrator::<method>()`

7. **One Phase Per Method**: Each orchestrator method migration is a separate phase with:
   - Creating new orchestrator file
   - Moving logic and tests
   - Updating mod.rs
   - Updating resolver
   - Running tests
   - Running linter

8. **Delete Old Files**: Only delete old orchestrator files after all their methods have been migrated and no resolvers call them

## File Structure After Migration

```
src/orchestrators/
├── mod.rs
├── auth/
│   ├── mod.rs
│   ├── auth_me.rs
│   └── auth_change_password.rs
├── user/
│   ├── mod.rs
│   ├── users.rs
│   ├── user.rs
│   ├── delete_user.rs
│   └── update_user.rs
├── role/
│   ├── mod.rs
│   ├── roles.rs
│   ├── role.rs
│   ├── update_role.rs
│   ├── add_role.rs
│   ├── remove_role.rs
│   ├── set_default_role.rs
│   └── role_permissions.rs
└── site/
    ├── mod.rs
    ├── add_site.rs
    ├── remove_site.rs
    ├── update_site.rs
    └── site.rs
```

## Benefits

1. **Smaller Files**: Each orchestrator file contains only the logic for one resolver workflow
2. **Easier Testing**: Tests are co-located with the specific logic they test
3. **Better Maintainability**: Easier to find and modify specific orchestrator logic
4. **Clearer Ownership**: One orchestrator per resolver makes the code structure more intuitive
5. **Follows AGENTS.md Guidelines**: Aligns with project's architectural standards
