# Plan: Refactor Permissions from JSON to String Array in Query Objects

## Overview

This plan refactored the permissions handling from the current `permissions()` struct method to direct string array handling in query objects. The `permissions()` method was removed, and all queries that handle permissions (both as input and output) were updated to work with string arrays directly. JSON conversions happen inside the query methods.

## Status: ✅ **COMPLETE**

## Current State Analysis

- The `Role` model has a `permissions_json: Option<String>` field that stores JSON in the database
- The `Role` struct has a `permissions()` method that deserializes JSON to `Vec<String>`
- Query objects like `CreateRoleQuery` and `UpdateRoleQuery` manually serialize/deserialize JSON
- Services and resolvers call `role.permissions()` to get permissions as a string array

## Implementation Plan

### Single Implementation Phase

**Note**: This refactoring must be implemented all at once since the model change will break all dependent layers. Tests should only be run after all changes are complete.

#### Step 1: Update Role Model Structure

**File: `src/models/role.rs`**

1. **Replace `permissions_json: Option<String>` with `permissions: Vec<String>`** in the `Role` struct
2. **Remove the `permissions()` method** from the `Role` struct implementation (no longer needed)
3. **Keep the `has_permission()` method** but update it to work directly with the `permissions` field
4. **Update tests** to reflect the new structure and removed methods

#### Step 2: Update All Query Objects

**Files: `src/queries/roles/*.rs`**

1. **Update `GetRoleByIdQuery`** in `src/queries/roles/get_role_by_id.rs`:
   - Query database for `permissions_json` field
   - Deserialize JSON to `Vec<String>` and set `permissions` field on returned `Role`

2. **Update `GetAllRolesQuery`** in `src/queries/roles/get_all_roles.rs`:
   - Query database for `permissions_json` field
   - Deserialize JSON to `Vec<String>` and set `permissions` field on returned `Role`

3. **Update `GetRoleByNameQuery`** in `src/queries/roles/get_role_by_name.rs`:
   - Query database for `permissions_json` field
   - Deserialize JSON to `Vec<String>` and set `permissions` field on returned `Role`

4. **Update `CreateRoleQuery`** in `src/queries/roles/create_role.rs`:
   - Serialize `Vec<String>` to JSON for database storage
   - Set `permissions` field directly on returned `Role` struct

5. **Update `UpdateRoleQuery`** in `src/queries/roles/update_role.rs`:
   - Serialize `Option<Vec<String>>` to JSON for database storage
   - Set `permissions` field directly on returned `Role` struct

#### Step 3: Update Services Layer

**File: `src/services/role_service.rs`**

1. **Replace all calls to `role.permissions()` with direct access to `role.permissions`**
2. **Keep all calls to `role.has_permission()` unchanged**

#### Step 4: Update Orchestrators Layer

**File: `src/orchestrators/role_orchestrator.rs`**

1. **Replace all calls to `role.permissions()` with direct access to `role.permissions`**
2. **Keep all calls to `role.has_permission()` unchanged**
3. **Update tests** to use `role.permissions` directly

#### Step 5: Update GraphQL Resolvers

**Files: `src/graphql/resolvers/*.rs`**

1. **In all resolver files that reference permissions**:
   - Replace calls to `role.permissions()` with direct access to `role.permissions`
   - Keep calls to `role.has_permission()` unchanged

**Files to update:**
- `src/graphql/resolvers/role.rs`
- `src/graphql/resolvers/roles.rs`
- `src/graphql/resolvers/add_role.rs`
- `src/graphql/resolvers/update_role.rs`
- `src/graphql/resolvers/set_default_role.rs`
- Any other resolvers that access role permissions

#### Step 6: Update Test Utilities

**File: `src/test_utils/mod.rs`**

1. **Update `create_test_role()`** to use `role.permissions` directly
2. **Update any test helpers** that use `role.permissions()`

#### Step 7: Run Tests

**After all changes are complete:**

