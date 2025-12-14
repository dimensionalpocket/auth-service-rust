# Plan: Update User Resolvers to Include Nested Role Objects

**Date**: 2025-12-13@21:07
**Breaking Change**: Yes

## Overview

Update all GraphQL resolvers that return user data to include a nested `role` object containing `id`, `name`, and `permissions` fields. Replace separate `roleId` and `roleName` fields with the new role object structure.

## Current State Analysis

### Resolvers Requiring Updates:
1. **`auth_me.rs`** - `AuthMeResponse` (currently has `roleId`, `roleName`)
2. **`auth_login.rs`** - `AuthLoginResponse` (currently missing role info)
3. **`auth_register.rs`** - `AuthRegisterResponse` (currently has `roleId`)
4. **`user.rs`** - `UserDetailsResponse` (currently has `roleId`, `roleName`)
5. **`users.rs`** - `UserListing` (currently has `roleName`)

### Current Field Replacements:
- `roleId` → `role.id`
- `roleName` → `role.name`
- **New fields to add**: `role.permissions`

## Implementation Plan

### Phase 1: Database Layer Updates

#### 1.1 Update UserWithRole Model
**Files**: `src/models/user.rs`

- Update `UserWithRole` model to include full `Role` object instead of just `role_name`
- Replace `role_name: String` with `role: Role`

#### 1.2 Update User Query Objects
**Files**: `src/queries/users/mod.rs`, `src/queries/users/get_user_by_id.rs`, `src/queries/users/get_all_users_with_roles.rs`

- Update existing queries to return full role information in `UserWithRole`
- Join with roles table to get complete role data including permissions
- Add permissions deserialization using `Role::deserialize_permissions()` pattern
- Follow the established query pattern: fetch `permissions_json` from database, convert to `Vec<String>` in query

#### 1.3 Update Multiple Users Query
**Files**: `src/queries/users/get_all_users_with_roles.rs`

- Modify queries to fetch complete role data for each user
- Return `Vec<UserWithRole>` with full `Role` objects

### Phase 2: Model Layer Updates

#### 2.1 Update UserWithRole Model
**Files**: `src/models/user.rs`

- Update `UserWithRole` struct to include full `Role` object:
```rust
pub struct UserWithRole {
    #[sqlx(flatten)]
    pub user: User,
    pub role: Role,  // Now has direct Vec<String> permissions field
}
```

#### 2.2 Add GraphQL Role Type for User Context
**Files**: Create `src/graphql/types/user_role.rs` (new)

- Define `UserRole` GraphQL type for nested role objects in user responses
- Include fields: `id`, `name`, `permissions`
- Note: Role model now has direct `permissions: Vec<String>` field, no conversion needed
- Convert `Role` model to `UserRole` GraphQL type

### Phase 3: Service Layer Updates

#### 3.1 Update User Service Methods
**Files**: `src/services/user_service.rs`

- Update `get_user_by_id()` to return `UserWithRole` with full role data
- Update `get_all_users()` to return `Vec<UserWithRole>` with full role data
- **No permissions conversion needed** - queries now return `Vec<String>` directly

#### 3.2 Update Auth Service Methods
**Files**: `src/services/auth_service.rs`

- Update authentication methods to include full role data
- Modify login and registration flows to return `UserWithFullRole`

### Phase 4: Orchestrator Layer Updates

#### 4.1 Update Auth Orchestrator
**Files**: `src/orchestrators/auth_orchestrator.rs`

- Update methods to handle `UserWithRole` with full role data
- Convert `Role` models to `UserRole` GraphQL types for responses

#### 4.2 Update User Orchestrator
**Files**: `src/orchestrators/user_orchestrator.rs`

- Update user retrieval methods to work with `UserWithRole` with full role data
- Handle role data transformation for GraphQL responses

### Phase 5: GraphQL Resolver Updates

#### 5.1 Update Auth Resolvers
**Files**: `src/graphql/resolvers/auth_me.rs`, `src/graphql/resolvers/auth_login.rs`, `src/graphql/resolvers/auth_register.rs`

**`auth_me.rs`**:
```rust
pub struct AuthMeResponse {
    pub user: UserWithRoleResponse,
}

pub struct UserWithRoleResponse {
    pub id: ID,
    pub uuid: String,
    pub username: String,
    pub role: UserRole,  // Replace roleId, roleName
    pub created_ts: i64,
    pub updated_ts: i64,
    pub session_iat: i64,
    pub session_exp: i64,
}
```

**`auth_login.rs`**:
```rust
pub struct AuthLoginResponse {
    pub token: String,
    pub user: UserWithRoleResponse,  // Add user with role
    pub message: String,
}
```

**`auth_register.rs`**:
```rust
pub struct AuthRegisterResponse {
    pub user: UserWithRoleResponse,  // Replace userId, roleId with full user
    pub message: String,
}
```

#### 5.2 Update User Management Resolvers
**Files**: `src/graphql/resolvers/user.rs`, `src/graphql/resolvers/users.rs`

**`user.rs`**:
```rust
pub struct UserDetailsResponse {
    pub user: UserWithRoleResponse,  // Replace separate fields with nested object
}
```

**`users.rs`**:
```rust
pub struct UserListing {
    pub id: ID,
    pub uuid: String,
    pub username: String,
    pub role: UserRole,  // Replace roleName
    pub created_ts: i64,
    pub updated_ts: i64,
}
```

