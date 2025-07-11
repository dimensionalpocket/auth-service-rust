# Plan: Add is_default Column to UserRole Model

**Date**: 2025-07-11@12:33  
**Task**: Add `is_default` boolean column to UserRole model/struct with migration updates, seed changes, and comprehensive test updates.

## Overview

This plan adds a new `is_default` boolean column to the `user_roles` table and corresponding UserRole struct. The column will be NOT NULL with a default value of false, and the 'user' role will be marked as the default role in the seeds.

## Changes Required

### 1. Database Migration Update

**File**: `config/database/migrations/001_create_user_roles.sql`

**Current**:
```sql
-- Create user_roles table
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL
);
```

**Updated**:
```sql
-- Create user_roles table
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE
);
```

### 2. UserRole Model Update

**File**: `src/models/user_role.rs`

**Current**:
```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
}
```

**Updated**:
```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
  pub is_default: bool,
}
```

### 3. Seeds Update

**File**: `config/database/seeds/001_default_roles.sql`

**Current**:
```sql
-- Insert default user roles
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO user_roles (name, created_ts) VALUES 
    ('admin', strftime('%s', 'now')),
    ('user', strftime('%s', 'now'));
```

**Updated**:
```sql
-- Insert default user roles
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO user_roles (name, created_ts, is_default) VALUES 
    ('admin', strftime('%s', 'now'), FALSE),
    ('user', strftime('%s', 'now'), TRUE);
```

### 4. Query Updates

All UserRole queries need to be updated to include the new `is_default` column in their SELECT statements:

#### 4.1 GetAllRolesQuery

**File**: `src/queries/user_roles/get_all_roles.rs`

**Current SELECT**:
```sql
SELECT id, name, created_ts FROM user_roles ORDER BY name
```

**Updated SELECT**:
```sql
SELECT id, name, created_ts, is_default FROM user_roles ORDER BY name
```

#### 4.2 GetRoleByIdQuery

**File**: `src/queries/user_roles/get_role_by_id.rs`

**Current SELECT**:
```sql
SELECT id, name, created_ts FROM user_roles WHERE id = ?
```

**Updated SELECT**:
```sql
SELECT id, name, created_ts, is_default FROM user_roles WHERE id = ?
```

#### 4.3 GetRoleByNameQuery

**File**: `src/queries/user_roles/get_role_by_name.rs`

**Current SELECT**:
```sql
SELECT id, name, created_ts FROM user_roles WHERE name = ?
```

**Updated SELECT**:
```sql
SELECT id, name, created_ts, is_default FROM user_roles WHERE name = ?
```

#### 4.4 GetDefaultUserRoleQuery (New)

**File**: `src/queries/user_roles/get_default_user_role.rs` (new file)

This new query will retrieve the default user role (where `is_default = TRUE`).

```rust
use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetDefaultUserRoleQuery;

impl GetDefaultUserRoleQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Option<UserRole>, sqlx::Error> {
    sqlx::query_as::<_, UserRole>(
      "SELECT id, name, created_ts, is_default FROM user_roles WHERE is_default = TRUE LIMIT 1"
    )
    .fetch_optional(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_default_user_role_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test roles with one default
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)")
      .execute(&pool)
      .await
      .unwrap();
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert_eq!(role.is_default, true);
  }

  #[tokio::test]
  async fn test_get_default_user_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test roles with no default
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('moderator', 1234567891, FALSE)")
      .execute(&pool)
      .await
      .unwrap();
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_default_user_role_empty_table() {
    let (pool, _temp_file) = create_test_database().await;
    
    let role = GetDefaultUserRoleQuery::run(&pool).await.unwrap();
    
    assert!(role.is_none());
  }
}
```

**Module Export Update**: `src/queries/user_roles/mod.rs`

```rust
pub mod get_all_roles;
pub mod get_role_by_id;
pub mod get_role_by_name;
pub mod get_default_user_role;

pub use get_all_roles::GetAllRolesQuery;
pub use get_role_by_id::GetRoleByIdQuery;
pub use get_role_by_name::GetRoleByNameQuery;
pub use get_default_user_role::GetDefaultUserRoleQuery;
```

### 5. CreateUserQuery Enhancement

**File**: `src/queries/users/create_user.rs`

Update the `CreateUserData` struct to make `role_id` optional and modify the `CreateUserQuery` to use the default role when not specified.

