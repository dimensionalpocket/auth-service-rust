# 30-add-user-administration-resolvers.md

## Overview
Add GraphQL resolvers for user administration with proper permission checks and phased implementation. Following existing patterns from site administration resolvers.

**IMPORTANT PATTERN NOTE**: All GraphQL types should be created inside resolver files, following the pattern used in `site.rs` and `add_site.rs`. Do NOT follow the `sites.rs` pattern which uses a separate `types` module - this is an exception, not the standard. All resolver files should define their own response types inline.

## Phases

### Phase 1: List Users Resolver
**Permission**: `can_list_users`

#### Implementation Steps (in order):

**Step 1: Create Query** ✅ **COMPLETE**
- `src/models/user.rs` - Add `UserWithRole` struct for JOIN queries
- `src/queries/users/get_all_users_with_roles.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it returns users with role information

**Step 2: Create Orchestrator** ✅ **COMPLETE**
- `src/orchestrators/user_orchestrator.rs` - Create new orchestrator (following `site_orchestrator.rs` pattern)
- `src/orchestrators/mod.rs` - Add `pub mod user_orchestrator;`
- `src/services/user_service.rs` - Add `UserError` enum if not exists
- **Test**: Write unit tests for orchestrator permission checks and business logic

**Step 3: Create Resolver**
- `src/graphql/resolvers/users.rs` - Create new resolver file (following `site.rs` and `add_site.rs` pattern)
- `src/graphql/resolvers/mod.rs` - Add `pub use users::UsersResolver;`
- `src/graphql/schema.rs` - Add `UsersResolver` to Query MergedObject
- **Test**: Write integration tests for the full resolver flow

#### Implementation Details:
```rust
// src/graphql/resolvers/users.rs
/// GraphQL output type for user listing
#[derive(async_graphql::SimpleObject)]
pub struct UserListing {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    /// The user's role name
    #[graphql(name = "roleName")]
    pub role_name: String,
    /// Timestamp when the user was created
    #[graphql(name = "createdTs")]
    pub created_ts: i64,
    /// Timestamp when the user was last updated
    #[graphql(name = "updatedTs")]
    pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UsersResolver;

#[Object]
impl UsersResolver {
    /// Returns all users with their role information (admin only).
    ///
    /// This query:
    /// - Requires user authentication
    /// - Checks if the user has "can_list_users" permission
    /// - Returns all users with their role names
    /// - Orders users by name alphabetically
    ///
    /// # Returns
    /// * `Vec<UserListing>` - List of all users with role information
    ///
    /// # Errors
    /// * Returns "Authentication required" if user is not authenticated
    /// * Returns "User not found" if authenticated user doesn't exist in database
    /// * Returns "Forbidden" if user lacks "can_list_users" permission
    /// * Returns GraphQL error if database operation fails
    #[instrument(skip(self, ctx))]
    #[graphql(name = "users")]
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<UserListing>> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        match UserOrchestrator::list_users_with_permission_check(
            pool,
            session_context,
        ).await {
            Ok(users) => Ok(users.into_iter().map(|user| UserListing {
                id: user.user.id,
                uuid: user.user.uuid,
                name: user.user.name,
                role_name: user.role_name,
                created_ts: user.user.created_ts,
                updated_ts: user.user.updated_ts,
            }).collect()),
            Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(err) => {
                tracing::error!("Failed to list users: {}", err);
                Err(async_graphql::Error::new("Failed to retrieve users"))
            }
        }
    }
}
```

**IMPORTANT NOTE**: All GraphQL types should be created inside the resolver files, following the pattern used in `site.rs` and `add_site.rs`. Do NOT follow the `sites.rs` pattern which uses a separate `types` module - this is an exception, not the standard.

### Phase 2: Get Single User Resolver
**Permission**: `can_view_user_details`

#### Implementation Steps (in order):

**Step 1: Create Query**
- `src/queries/users/get_user_by_id_with_role.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it returns a single user with role information

**Step 2: Extend Orchestrator**
- `src/orchestrators/user_orchestrator.rs` - Add get user method
- **Test**: Write unit tests for orchestrator permission checks and business logic

**Step 3: Create Resolver**
- `src/graphql/resolvers/user.rs` - Create new single resolver file (following `site.rs` pattern)
- `src/graphql/resolvers/mod.rs` - Add `pub use user::UserResolver;`
- `src/graphql/schema.rs` - Add `UserResolver` to Query MergedObject
- **Test**: Write integration tests for the full resolver flow