#### 5.3 Add Common User Response Type
**Files**: Create `src/graphql/types/user_response.rs` (new)

- Define shared `UserWithRoleResponse` type used across all resolvers
- Include conversion methods from `UserWithFullRole` to GraphQL type

### Phase 6: Schema Updates

#### 6.1 Update GraphQL Schema
**Files**: `src/graphql/schema.rs`

- Import new `UserRole` and updated response types
- Update type annotations in resolver implementations

### Phase 7: Test Updates

#### 7.1 Update Unit Tests
**Files**: All test modules in affected resolver files

- Update test expectations to match new response structure
- Test role object nesting and permissions array
- Verify field names match GraphQL conventions (camelCase)

#### 7.2 Update Integration Tests
**Files**: `tests/integration_tests.rs`, `tests/database_integration_tests.rs`

- Update auth flow tests to expect role objects
- Update user management tests for new response format
- Test role permissions are properly included in responses

### Phase 8: README Updates

#### 8.1 Update GraphQL Schema Tables
**Files**: `README.md`

- Update the Queries and Mutations tables to reflect new response structures
- Remove references to `roleId` and `roleName` fields  
- Update field descriptions in auth and user management queries/mutations to show nested `role` objects
- Update existing GraphQL examples to show the new role object structure
- Update any existing response examples that reference user response types

## Implementation Details

### New GraphQL Type Definitions

```rust
// src/graphql/types/user_role.rs
use async_graphql::{Object, Result};
use crate::models::Role;

pub struct UserRole {
    pub id: ID,
    pub name: String,
    pub permissions: Vec<String>,
}

#[Object]
impl UserRole {
    pub async fn id(&self) -> ID {
        self.id.clone()
    }
    
    pub async fn name(&self) -> &str {
        &self.name
    }
    
    pub async fn permissions(&self) -> &Vec<String> {
        &self.permissions
    }
}

impl From<Role> for UserRole {
    fn from(role: Role) -> Self {
        Self {
            id: ID(role.id.to_string()),
            name: role.name,
            permissions: role.permissions,  // Direct access, no method call needed
        }
    }
}
```

### Database Query Example

```rust
// src/queries/users/get_user_by_id.rs (updated)
pub async fn get_user_by_id(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<UserWithRole, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT 
            u.id,
            u.uuid,
            u.created_ts,
            u.updated_ts,
            u.name,
            u.role_id,
            u.password_hash,
            u.metadata_json,
            r.id as role_id,
            r.name as role_name,
            r.created_ts as role_created_ts,
            r.updated_ts as role_updated_ts,
            r.is_default as role_is_default,
            r.permissions_json as role_permissions_json
        FROM users u
        LEFT JOIN roles r ON u.role_id = r.id
        WHERE u.id = ?
        "#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let user = User {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        created_ts: row.try_get("created_ts")?,
        updated_ts: row.try_get("updated_ts")?,
        name: row.try_get("name")?,
        role_id: row.try_get("role_id")?,
        password_hash: row.try_get("password_hash")?,
        metadata_json: row.try_get("metadata_json")?,
    };

    let permissions_json: Option<String> = row.try_get("role_permissions_json")?;
    let permissions = Role::deserialize_permissions(&permissions_json);
    
    let role = Role {
        id: row.try_get("role_id")?,
        name: row.try_get("role_name")?,
        created_ts: row.try_get("role_created_ts")?,
        updated_ts: row.try_get("role_updated_ts")?,
        is_default: row.try_get("role_is_default")?,
        permissions,
    };

    Ok(UserWithRole { user, role })
}
```

### Resolver Implementation Example

```rust
// src/graphql/resolvers/auth_me.rs
pub async fn auth_me(&self, ctx: &Context<'_>) -> Result<AuthMeResponse> {
    let session = ctx.data::<Session>()?;
    let pool = ctx.data::<SqlitePool>()?;
    
    let user_with_role = self
        .auth_orchestrator
        .get_authenticated_user(pool, session)
        .await?;
    
    Ok(AuthMeResponse {
        user: UserWithRoleResponse::from(user_with_role),
    })
}
```

## Testing Strategy

### Unit Tests
- Test conversion from `Role` to `UserRole` GraphQL type
- Test `UserWithRole` model creation and role nesting
- Test resolver responses include proper role objects
- Test permissions are correctly deserialized from JSON in queries
- Verify `Role::deserialize_permissions()` is used correctly in user queries

### Integration Tests  
- Test auth flows return complete role information
- Test user listing queries include role objects for all users
- Test role permissions arrays are properly serialized

### Field Name Validation
- Verify all GraphQL fields follow camelCase convention
- Test role object nesting works correctly in GraphQL queries
- Confirm old fields (`roleId`, `roleName`) are removed

## Dependencies

No new crates required. Uses existing:
- `async-graphql` for GraphQL types
- `sqlx` for database queries  
- `serde` for JSON handling (existing)
- `Role::deserialize_permissions()` method for permissions conversion (existing)

## Notes

- This is a breaking change that affects all user-related GraphQL queries
- Role permissions will be returned as arrays of strings in GraphQL responses
- All timestamp fields maintain existing Unix epoch format
- Database schema changes are not required - role data already exists
- Permissions conversion follows the established pattern: JSON in database → `Vec<String>` in queries → direct field access in GraphQL
- The refactored permissions system means no `permissions()` method calls needed - use direct field access: `role.permissions`