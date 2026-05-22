# Migrate All Error Types to Types Module

**Date**: 2026-01-02@18:24

## Overview

This plan outlines the migration of all service-level error types to the `src/types` module, following the same pattern established with `RoleError`. Error types should be categorized by domain and organized in the types module for better code organization.

## Error Types to Migrate

### 1. SessionError
- **Current Location**: `src/services/session_service.rs` (lines 12-42)
- **Target Location**: `src/types/session/session_error.rs`
- **Domain**: Authentication/Session
- **Usage Count**: 19 occurrences across 6 files
- **Re-exported**: Yes, via `src/services/mod.rs:15`

### 2. UserError
- **Current Location**: `src/services/user_service.rs` (lines 12-68)
- **Target Location**: `src/types/user/user_error.rs`
- **Domain**: User Management
- **Usage Count**: 12 occurrences across 10 files
- **Re-exported**: Yes, via `src/services/mod.rs:20`

### 3. SiteError
- **Current Location**: `src/services/site_service.rs` (lines 10-46)
- **Target Location**: `src/types/site/site_error.rs`
- **Domain**: Site Management
- **Usage Count**: 6 occurrences across 6 files
- **Re-exported**: Yes, via `src/services/mod.rs:19`

### 4. PasswordError
- **Current Location**: `src/services/password_service.rs` (lines 10-29)
- **Target Location**: `src/types/password/password_error.rs`
- **Domain**: Password Management
- **Usage Count**: 2 occurrences across 2 files
- **Re-exported**: Yes, via `src/services/mod.rs:13`

### 5. DpsAuthApiError
- **Current Location**: `src/dps_auth_api.rs` (lines 31-58)
- **Target Location**: Keep in `src/dps_auth_api.rs` (public API)
- **Domain**: API Configuration
- **Usage Count**: 1 occurrence in lib.rs (re-export)
- **Decision**: Do not migrate - part of public library API

## Migration Strategy

### Phase 1: SessionError
Create `src/types/session/session_error.rs` and `src/types/session/mod.rs`:
- Move SessionError enum definition
- Move Display and Error trait implementations
- Move From<DpsAuthSessionError> implementation
- Update imports in all consuming files

### Phase 2: UserError
Create `src/types/user/user_error.rs`:
- Move UserError enum definition
- Move Display and Error trait implementations
- Move From<PasswordError> implementation
- Update imports in all consuming files

### Phase 3: SiteError
Create `src/types/site/site_error.rs` and `src/types/site/mod.rs`:
- Move SiteError enum definition
- Move Display and Error trait implementations
- Move From<sqlx::Error> implementation
- Update imports in all consuming files

### Phase 4: PasswordError
Create `src/types/password/password_error.rs` and `src/types/password/mod.rs`:
- Move PasswordError enum definition
- Move Display and Error trait implementations
- Update imports in all consuming files

## Files to Create

### Phase 1: SessionError

#### `src/types/session/session_error.rs`
```rust
use dps_auth_session::DpsAuthSessionError;
use std::fmt;

/// Custom error type for session operations
#[derive(Debug)]
pub enum SessionError {
  /// Token encoding/decoding errors from the auth session service
  AuthSessionError(DpsAuthSessionError),
  /// Authentication failed - user input validation
  AuthenticationError(String),
  /// Database operation failed during authentication
  DatabaseError(String),
  /// Password verification failed
  PasswordVerificationError(String),
}

impl fmt::Display for SessionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SessionError::AuthSessionError(err) => write!(f, "Auth session error: {err}"),
      SessionError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SessionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      SessionError::PasswordVerificationError(msg) => {
        write!(f, "Password verification error: {msg}")
      }
    }
  }
}

impl From<DpsAuthSessionError> for SessionError {
  fn from(err: DpsAuthSessionError) -> Self {
    SessionError::AuthSessionError(err)
  }
}

impl std::error::Error for SessionError {}
```

#### `src/types/session/mod.rs`
```rust
pub mod session_error;

pub use session_error::SessionError;
```

### Phase 2: UserError