#### Implementation Details:
```rust
// src/graphql/resolvers/user.rs
/// GraphQL output type for complete user details (admin only)
#[derive(async_graphql::SimpleObject)]
pub struct UserDetailsResponse {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    /// The user's role ID
    #[graphql(name = "roleId")]
    pub role_id: i64,
    /// The user's role name
    #[graphql(name = "roleName")]
    pub role_name: String,
    /// Timestamp when the user was created
    #[graphql(name = "createdTs")]
    pub created_ts: i64,
    /// Timestamp when the user was last updated
    #[graphql(name = "updatedTs")]
    pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UserResolver;

#[Object]
impl UserResolver {
    #[instrument(skip(ctx), fields(user_id = %id))]
    #[graphql(name = "user")]
    async fn user(&self, ctx: &Context<'_>, id: i64) -> Result<UserDetailsResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        match UserOrchestrator::get_user_details_with_permission_check(
            pool,
            session_context,
            id,
        ).await {
            Ok(user) => Ok(UserDetailsResponse {
                id: user.user.id,
                uuid: user.user.uuid,
                name: user.user.name,
                role_id: user.user.role_id,
                role_name: user.role_name,
                created_ts: user.user.created_ts,
                updated_ts: user.user.updated_ts,
            }),
            Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
                "User with ID {user_id} not found"
            ))),
            Err(err) => {
                tracing::error!("Failed to get user details: {}", err);
                Err(async_graphql::Error::new("Failed to retrieve user details"))
            }
        }
    }
}
```

### Phase 3: Delete User Resolver
**Permission**: `can_delete_user`

#### Implementation Steps (in order):

**Step 1: Create Query**
- `src/queries/users/delete_user_by_id.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it deletes users correctly

**Step 2: Extend Service and Orchestrator**
- `src/services/user_service.rs` - Add delete method and UserError enum (if not exists)
- `src/orchestrators/user_orchestrator.rs` - Add delete user method with self-deletion check
- **Test**: Write unit tests for service and orchestrator permission checks and business logic

**Step 3: Create Resolver**
- `src/graphql/resolvers/delete_user.rs` - Create new resolver file (following `remove_site.rs` pattern)
- `src/graphql/resolvers/mod.rs` - Add `pub use delete_user::DeleteUserResolver;`
- `src/graphql/schema.rs` - Add `DeleteUserResolver` to Mutation MergedObject
- **Test**: Write integration tests for the full resolver flow

#### Implementation Details:
```rust
// src/graphql/resolvers/delete_user.rs
#[derive(Default, Debug)]
pub struct DeleteUserResolver;

#[Object]
impl DeleteUserResolver {
    #[instrument(skip(ctx), fields(user_id = %id))]
    #[graphql(name = "deleteUser")]
    async fn delete_user(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        match UserOrchestrator::delete_user_with_permission_check(
            pool,
            session_context,
            id,
        ).await {
            Ok(_) => Ok(true),
            Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
                "User with ID {user_id} not found"
            ))),
            Err(UserError::SelfDeletion) => Err(async_graphql::Error::new("Cannot delete your own account")),
            Err(err) => {
                tracing::error!("Failed to delete user: {}", err);
                Err(async_graphql::Error::new("Failed to delete user"))
            }
        }
    }
}
```

### Phase 4: Update User Resolver
**Permission**: `can_edit_user`

#### Implementation Steps (in order):

**Step 1: Create Query**
- `src/queries/users/update_user_details.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it updates users correctly with optional fields

**Step 2: Extend Service and Orchestrator**
- `src/services/user_service.rs` - Add update method with password validation
- `src/orchestrators/user_orchestrator.rs` - Add update user method
- **Test**: Write unit tests for service password validation and orchestrator permission checks

**Step 3: Create Resolver**
- `src/graphql/resolvers/update_user.rs` - Create new resolver file (following `update_site.rs` pattern)
- `src/graphql/resolvers/mod.rs` - Add `pub use update_user::UpdateUserResolver;`
- `src/graphql/schema.rs` - Add `UpdateUserResolver` to Mutation MergedObject
- **Test**: Write integration tests for the full resolver flow including password validation

#### Implementation Details:
```rust
// src/graphql/resolvers/update_user.rs
/// GraphQL input type for user updates
#[derive(InputObject)]
pub struct UpdateUserInput {
    pub id: i64,
    pub name: Option<String>,
    /// The user's role ID
    #[graphql(name = "roleId")]
    pub role_id: Option<i64>,
    pub password: Option<String>,
    #[graphql(name = "passwordConfirmation")]
    pub password_confirmation: Option<String>,
}

/// GraphQL output type for user update response
#[derive(async_graphql::SimpleObject)]
pub struct UpdateUserResponse {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    /// The user's role ID
    #[graphql(name = "roleId")]
    pub role_id: i64,
    /// Timestamp when the user was created
    #[graphql(name = "createdTs")]
    pub created_ts: i64,
    /// Timestamp when the user was last updated
    #[graphql(name = "updatedTs")]
    pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UpdateUserResolver;

#[Object]
impl UpdateUserResolver {
    #[instrument(skip(self, ctx, input), fields(user_id = %input.id))]
    #[graphql(name = "updateUser")]
    async fn update_user(&self, ctx: &Context<'_>, input: UpdateUserInput) -> Result<UpdateUserResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        // Validate password confirmation if provided
        if let (Some(password), Some(password_confirmation)) = (&input.password, &input.password_confirmation) {
            if password != password_confirmation {
                return Err(Error::new("Password confirmation does not match"));
            }
        } else if input.password.is_some() || input.password_confirmation.is_some() {
            return Err(Error::new("Both password and password confirmation must be provided"));
        }

        match UserOrchestrator::update_user_with_permission_check(
            pool,
            session_context,
            input.id,
            input.name,
            input.role_id,
            input.password,
        ).await {
            Ok(user) => Ok(UpdateUserResponse {
                id: user.id,
                uuid: user.uuid,
                name: user.name,
                role_id: user.role_id,
                created_ts: user.created_ts,
                updated_ts: user.updated_ts,
            }),
            Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
                "User with ID {user_id} not found"
            ))),
            Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(msg)),
            Err(err) => {
                tracing::error!("Failed to update user: {}", err);
                Err(async_graphql::Error::new("Failed to update user"))
            }
        }
    }
}
```