1. **Run all tests**: `cargo test --quiet` (runs both unit and integration tests)
2. **Fix any compilation errors** that arise from missed references
3. **Update any remaining test files** that use the old `role.permissions()` method
4. **Re-run tests** to verify all fixes work correctly

## Technical Implementation Details

### Updated Role Model

```rust
// In src/models/role.rs
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub created_ts: i64,
    pub updated_ts: i64,
    pub is_default: bool,
    pub permissions: Vec<String>,  // Changed from permissions_json: Option<String>
}

impl Role {
    /// Check if this role has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}
```

### Query Method Pattern

For query methods that fetch roles from database:

```rust
// In query files (e.g., get_role_by_id.rs)
impl GetRoleByIdQuery {
    pub async fn run(pool: &SqlitePool, role_id: i64) -> Result<Option<Role>, sqlx::Error> {
        // Query the database for permissions_json
        let row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json 
             FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            // Extract permissions_json and deserialize to Vec<String>
            let permissions_json: Option<String> = row.try_get("permissions_json")?;
            let permissions = deserialize_permissions(&permissions_json);
            
            // Build Role struct with permissions as Vec<String>
            Ok(Some(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                created_ts: row.try_get("created_ts")?,
                updated_ts: row.try_get("updated_ts")?,
                is_default: row.try_get("is_default")?,
                permissions,
            }))
        } else {
            Ok(None)
        }
    }
    
    fn deserialize_permissions(json_str: &Option<String>) -> Vec<String> {
        match json_str {
            Some(json) => serde_json::from_str(json).unwrap_or_else(|_| vec![]),
            None => vec![],
        }
    }
}
```

### Input Query Pattern

For query methods that save roles to database:

```rust
// In create_role.rs
impl CreateRoleQuery {
    pub async fn run(pool: &SqlitePool, data: CreateRoleData) -> Result<Role, sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        
        // Serialize Vec<String> to JSON for database storage
        let permissions_json = serde_json::to_string(&data.permissions).map_err(|e| {
            sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize permissions: {e}"),
            ))
        })?;

        // Insert into database with JSON
        let result = sqlx::query(
            r#"
            INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&data.name)
        .bind(now)
        .bind(now)
        .bind(data.is_default)
        .bind(permissions_json)
        .execute(pool)
        .await?;

        let role_id = result.last_insert_rowid();

        // Return Role struct with permissions as Vec<String>
        Ok(Role {
            id: role_id,
            name: data.name,
            created_ts: now,
            updated_ts: now,
            is_default: data.is_default,
            permissions: data.permissions,  // Direct assignment, no JSON
        })
    }
}
```

### Service Layer Updates

Service methods work directly with the updated Role model:

```rust
// In role_service.rs
pub async fn check_user_permission(
    pool: &SqlitePool,
    user: &User,
    required_permission: &str,
) -> Result<bool, RoleError> {
    if let Some(role) = GetRoleByIdQuery::run(pool, user.role_id).await? {
        Ok(role.has_permission(required_permission))
    } else {
        Ok(false)
    }
}
```

## Files to Modify

1. `src/models/role.rs` - Replace permissions_json field with permissions field, remove permissions() method
2. `src/queries/roles/get_role_by_id.rs` - Update to deserialize JSON to Vec<String> and set permissions field
3. `src/queries/roles/get_all_roles.rs` - Update to deserialize JSON to Vec<String> and set permissions field
4. `src/queries/roles/get_role_by_name.rs` - Update to deserialize JSON to Vec<String> and set permissions field
5. `src/queries/roles/create_role.rs` - Update to serialize Vec<String> to JSON for database, set permissions field on return
6. `src/queries/roles/update_role.rs` - Update to serialize Vec<String> to JSON for database, set permissions field on return
7. `src/services/role_service.rs` - Update to use role.permissions directly instead of role.permissions()
8. `src/orchestrators/role_orchestrator.rs` - Replace role.permissions() with role.permissions, keep role.has_permission() calls
9. `src/test_utils/mod.rs` - Update test helpers to work with new structure
10. Various resolver files in `src/graphql/resolvers/` - Replace role.permissions() with role.permissions, keep role.has_permission() calls

