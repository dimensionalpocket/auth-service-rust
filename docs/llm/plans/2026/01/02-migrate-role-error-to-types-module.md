# Migrate RoleError Type to Types Module

**Date**: 2026-01-02@18:17

## Overview

Migrate the `RoleError` enum from `src/services/role_service.rs` to its own file in `src/types/role/role_error.rs` following the established pattern for app-wide types (non-GraphQL).

## Files to Create

### `src/types/role/role_error.rs`

```rust
/// Custom error type for role operations
#[derive(Debug)]
pub enum RoleError {
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Authentication failed (used by orchestrators)
  AuthenticationError(String),
  /// Authorization failed (used by orchestrators)
  AuthorizationError(String),
  /// Role not found
  RoleNotFound(i64),
  /// Role name already exists
  RoleNameAlreadyExists(String),
  /// Role is in use and cannot be deleted
  RoleInUse(i64),
  /// Input validation failed
  ValidationError(String),
  /// Invalid permission string
  InvalidPermission(String),
}

impl std::fmt::Display for RoleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RoleError::DatabaseError(err) => write!(f, "Database error: {err}"),
      RoleError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      RoleError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
      RoleError::RoleNotFound(id) => write!(f, "Role not found: {id}"),
      RoleError::RoleNameAlreadyExists(name) => write!(f, "Role name already exists: {name}"),
      RoleError::RoleInUse(id) => write!(f, "Role is in use and cannot be deleted: {id}"),
      RoleError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      RoleError::InvalidPermission(permission) => write!(f, "Invalid permission: {permission}"),
    }
  }
}

impl std::error::Error for RoleError {}

impl From<sqlx::Error> for RoleError {
  fn from(err: sqlx::Error) -> Self {
    RoleError::DatabaseError(err)
  }
}
```

### `src/types/role/mod.rs`

```rust
pub mod role_error;

pub use role_error::RoleError;
```

## Files to Modify

### `src/types/mod.rs`

Add the role module:

```rust
pub mod role;
pub mod user;

pub use role::RoleError;
pub use user::UpdateUserInput;
```

### `src/services/role_service.rs`

Remove the `RoleError` enum definition (lines 12-53) and add import at top:

```rust
use crate::models::role::is_valid_role_permission;
use crate::models::role::Role;
use crate::models::user::User;
use crate::queries::roles::{
  CreateRoleData, CreateRoleQuery, DeleteRoleQuery, GetAllRolesQuery, GetRoleByIdQuery,
  GetRoleByNameQuery, SetDefaultRoleQuery, UpdateRoleData, UpdateRoleQuery,
};
use crate::types::RoleError;
use sqlx::{Row, SqliteConnection};
use tracing::warn;

pub struct RoleService;

impl RoleService {
  // ... rest of the service code, no changes needed
}
```

### `src/orchestrators/role_orchestrator.rs`

Update import:

```rust
use crate::services::role_service::RoleService;
use crate::types::RoleError;
```

### `src/orchestrators/user_orchestrator.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/orchestrators/site_orchestrator.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/add_role.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/role.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/remove_role.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/update_role.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/set_default_role.rs`

Update import:

```rust
use crate::types::RoleError;
```

### `src/graphql/resolvers/role_permissions.rs`

Update import:

```rust
use crate::types::RoleError;
```

## Implementation Steps

1. Create `src/types/role/role_error.rs` with the `RoleError` enum definition
2. Create `src/types/role/mod.rs` to export `RoleError`
3. Update `src/types/mod.rs` to include and export the new `role` module
4. Remove the `RoleError` enum definition from `src/services/role_service.rs`
5. Add `use crate::types::RoleError;` to `src/services/role_service.rs`
6. Update all 9 import statements across orchestrator and resolver files to use the new path

## Testing

After implementation, run:
- `cargo test --quiet` to ensure all tests pass
- `cargo clippy --allow-dirty --fix && cargo fmt` to ensure code quality
