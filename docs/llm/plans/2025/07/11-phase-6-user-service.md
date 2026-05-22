# Phase 6: User Service - User Creation and Retrieval

**Date**: 2025-07-11@13:58  
**Phase**: 6 - User Service Implementation  
**Goal**: Implement UserService for comprehensive user management with creation and retrieval capabilities

## Overview

This phase implements the `UserService` following the established project patterns. The service will provide methods for creating new users with validation, and retrieving users by ID and name. The implementation leverages existing infrastructure including `PasswordService` for secure password hashing, database queries for data access, and the permissions system for role assignment.

## Requirements Analysis

Based on the README specifications and existing codebase:

- Implement `UserService` with static methods following existing service patterns
- `create_user` method with input validation and username uniqueness checks
- `get_user_by_id` method delegating to existing query
- `get_user_by_name` method requiring new query implementation
- Integration with `PasswordService` for password hashing
- Default role assignment using existing role system
- Comprehensive unit tests with full coverage
- Store service in `src/services/user_service.rs`

### Current State Analysis

**Existing Infrastructure:**
- `PasswordService` with `generate()` and `verify()` methods ✅
- `CreateUserQuery` with comprehensive validation and default role support ✅
- `GetUserByIdQuery` for user retrieval by ID ✅
- `GetUserByUuidQuery` for user retrieval by UUID ✅
- `UserRoleService` with role management and permission checking ✅
- User model with all required fields ✅

**Missing Components:**
- `GetUserByNameQuery` for user retrieval by name ❌
- `UserService` implementation ❌
- Integration tests for complete user workflows ❌

## Tasks Breakdown

### 1. Implement GetUserByNameQuery

Create `src/queries/users/get_user_by_name.rs`:

```rust
use sqlx::SqlitePool;
use crate::models::User;

pub struct GetUserByNameQuery;

impl GetUserByNameQuery {
  pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE name = ? COLLATE NOCASE"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
  }
}
```

**Key Features:**
- Case-insensitive name matching using `COLLATE NOCASE`
- Returns `Option<User>` for consistency with other query patterns
- Comprehensive unit tests covering found/not found scenarios

### 2. Update Module Exports

Update `src/queries/users/mod.rs`:

```rust
pub mod create_user;
pub mod get_user_by_id;
pub mod get_user_by_uuid;
pub mod get_user_by_name;

pub use create_user::{CreateUserQuery, CreateUserData};
pub use get_user_by_id::GetUserByIdQuery;
pub use get_user_by_uuid::GetUserByUuidQuery;
pub use get_user_by_name::GetUserByNameQuery;
```

### 3. Implement UserService

Create `src/services/user_service.rs`:

```rust
use crate::models::User;
use crate::queries::users::{CreateUserQuery, CreateUserData, GetUserByIdQuery, GetUserByNameQuery};
use crate::services::{PasswordService, PasswordError};
use sqlx::SqlitePool;
use uuid::Uuid;

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
}

impl std::fmt::Display for UserError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserError::UsernameAlreadyExists(username) => write!(f, "Username '{}' is already in use", username),
      UserError::PasswordHashingFailed(err) => write!(f, "Password hashing failed: {}", err),
      UserError::DatabaseError(err) => write!(f, "Database error: {}", err),
      UserError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
    }
  }
}

impl std::error::Error for UserError {}

impl From<PasswordError> for UserError {
  fn from(err: PasswordError) -> Self {
    UserError::PasswordHashingFailed(err)
  }
}

impl From<sqlx::Error> for UserError {
  fn from(err: sqlx::Error) -> Self {
    UserError::DatabaseError(err)
  }
}

/// Service for user management operations
pub struct UserService;

impl UserService {
  /// Create a new user with validation and default role assignment
  /// 
  /// This method validates the input (username and password), checks for username
  /// uniqueness, hashes the password using PasswordService, and assigns the default
  /// role to the user.
  /// 
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `username` - The username for the new user (must be unique)
  /// * `password` - The plain text password (will be hashed)
  /// 
  /// # Returns
  /// * `Ok(User)` - Successfully created user
  /// * `Err(UserError)` - Creation failed due to validation, uniqueness, or database error
  pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
  ) -> Result<User, UserError> {
    // Input validation
    Self::validate_username(username)?;
    Self::validate_password(password)?;
    
    // Check if username already exists
    if let Some(_existing_user) = GetUserByNameQuery::run(pool, username).await? {
      return Err(UserError::UsernameAlreadyExists(username.to_string()));
    }
    
    // Hash the password
    let password_hash = PasswordService::generate(password)?;
    
    // Generate UUID for the user
    let user_uuid = Uuid::new_v4().to_string();
    
    // Create user data
    let create_data = CreateUserData {
      uuid: user_uuid,
      name: username.to_string(),
      role_id: None, // Use default role
      password_hash,
      metadata_json: None,
    };
    
    // Create the user
    let user = CreateUserQuery::run(pool, create_data).await?;
    
    Ok(user)
  }
  
  /// Retrieve a user by ID
  /// 
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to retrieve
  /// 
  /// # Returns
  /// * `Ok(Some(User))` - User found
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn get_user_by_id(pool: &SqlitePool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    GetUserByIdQuery::run(pool, user_id).await
  }
  
  /// Retrieve a user by name (case-insensitive)
  /// 
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `name` - The name of the user to retrieve
  /// 
  /// # Returns
  /// * `Ok(Some(User))` - User found
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn get_user_by_name(pool: &SqlitePool, name: &str) -> Result<Option<User>, sqlx::Error> {
    GetUserByNameQuery::run(pool, name).await
  }
  
  /// Validate username according to business rules
  fn validate_username(username: &str) -> Result<(), UserError> {
    if username.trim().is_empty() {
      return Err(UserError::ValidationError("Username cannot be empty".to_string()));
    }
    
    if username.len() < 3 {
      return Err(UserError::ValidationError("Username must be at least 3 characters long".to_string()));
    }
    
    if username.len() > 20 {
      return Err(UserError::ValidationError("Username cannot be longer than 20 characters".to_string()));
    }
    
    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
      return Err(UserError::ValidationError("Username can only contain letters, numbers, underscores, and hyphens".to_string()));
    }
    
    Ok(())
  }
  
  /// Validate password according to business rules
  fn validate_password(password: &str) -> Result<(), UserError> {
    if password.is_empty() {
      return Err(UserError::ValidationError("Password cannot be empty".to_string()));
    }
    
    if password.len() < 6 {
      return Err(UserError::ValidationError("Password must be at least 6 characters long".to_string()));
    }
    
    if password.len() > 128 {
      return Err(UserError::ValidationError("Password cannot be longer than 128 characters".to_string()));
    }
    
    Ok(())
  }
}
```

**Key Features:**
- Comprehensive input validation for username and password
- Username uniqueness checking before creation
- Integration with `PasswordService` for secure password hashing
- Automatic UUID generation for new users
- Default role assignment via existing `CreateUserQuery`
- Custom error types with proper error handling and conversion
- Delegation to existing query objects for data access
- Follows established service patterns (static methods, comprehensive documentation)

### 4. Update Service Module Exports

Update `src/services/mod.rs`:

```rust
pub mod server_service;
pub mod password_service;
pub mod user_role_service;
pub mod user_service;

pub use server_service::ServerService;
pub use password_service::{PasswordService, PasswordError};
pub use user_role_service::UserRoleService;
pub use user_service::{UserService, UserError};
```

### 5. Add Required Dependencies

Update `Cargo.toml` to ensure `uuid` dependency is available:

```toml
[dependencies]
# Existing dependencies...
uuid = { version = "1.0", features = ["v4"] }
```

## Testing Strategy

### 1. GetUserByNameQuery Tests

- Test successful user retrieval by name
- Test case-insensitive name matching
- Test user not found scenario
- Test with special characters in names

### 2. UserService Unit Tests

**create_user method:**
- Test successful user creation with valid inputs
- Test username validation (empty, too short, too long, invalid characters)
- Test password validation (empty, too short, too long)
- Test username uniqueness enforcement
- Test password hashing integration
- Test default role assignment
- Test database error handling

**get_user_by_id method:**
- Test delegation to GetUserByIdQuery
- Test user found and not found scenarios

**get_user_by_name method:**
- Test delegation to GetUserByNameQuery
- Test case-insensitive retrieval
- Test user found and not found scenarios

### 3. Integration Tests

Create comprehensive integration tests demonstrating:
- Complete user creation workflow
- User retrieval by different methods
- Username uniqueness enforcement across service calls
- Password verification workflow (create user, then verify password)

## Files to be Created/Modified

### New Files:
1. `src/queries/users/get_user_by_name.rs` - New query for user retrieval by name
2. `src/services/user_service.rs` - Main UserService implementation