#### `src/types/user/user_error.rs`
```rust
use crate::services::PasswordError;
use sqlx::SqliteConnection;
use std::fmt;

/// Custom error type for user operations
#[derive(Debug)]
pub enum UserError {
  /// Username is already in use
  UsernameAlreadyExists(String),
  /// Password hashing failed
  PasswordHashingFailed(PasswordError),
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Input validation failed
  ValidationError(String),
  /// Authentication failed
  AuthenticationError(String),
  /// Authorization failed
  AuthorizationError(String),
  /// User not found
  UserNotFound(i64),
  /// Self-deletion attempted
  SelfDeletion,
  /// Session creation failed
  SessionError(String),
}

impl std::fmt::Display for UserError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserError::UsernameAlreadyExists(username) => {
        write!(f, "Username '{username}' is already in use")
      }
      UserError::PasswordHashingFailed(err) => write!(f, "Password hashing failed: {err}"),
      UserError::DatabaseError(err) => write!(f, "Database error: {err}"),
      UserError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      UserError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      UserError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
      UserError::UserNotFound(user_id) => write!(f, "User with ID {user_id} not found"),
      UserError::SelfDeletion => write!(f, "Cannot delete your own account"),
      UserError::SessionError(msg) => write!(f, "Session creation failed: {msg}"),
    }
  }
}

impl From<PasswordError> for UserError {
  fn from(err: PasswordError) -> Self {
    UserError::PasswordHashingFailed(err)
  }
}

impl std::error::Error for UserError {}

impl From<sqlx::Error> for UserError {
  fn from(err: sqlx::Error) -> Self {
    UserError::DatabaseError(err)
  }
}
```

#### `src/types/user/mod.rs` (update)
```rust
pub mod user_error;
pub mod update_user_input;

pub use user_error::UserError;
pub use update_user_input::UpdateUserInput;
```

### Phase 3: SiteError

#### `src/types/site/site_error.rs`
```rust
use sqlx::SqliteConnection;
use std::fmt;

/// Custom error type for site operations
#[derive(Debug)]
pub enum SiteError {
  /// Slug is already in use
  SlugAlreadyExists(String),
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Input validation failed
  ValidationError(String),
  /// Site not found
  SiteNotFound(i64),
  /// Authentication failed
  AuthenticationError(String),
  /// Authorization failed
  AuthorizationError(String),
}

impl std::fmt::Display for SiteError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SiteError::SlugAlreadyExists(slug) => {
        write!(f, "Slug '{slug}' is already in use")
      }
      SiteError::DatabaseError(err) => write!(f, "Database error: {err}"),
      SiteError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      SiteError::SiteNotFound(id) => write!(f, "Site with ID {id} not found"),
      SiteError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SiteError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
    }
  }
}

impl std::error::Error for SiteError {}

impl From<sqlx::Error> for SiteError {
  fn from(err: sqlx::Error) -> Self {
    SiteError::DatabaseError(err)
  }
}
```

#### `src/types/site/mod.rs`
```rust
pub mod site_error;

pub use site_error::SiteError;
```

### Phase 4: PasswordError

#### `src/types/password/password_error.rs`
```rust
use std::fmt;

/// Custom error type for password operations
#[derive(Debug)]
pub enum PasswordError {
  /// Error occurred during password hashing
  HashingError(String),
  /// Error occurred during password verification
  VerificationError(String),
  /// Invalid hash format provided
  InvalidHash(String),
}

impl fmt::Display for PasswordError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PasswordError::HashingError(msg) => write!(f, "Password hashing error: {msg}"),
      PasswordError::VerificationError(msg) => write!(f, "Password verification error: {msg}"),
      PasswordError::InvalidHash(msg) => write!(f, "Invalid hash format: {msg}"),
    }
  }
}

impl std::error::Error for PasswordError {}
```

#### `src/types/password/mod.rs`
```rust
pub mod password_error;

pub use password_error::PasswordError;
```

## Files to Modify

### `src/types/mod.rs`
```rust
pub mod password;
pub mod role;
pub mod session;
pub mod site;
pub mod user;

pub use password::PasswordError;
pub use role::RoleError;
pub use session::SessionError;
pub use site::SiteError;
pub use user::{UserError, UpdateUserInput};
```

### `src/services/session_service.rs`
Remove SessionError enum definition (lines 12-42) and add import:
```rust
use crate::models::user::User;
use crate::services::{PasswordService, UserService};
use crate::types::SessionError;
use dps_auth_session::{DpsAuthSession, DpsAuthSessionError};
use sqlx::SqliteConnection;
use std::fmt;

// Re-export the session payload from the new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;
```

### `src/services/user_service.rs`
Remove UserError enum definition (lines 12-68) and add import:
```rust
use crate::models::User;
use crate::queries::users::{
  CreateUserData, CreateUserQuery, DeleteUserByIdQuery, GetUserByIdQuery, GetUserByNameQuery,
  UpdateUserData, UpdateUserPasswordData, UpdateUserPasswordQuery, UpdateUserQuery,
};
use crate::services::{PasswordError, PasswordService};
use crate::types::UserError;
use sqlx::SqliteConnection;
use uuid::Uuid;
```

### `src/services/site_service.rs`
Remove SiteError enum definition (lines 10-46) and add import:
```rust
use crate::models::Site;
use crate::queries::sites::{
  CreateSiteData, CreateSiteQuery, DeleteSiteQuery, GetAllSitesQuery, UpdateSiteData,
  UpdateSiteQuery,
};
use crate::types::SiteError;
use sqlx::SqliteConnection;
```