## Testing Strategy

1. **Unit Tests**: Update all existing tests to not use the removed methods
2. **Integration Tests**: Ensure GraphQL resolvers still work correctly with string arrays
3. **Permission Tests**: Verify permission checking still works after refactoring
4. **JSON Handling Tests**: Test edge cases like malformed JSON, empty arrays, null values

## Benefits

1. **Cleaner Architecture**: JSON handling is encapsulated in query layer
2. **Better Separation of Concerns**: Models don't handle data conversion
3. **Easier Testing**: Services work with native string arrays
4. **Reduced Coupling**: GraphQL layer doesn't depend on model methods for data conversion

## Risk Mitigation

1. **All-at-once Implementation**: All changes must be implemented together before running tests
2. **Database Schema Unchanged**: Still stores `permissions_json` in database, no migration needed
3. **Comprehensive Testing**: Only run tests after all changes are complete
4. **Type Safety**: Compile-time guarantees that permissions are always string arrays in Rust code

## Implementation Summary

### ✅ **All Steps Completed Successfully**

**1. Role Model Structure (`src/models/role.rs`)**:
- ✅ Replaced `permissions_json: Option<String>` with `permissions: Vec<String>`
- ✅ Removed the `permissions()` method 
- ✅ Updated `has_permission()` method to work with new field
- ✅ Added static `Role::deserialize_permissions()` method to eliminate duplication
- ✅ Added comprehensive unit tests for the new method

**2. Query Objects (`src/queries/roles/*.rs`)**:
- ✅ Updated all query methods to manually deserialize JSON to `Vec<String>`
- ✅ Removed duplicate `deserialize_permissions` functions and centralized in Role model
- ✅ All query methods now return `Role` with `permissions` field set
- ✅ Updated all test methods to work with new structure

**3. Services Layer (`src/services/role_service.rs`)**:
- ✅ Replaced all `role.permissions()` calls with `role.permissions`

**4. Orchestrators Layer (`src/orchestrators/role_orchestrator.rs`)**:
- ✅ Replaced all `role.permissions()` calls with `role.permissions`

**5. GraphQL Resolvers (`src/graphql/resolvers/*.rs`)**:
- ✅ Replaced all `role.permissions()` calls with `role.permissions`

**6. Test Utilities (`src/test_utils/mod.rs`)**:
- ✅ Updated test helpers to use direct field access

### **Final Implementation Details**

**Centralized Static Method**:
```rust
impl Role {
  /// Deserialize permissions from JSON string to Vec<String>
  pub fn deserialize_permissions(json_str: &Option<String>) -> Vec<String> {
    match json_str {
      Some(json) => serde_json::from_str(json).unwrap_or_else(|_| vec![]),
      None => vec![],
    }
  }
}
```

**Query Usage Pattern**:
```rust
// In query files
let permissions_json: Option<String> = row.try_get("permissions_json")?;
let permissions = Role::deserialize_permissions(&permissions_json);
```

### **Results**:
- ✅ **All 453 tests passing**
- ✅ **No compilation errors**
- ✅ **Code formatted and linted**
- ✅ **Database schema unchanged** (still stores JSON)
- ✅ **Type safety improved** (Rust code works with native `Vec<String>`)
- ✅ **DRY principle applied** - eliminated duplicate deserialization logic
- ✅ **Better testability** - centralized method has dedicated unit tests

### **Architecture Benefits**:
- **Clean separation**: Database stores JSON for efficiency, but Rust code works with proper types
- **Type safety**: Compile-time guarantee that permissions are always string arrays in the model
- **Maintainability**: Single source of truth for JSON deserialization logic
- **Performance**: No repeated JSON parsing when accessing permissions multiple times
5. **Backup**: Ensure code is committed before starting implementation