### Modified Files:
1. `src/queries/users/mod.rs` - Add GetUserByNameQuery export
2. `src/services/mod.rs` - Add UserService export
3. `Cargo.toml` - Ensure uuid dependency (likely already present)

## Code Samples

### GetUserByNameQuery Implementation

```rust
use sqlx::SqlitePool;
use crate::models::User;

pub struct GetUserByNameQuery;

impl GetUserByNameQuery {
  pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE name = ? COLLATE NOCASE"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use uuid::Uuid;

  #[tokio::test]
  async fn test_get_user_by_name_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test role first
    let role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = role_result.last_insert_rowid();
    
    // Insert test user
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'TestUser', ?, 'hashed_password', NULL)"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();
    
    let user = GetUserByNameQuery::run(&pool, "TestUser").await.unwrap();
    
    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "TestUser");
  }

  #[tokio::test]
  async fn test_get_user_by_name_case_insensitive() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test role first
    let role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = role_result.last_insert_rowid();
    
    // Insert test user with mixed case
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (?, 1234567890, 1234567890, 'TestUser', ?, 'hashed_password', NULL)"
    )
    .bind(&user_uuid)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();
    
    // Should find user regardless of case
    let user_lower = GetUserByNameQuery::run(&pool, "testuser").await.unwrap();
    let user_upper = GetUserByNameQuery::run(&pool, "TESTUSER").await.unwrap();
    let user_mixed = GetUserByNameQuery::run(&pool, "tEsTuSeR").await.unwrap();
    
    assert!(user_lower.is_some());
    assert!(user_upper.is_some());
    assert!(user_mixed.is_some());
    
    assert_eq!(user_lower.unwrap().name, "TestUser");
    assert_eq!(user_upper.unwrap().name, "TestUser");
    assert_eq!(user_mixed.unwrap().name, "TestUser");
  }

  #[tokio::test]
  async fn test_get_user_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    let user = GetUserByNameQuery::run(&pool, "NonExistentUser").await.unwrap();
    
    assert!(user.is_none());
  }
}
```

### UserService Key Methods

```rust
impl UserService {
  pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
  ) -> Result<User, UserError> {
    // Input validation
    Self::validate_username(username)?;
    Self::validate_password(password)?;
    
    // Check uniqueness
    if let Some(_existing_user) = GetUserByNameQuery::run(pool, username).await? {
      return Err(UserError::UsernameAlreadyExists(username.to_string()));
    }
    
    // Hash password and create user
    let password_hash = PasswordService::generate(password)?;
    let user_uuid = Uuid::new_v4().to_string();
    
    let create_data = CreateUserData {
      uuid: user_uuid,
      name: username.to_string(),
      role_id: None, // Use default role
      password_hash,
      metadata_json: None,
    };
    
    CreateUserQuery::run(pool, create_data).await.map_err(UserError::from)
  }
  
  pub async fn get_user_by_id(pool: &SqlitePool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    GetUserByIdQuery::run(pool, user_id).await
  }
  
  pub async fn get_user_by_name(pool: &SqlitePool, name: &str) -> Result<Option<User>, sqlx::Error> {
    GetUserByNameQuery::run(pool, name).await
  }
}
```

## Success Criteria

1. **GetUserByNameQuery Implementation**
   - ✅ Case-insensitive user retrieval by name
   - ✅ Comprehensive unit tests with 100% coverage
   - ✅ Proper error handling for database operations

2. **UserService Implementation**
   - ✅ `create_user` method with full validation and uniqueness checking
   - ✅ `get_user_by_id` method delegating to existing query
   - ✅ `get_user_by_name` method delegating to new query
   - ✅ Custom error types with proper error handling
   - ✅ Comprehensive unit tests with 100% coverage

3. **Integration and Testing**
   - ✅ All existing tests continue to pass
   - ✅ New functionality thoroughly tested
   - ✅ Integration tests demonstrate complete workflows
   - ✅ Code follows established project patterns

4. **Documentation and Maintenance**
   - ✅ Comprehensive Rust doc comments for all public methods
   - ✅ Clear error messages and proper error propagation
   - ✅ Code follows project formatting and style guidelines

## Future Considerations

This implementation provides the foundation for future user management features:

- **User Authentication**: Password verification workflows
- **User Updates**: Methods for updating user information
- **User Deletion**: Soft delete or hard delete capabilities
- **User Search**: Advanced search and filtering capabilities
- **User Roles**: Dynamic role assignment and management
- **User Metadata**: Extended user profile information

The service architecture supports easy extension while maintaining the established patterns and testing standards.