### `src/services/password_service.rs`
Remove PasswordError enum definition (lines 10-29) and add import:
```rust
use argon2::{
  password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
  Argon2, Params,
};
use bcrypt;
use crate::types::PasswordError;
use std::fmt;
```

### `src/services/mod.rs`
Update re-exports to import from types module:
```rust
pub mod auth_service;
pub mod cookie_service;
pub mod password_service;
pub mod role_service;
pub mod server_service;
pub mod session_service;
pub mod shutdown_service;
pub mod site_service;
pub mod user_service;

pub use auth_service::{AuthMeResult, AuthResult, AuthService, RegisterResult};
pub use cookie_service::CookieService;
pub use password_service::PasswordService;
pub use server_service::ServerService;
pub use session_service::SessionService;
// Re-export SessionPayload from new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;
pub use role_service::RoleService;
pub use site_service::SiteService;
pub use user_service::UserService;

// Re-export error types from types module for convenience
pub use crate::types::PasswordError;
pub use crate::types::RoleError;
pub use crate::types::SessionError;
pub use crate::types::SiteError;
pub use crate::types::UserError;
```

### Import Updates

#### SessionError (19 occurrences in 6 files)
- `src/graphql/resolvers/auth_login.rs` - Update `use crate::services::{AuthService, CookieService, SessionError};`
- `src/graphql/resolvers/auth_me.rs` - Update `use crate::services::SessionError;`
- `src/orchestrators/auth_orchestrator.rs` - Update `use crate::services::{AuthService, SessionError, UserError, UserService};`
- `src/services/auth_service.rs` - Update `use crate::services::{SessionError, SessionService, UserError, UserService};`

#### UserError (12 occurrences in 10 files)
- `src/orchestrators/user_orchestrator.rs` - Update `use crate::services::{RoleService, UserError, UserService};`
- `src/graphql/resolvers/auth_register.rs` - Update `use crate::services::{AuthService, CookieService, UserError};`
- `src/graphql/resolvers/update_user.rs` - Update `use crate::services::UserError;`
- `src/orchestrators/auth_orchestrator.rs` - Update `use crate::services::{AuthService, SessionError, UserError, UserService};`
- `src/services/auth_service.rs` - Update `use crate::services::{SessionError, SessionService, UserError, UserService};`
- `src/graphql/resolvers/auth_change_password.rs` - Update `use crate::services::UserError;`
- `src/graphql/resolvers/users.rs` - Update `use crate::services::UserError;`
- `src/graphql/resolvers/user.rs` - Update `use crate::services::UserError;`
- `src/graphql/resolvers/delete_user.rs` - Update `use crate::services::UserError;`

#### SiteError (6 occurrences in 6 files)
- `src/orchestrators/site_orchestrator.rs` - Update `use crate::services::{RoleService, SiteError, SiteService};`
- `src/graphql/resolvers/site.rs` - Update `use crate::services::SiteError;`
- `src/graphql/resolvers/update_site.rs` - Update `use crate::services::SiteError;`
- `src/graphql/resolvers/remove_site.rs` - Update `use crate::services::SiteError;`
- `src/graphql/resolvers/add_site.rs` - Update `use crate::services::SiteError;`

#### PasswordError (2 occurrences in 2 files)
- `src/services/user_service.rs` - Update `use crate::services::{PasswordError, PasswordService};`

## Implementation Order

Implement migrations in this order, with full testing after each phase:

1. **Phase 1: SessionError** - Most complex due to external dependency
2. **Phase 2: UserError** - Dependent on PasswordError, but has circular dependency
3. **Phase 3: SiteError** - Independent, straightforward
4. **Phase 4: PasswordError** - Simplest, no external dependencies

**Note**: UserError references PasswordError, so Phase 2 should come after Phase 4, or we need to use a forward declaration.

## Testing

After each phase is complete:
- Run `cargo test --quiet` to ensure all tests pass
- Run `cargo clippy --allow-dirty --fix && cargo fmt` to ensure code quality

## Summary of Changes

- **4 new domain directories**: session, site, password, user (extended)
- **4 new error files**: session_error.rs, site_error.rs, password_error.rs, user_error.rs
- **4 new mod.rs files**: session/mod.rs, site/mod.rs, password/mod.rs, user/mod.rs (updated)
- **4 service files modified**: session_service.rs, user_service.rs, site_service.rs, password_service.rs
- **1 types/mod.rs updated** to export all error types
- **1 services/mod.rs updated** to re-export from types module
- **33+ import statements updated** across 6 GraphQL resolvers and 3 orchestrators

## Benefits

1. **Separation of Concerns**: Error types are domain types, not service implementation details
2. **Consistency**: All error types follow the same pattern in the types module
3. **Maintainability**: Easier to find and update error definitions
4. **Reusability**: Error types can be used by multiple services more naturally
5. **Clear Organization**: Types module becomes the single source of truth for all domain error types