#### 5.1 Updated CreateUserData Struct

**Current**:
```rust
#[derive(Debug)]
pub struct CreateUserData {
  pub uuid: String,
  pub name: String,
  pub role_id: i64,
  pub password_hash: String,
  pub metadata_json: Option<String>,
}
```

**Updated**:
```rust
#[derive(Debug)]
pub struct CreateUserData {
  pub uuid: String,
  pub name: String,
  pub role_id: Option<i64>,
  pub password_hash: String,
  pub metadata_json: Option<String>,
}
```

#### 5.2 Updated CreateUserQuery Implementation

**Current**:
```rust
impl CreateUserQuery {
  pub async fn run(pool: &SqlitePool, data: CreateUserData) -> Result<User, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    
    let result = sqlx::query(
      r#"
      INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json)
      VALUES (?, ?, ?, ?, ?, ?, ?)
      "#
    )
    .bind(&data.uuid)
    .bind(now)
    .bind(now)
    .bind(&data.name)
    .bind(data.role_id)
    .bind(&data.password_hash)
    .bind(&data.metadata_json)
    .execute(pool)
    .await?;
    
    let user_id = result.last_insert_rowid();
    
    // Return the created user
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
  }
}
```

**Updated**:
```rust
use crate::queries::user_roles::GetDefaultUserRoleQuery;

impl CreateUserQuery {
  pub async fn run(pool: &SqlitePool, data: CreateUserData) -> Result<User, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    
    // Determine the role_id to use
    let role_id = match data.role_id {
      Some(id) => id,
      None => {
        // Get the default role
        let default_role = GetDefaultUserRoleQuery::run(pool).await?;
        match default_role {
          Some(role) => role.id,
          None => {
            // Return a custom error if no default role exists
            return Err(sqlx::Error::RowNotFound);
          }
        }
      }
    };
    
    let result = sqlx::query(
      r#"
      INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json)
      VALUES (?, ?, ?, ?, ?, ?, ?)
      "#
    )
    .bind(&data.uuid)
    .bind(now)
    .bind(now)
    .bind(&data.name)
    .bind(role_id)
    .bind(&data.password_hash)
    .bind(&data.metadata_json)
    .execute(pool)
    .await?;
    
    let user_id = result.last_insert_rowid();
    
    // Return the created user
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
  }
}
```

#### 5.3 Updated Tests for CreateUserQuery

Add new tests to verify the default role functionality:

```rust
#[tokio::test]
async fn test_create_user_with_default_role() {
  let (pool, _temp_file) = create_test_database().await;
  
  // Insert test roles with one default
  sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)")
    .execute(&pool)
    .await
    .unwrap();
  
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid.clone(),
    name: "Test User".to_string(),
    role_id: None, // Use default role
    password_hash: "hashed_password".to_string(),
    metadata_json: Some(r#"{"test": true}"#.to_string()),
  };
  
  let user = CreateUserQuery::run(&pool, create_data).await.unwrap();
  
  assert_eq!(user.uuid, user_uuid);
  assert_eq!(user.name, "Test User");
  // Should have the default role (user role)
  let default_role = GetDefaultUserRoleQuery::run(&pool).await.unwrap().unwrap();
  assert_eq!(user.role_id, default_role.id);
}

#[tokio::test]
async fn test_create_user_with_explicit_role() {
  let (pool, _temp_file) = create_test_database().await;
  
  // Insert test roles with one default
  let admin_result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE)")
    .execute(&pool)
    .await
    .unwrap();
  let admin_role_id = admin_result.last_insert_rowid();
  
  sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567891, TRUE)")
    .execute(&pool)
    .await
    .unwrap();
  
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid.clone(),
    name: "Test Admin".to_string(),
    role_id: Some(admin_role_id), // Explicitly specify admin role
    password_hash: "hashed_password".to_string(),
    metadata_json: None,
  };
  
  let user = CreateUserQuery::run(&pool, create_data).await.unwrap();
  
  assert_eq!(user.uuid, user_uuid);
  assert_eq!(user.name, "Test Admin");
  assert_eq!(user.role_id, admin_role_id); // Should have admin role, not default
}

#[tokio::test]
async fn test_create_user_no_default_role_fails() {
  let (pool, _temp_file) = create_test_database().await;
  
  // Insert test roles with no default
  sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('moderator', 1234567891, FALSE)")
    .execute(&pool)
    .await
    .unwrap();
  
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid,
    name: "Test User".to_string(),
    role_id: None, // Try to use default role, but none exists
    password_hash: "hashed_password".to_string(),
    metadata_json: None,
  };
  
  let result = CreateUserQuery::run(&pool, create_data).await;
  assert!(result.is_err()); // Should fail because no default role exists
}
```