## Database Schema Considerations

### User Model Extensions
Following existing patterns, create JOIN query result structs:

```rust
// src/models/user.rs
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserWithRole {
    #[serde(flatten)]
    pub user: User,
    pub role_name: String,
}
```

### Query Implementations
All queries should use sqlx with proper parameter binding, following existing patterns:

```rust
// src/queries/users/get_all_users_with_roles.rs
pub async fn run(pool: &SqlitePool) -> Result<Vec<UserWithRole>, sqlx::Error> {
    sqlx::query_as::<_, UserWithRole>(
      r#"
      SELECT 
        u.id, u.uuid, u.created_ts, u.updated_ts, u.name, u.role_id, u.password_hash, u.metadata_json,
        r.name as role_name
      FROM users u
      JOIN user_roles r ON u.role_id = r.id
      ORDER BY u.name
      "#
    )
    .fetch_all(pool)
    .await
}
```

### Permission System Integration
Following existing `SiteOrchestrator` pattern using `UserRoleService::check_user_permission`:

```rust
// src/orchestrators/user_orchestrator.rs
pub async fn list_users_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
) -> Result<Vec<UserWithRole>, UserError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
        .user_id()
        .ok_or(UserError::AuthenticationError("Authentication required".to_string()))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
        .await
        .map_err(UserError::DatabaseError)?
        .ok_or(UserError::UserNotFound(user_id))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_list_users")
        .await
        .map_err(UserError::DatabaseError)?;

    if !allowed {
        return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all users
    GetAllUsersWithRolesQuery::run(pool).await.map_err(UserError::DatabaseError)
}
```

### Error Handling
Create `UserError` enum in `src/services/user_service.rs` following `SiteError` pattern:

```rust
#[derive(Debug)]
pub enum UserError {
    AuthenticationError(String),
    AuthorizationError(String),
    ValidationError(String),
    UserNotFound(i64),
    SelfDeletion,
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for UserError {
    fn from(err: sqlx::Error) -> Self {
        UserError::DatabaseError(err)
    }
}
```

## Security Considerations

1. **Password Updates**: Only update password if both `password` and `password_confirmation` are provided and match
2. **Self-Modification**: Prevent users from deleting themselves (`SelfDeletion` error)
3. **Self-Deletion**: Check `session_context.user_id() != target_user_id`
4. **Role Changes**: Only allow role changes if user has `can_edit_user` permission
5. **Input Validation**: Validate all inputs (username constraints, etc.)
6. **Password Logging**: Ensure password fields are skipped in `#[instrument]` macros

## Testing Strategy

### Unit Tests
Following existing patterns from `site.rs` tests:
- Test each resolver with different permission levels
- Test password validation logic
- Test role assignment logic
- Test error handling (user not found, insufficient permissions, self-deletion)

### Integration Tests
- Test full resolver flow with database
- Test permission enforcement using role-based permissions
- Test admin bypass (users with `is_admin` permission)

## TODO: Update README.md
After implementation, update the README.md mutations/queries table to include:
- Query: `users` - List all users (requires can_list_users)
- Query: `user(id: ID!)` - Get user by ID (requires can_view_user_details)
- Mutation: `deleteUser(id: ID!)` - Delete user (requires can_delete_user)
- Mutation: `updateUser(input: UpdateUserInput!)` - Update user (requires can_edit_user)

## GraphQL Naming Conventions
Based on existing codebase analysis:
- **Query/Mutation names**: camelCase with `#[graphql(name = "camelCase")]` (e.g., `updateUser`, `deleteUser`, `authLogin`)
- **Field names**: snake_case in Rust with `#[graphql(name = "camelCase")]` for GraphQL output (e.g., `role_id` → `roleId`, `created_ts` → `createdTs`)
- **Input types**: camelCase field names with `#[graphql(name = "camelCase")]` annotations
- **Response types**: Follow same pattern as existing `AddSiteResponse`, `UpdateSiteResponse`, `AuthLoginResponse`

## Dependencies
No new crates required - using existing async-graphql, sqlx, tracing, and project patterns.