### 6. Test Updates

All test files that insert test data into the `user_roles` table need to be updated to include the `is_default` column:

#### 5.1 UserRole Query Tests

**Files to update**:
- `src/queries/user_roles/get_all_roles.rs` (test functions)
- `src/queries/user_roles/get_role_by_id.rs` (test functions)
- `src/queries/user_roles/get_role_by_name.rs` (test functions)

**Current test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts) VALUES ('admin', 1234567890)
```

**Updated test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE)
```

#### 5.2 User Query Tests

**Files to update**:
- `src/queries/users/create_user.rs` (test functions)
- `src/queries/users/get_user_by_id.rs` (test functions)
- `src/queries/users/get_user_by_uuid.rs` (test functions)

**Current test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts) VALUES ('user', 1234567890)
```

**Updated test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)
```

**Note**: Existing CreateUserQuery tests will also need to be updated to use `Some(role_id)` instead of `role_id` directly in the `CreateUserData` struct.

#### 5.3 Integration Tests

**File**: `tests/database_integration_tests.rs`

**Current test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts) VALUES ('admin', 1234567890), ('user', 1234567891)
```

**Updated test INSERT pattern**:
```sql
INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)
```

#### 5.4 New Test Assertions

Add test assertions to verify the `is_default` field is correctly set:

**In GetAllRolesQuery tests**:
```rust
assert_eq!(roles[0].is_default, false); // admin role
assert_eq!(roles[1].is_default, true);  // user role
```

**In GetRoleByIdQuery and GetRoleByNameQuery tests**:
```rust
assert_eq!(role.is_default, false); // or true depending on test data
```

### 6. Schema Dump Update

**File**: `config/database/schema.sql`

The schema dump will be automatically updated when migrations are run, but the expected change is:

**Current**:
```sql
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL
);
```

**Updated**:
```sql
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE
);
```

## Implementation Steps

1. **Update Migration**: Modify `001_create_user_roles.sql` to include the `is_default` column
2. **Update Model**: Add `is_default` field to the `UserRole` struct
3. **Update Seeds**: Modify seeds to set `is_default = TRUE` for the 'user' role
4. **Update Existing Queries**: Update all three existing UserRole query objects to include `is_default` in SELECT statements
5. **Create New Query**: Implement `GetDefaultUserRoleQuery` with comprehensive tests
6. **Update Module Exports**: Add the new query to `src/queries/user_roles/mod.rs`
7. **Update Tests**: Update all test INSERT statements to include `is_default` values
8. **Add Test Assertions**: Add assertions to verify `is_default` field behavior
9. **Run Tests**: Execute full test suite to validate all changes work correctly
10. **Update Schema**: Run migrations to update the schema dump

## Files to be Modified

1. `config/database/migrations/001_create_user_roles.sql`
2. `config/database/seeds/001_default_roles.sql`
3. `src/models/user_role.rs`
4. `src/queries/user_roles/get_all_roles.rs`
5. `src/queries/user_roles/get_role_by_id.rs`
6. `src/queries/user_roles/get_role_by_name.rs`
7. `src/queries/user_roles/get_default_user_role.rs` (new file)
8. `src/queries/user_roles/mod.rs` (to export new query)
9. `src/queries/users/create_user.rs` (major changes: optional role_id + default role logic)
10. `src/queries/users/get_user_by_id.rs`
11. `src/queries/users/get_user_by_uuid.rs`
12. `tests/database_integration_tests.rs`
13. `config/database/schema.sql` (automatically updated by migration)

## Expected Test Results

After implementation, all existing tests should continue to pass, and the new `is_default` field should be properly populated:

- Admin role: `is_default = false`
- User role: `is_default = true`
- All UserRole query objects should return the `is_default` field
- All tests should verify the correct `is_default` values

## Validation

The implementation will be validated by running the complete test suite:
```bash
mise exec -- cargo test
```

All tests must pass to confirm the changes are working correctly and no regressions have been introduced.