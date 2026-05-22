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

**Step 3: Create Resolver** ✅ **COMPLETE**
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

**Step 1: Create Query** ✅ **COMPLETE**
- `src/queries/users/get_user_by_id_with_role.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it returns a single user with role information

**Step 2: Extend Orchestrator** ✅ **COMPLETE**
- `src/orchestrators/user_orchestrator.rs` - Add get user method
- **Test**: Write unit tests for orchestrator permission checks and business logic

**Step 3: Create Resolver** ✅ **COMPLETE**
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

**Step 1: Create Query** ✅ **COMPLETE**
- `src/queries/users/delete_user_by_id.rs` - Create new query
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it deletes users correctly

**Step 2: Extend Service and Orchestrator** ✅ **COMPLETE**
- `src/services/user_service.rs` - Add delete method and UserError enum (if not exists)
- `src/orchestrators/user_orchestrator.rs` - Add delete user method with self-deletion check
- **Test**: Write unit tests for service and orchestrator permission checks and business logic

**Step 3: Create Resolver** ✅ **COMPLETE**
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

**Step 1: Create Query** ✅ **COMPLETE**
- `src/queries/users/update_user.rs` - Create new query supporting partial updates (PATCH semantics)
- `src/queries/users/mod.rs` - Add new query export
- **Test**: Write unit tests for the query to verify it updates users correctly with optional fields and proper null handling
- **FIXED**: Corrected `role_id` handling - database schema shows `role_id INTEGER NOT NULL`, so `UpdateUserData.role_id` is now `Option<i64>` (not `Option<Option<i64>>`), and query binding updated accordingly

**Step 2: Extend Service and Orchestrator** ✅ **COMPLETE**
- `src/services/user_service.rs` - Add `update_user` method with partial update support and password validation
- `src/orchestrators/user_orchestrator.rs` - Add `update_user_with_permission_check` method
- **Test**: Write unit tests for service partial update logic, password validation, and orchestrator permission checks

**Step 3: Create Resolver** ✅ **COMPLETE**
- `src/graphql/resolvers/update_user.rs` - Create new resolver file (following `update_site.rs` pattern)
- `src/graphql/resolvers/mod.rs` - Add `pub use update_user::UpdateUserResolver;`
- `src/graphql/schema.rs` - Add `UpdateUserResolver` to Mutation MergedObject
- **Test**: Write integration tests for the full resolver flow including partial updates and password validation

#### Implementation Notes (Updated based on Phase 3 learnings and site update patterns):
- **Session Context**: Use `session_context.clone()` when passing to orchestrators (as learned from delete user)
- **Partial Updates**: Follow `update_site.rs` pattern with `Option<Option<T>>` for nullable fields (metadata_json) and `Option<T>` for non-nullable fields (role_id) - this allows explicit null values to set nullable database columns to NULL
- **Database Schema Constraints**: `role_id` is NOT NULL in database (foreign key constraint), so it cannot be set to NULL - only `Option<i64>` is needed for partial updates
- **Metadata JSON Handling**: The `metadata_json` field uses `Option<Option<String>>` in UpdateUserData to distinguish between no update (None) vs explicit NULL (Some(None)) vs value update (Some(Some(value)))
- **Password Handling**: Use `#[instrument(skip(self, ctx, input, password, password_confirmation))]` pattern for sensitive data
- **Error Consistency**: Follow same error handling patterns as delete user (AuthenticationError, AuthorizationError, UserNotFound, ValidationError)
- **Testing Patterns**: Use delete user and update site tests as template for comprehensive permission testing and database verification
- **Documentation**: Include comprehensive doc comments covering all requirements and error cases
- **Username Uniqueness**: Validate username uniqueness when name is being updated (case-insensitive check)
- **UserError Enum**: The UserError enum already exists with all necessary variants including UsernameAlreadyExists, ValidationError, etc.

#### Implementation Details:
```rust
// src/queries/users/update_user.rs
/// Data structure for updating user details with partial update support
#[derive(Debug)]
pub struct UpdateUserData {
    pub id: i64,
    pub name: Option<String>,
    pub role_id: Option<i64>, // role_id is NOT NULL in database, so only Option<i64> for partial updates
    pub password_hash: Option<String>,
    pub metadata_json: Option<Option<String>>, // Allows explicit NULL setting for metadata
}

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
    /// Optional metadata JSON for the user
    #[graphql(name = "metadataJson")]
    pub metadata_json: Option<String>,
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
    /// Updates an existing user with the provided parameters.
    ///
    /// This mutation:
    /// - Requires user authentication
    /// - Checks if the user has "can_edit_user" permission
    /// - Validates the username format and uniqueness if provided
    /// - Updates only the fields provided (PATCH semantics)
    /// - Explicit null values set database columns to NULL
    /// - Validates password confirmation if password is provided
    /// - Automatically updates the updated_ts timestamp
    /// - Returns the updated user information
    ///
    /// # Arguments
    /// * `id` - User ID to update
    /// * `name` - Optional new username for the user (3-20 chars, alphanumeric + underscore/hyphen)
     /// * `roleId` - Optional new role ID for the user
     /// * `password` - Optional new password for the user
     /// * `passwordConfirmation` - Optional password confirmation (required if password provided)
     /// * `metadataJson` - Optional new metadata JSON for the user
    ///
    /// # Returns
    /// * `UpdateUserResponse` - The updated user information
    ///
    /// # Errors
    /// * Returns "Authentication required" if user is not authenticated
    /// * Returns "User not found" if authenticated user doesn't exist in database
    /// * Returns "Forbidden" if user lacks "can_edit_user" permission
    /// * Returns GraphQL error if username validation fails
    /// * Returns GraphQL error if username already exists
    /// * Returns GraphQL error if password confirmation doesn't match
    /// * Returns GraphQL error if target user is not found
    /// * Returns GraphQL error if database operation fails
    #[instrument(skip(self, ctx, input, password, password_confirmation), fields(user_id = %input.id))]
    #[graphql(name = "updateUser")]
    async fn update_user(&self, ctx: &Context<'_>, input: UpdateUserInput) -> Result<UpdateUserResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        // Validate password confirmation if provided
        if let (Some(password), Some(password_confirmation)) = (&input.password, &input.password_confirmation) {
            if password != password_confirmation {
                return Err(async_graphql::Error::new("Password confirmation does not match"));
            }
        } else if input.password.is_some() || input.password_confirmation.is_some() {
            return Err(async_graphql::Error::new("Both password and password confirmation must be provided"));
        }

        // Convert parameters to UpdateUserData with proper null handling
        let update_data = UpdateUserData {
            id: input.id,
            name: input.name,
            role_id: input.role_id, // role_id is NOT NULL in database, so only Option<i64> needed
            password_hash: None, // Will be set by service if password provided
            metadata_json: input.metadata_json.map(Some), // Convert Option<T> to Option<Option<T>> for nullable field
        };

        match UserOrchestrator::update_user_with_permission_check(
            pool,
            session_context.clone(),
            input.id,
            update_data,
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
            Err(UserError::UsernameAlreadyExists(username)) => Err(async_graphql::Error::new(format!(
                "Username '{username}' is already in use"
            ))),
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

```rust
// src/queries/users/update_user.rs
/// Database query for updating user details with partial update support
pub struct UpdateUserQuery;

impl UpdateUserQuery {
    /// Update a user's details in the database with partial update support
    ///
    /// This method updates only the fields provided in the update_data.
    /// It uses dynamic SQL building to handle partial updates properly.
    /// It automatically updates the updated_ts timestamp.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `update_data` - The user data to update (partial fields only)
    ///
    /// # Returns
    /// * `Ok(User)` - The updated user with new timestamp
    /// * `Err(sqlx::Error)` - Database error if the update fails
    ///
    /// # Errors
    /// * Returns error if user_id doesn't exist
    /// * Returns error if database operation fails
    pub async fn run(
        pool: &SqlitePool,
        update_data: UpdateUserData,
    ) -> Result<User, sqlx::Error> {
        let current_timestamp = chrono::Utc::now().timestamp();
        
        // Build dynamic UPDATE query based on provided fields
        let mut query_parts = Vec::new();
        let mut bind_values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send>> = Vec::new();
        
        // Add fields to update if they are provided
        if update_data.name.is_some() {
            set_clauses.push("name = ?");
            has_updates = true;
        }
        
        if update_data.role_id.is_some() {
            set_clauses.push("role_id = ?");
            has_updates = true;
        }
        
        if update_data.password_hash.is_some() {
            set_clauses.push("password_hash = ?");
            has_updates = true;
        }
        
        if update_data.metadata_json.is_some() {
            set_clauses.push("metadata_json = ?");
            has_updates = true;
        }
        
        if let Some(role_id) = &update_data.role_id {
            query_parts.push("role_id = ?");
            bind_values.push(Box::new(role_id));
        }
        
        if let Some(password_hash) = &update_data.password_hash {
            query_parts.push("password_hash = ?");
            bind_values.push(Box::new(password_hash.clone()));
        }
        
        if let Some(metadata_json) = &update_data.metadata_json {
            query_parts.push("metadata_json = ?");
            bind_values.push(Box::new(metadata_json.clone()));
        }
        
        // Always update the timestamp
        query_parts.push("updated_ts = ?");
        bind_values.push(Box::new(current_timestamp));
        
        // If no fields to update, just return the existing user
        if query_parts.is_empty() {
            return GetUserByIdQuery::run(pool, update_data.id)
                .await?
                .ok_or(sqlx::Error::RowNotFound);
        }
        
        // Build the final query
        let set_clause = query_parts.join(", ");
        let query_str = format!(
            r#"
            UPDATE users 
            SET {}
            WHERE id = ?
            RETURNING id, uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts
            "#,
            set_clause
        );
        
        let mut query = sqlx::query_as::<_, User>(&query_str);
        
        // Bind all the values
        for value in bind_values {
            query = query.bind(value);
        }
        
        // Bind the WHERE clause parameter
        query = query.bind(update_data.id);
        
        query.fetch_one(pool).await
    }
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

/// Update a user with permission check and validation
///
/// This method:
/// - Checks if user is authenticated
/// - Verifies user has "can_edit_user" permission
/// - Validates username uniqueness if name is being updated
/// - Validates password if provided
/// - Updates only the provided fields (partial update)
/// - Returns the updated user
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `session_context` - Session context for authentication/authorization
/// * `target_user_id` - ID of user to update
/// * `update_data` - Partial user data to update (with Option<Option<T>> for nullable fields)
/// * `password` - Optional new password (plain text, will be hashed)
///
/// # Returns
/// * `Ok(User)` - Successfully updated user
/// * `Err(UserError)` - Authentication, authorization, validation, or database error
pub async fn update_user_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    target_user_id: i64,
    mut update_data: UpdateUserData,
    password: Option<String>,
) -> Result<User, UserError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
        .user_id()
        .ok_or(UserError::AuthenticationError("Authentication required".to_string()))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
        .await
        .map_err(UserError::DatabaseError)?
        .ok_or(UserError::UserNotFound(user_id))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_edit_user")
        .await
        .map_err(UserError::DatabaseError)?;

    if !allowed {
        return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Validate and update user
    UserService::update_user(pool, target_user_id, update_data, password).await
}
```

### Service Layer Integration
Following existing `UserService` patterns for validation and database operations:

```rust
// src/services/user_service.rs
impl UserService {
    /// Update a user's details with validation and partial update support
    ///
    /// This method validates input, checks for username uniqueness if name is changing,
    /// hashes password if provided, and updates only specified fields.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `user_id` - The ID of user to update
    /// * `update_data` - Partial user data to update (with Option<Option<T>> for nullable fields)
    /// * `password` - Optional new password (plain text, will be hashed)
    ///
    /// # Returns
    /// * `Ok(User)` - Successfully updated user
    /// * `Err(UserError)` - Validation, uniqueness, or database error
    pub async fn update_user(
        pool: &SqlitePool,
        user_id: i64,
        mut update_data: UpdateUserData,
        password: Option<String>,
    ) -> Result<User, UserError> {
        // Get current user for validation and comparison
        let current_user = GetUserByIdQuery::run(pool, user_id)
            .await
            .map_err(UserError::DatabaseError)?
            .ok_or(UserError::UserNotFound(user_id))?;

        // Validate username if provided
        if let Some(ref name) = update_data.name {
            Self::validate_username(name)?;
            
            // Check username uniqueness if name is changing
            if name != &current_user.name {
                if let Some(_existing_user) = GetUserByNameQuery::run(pool, name).await? {
                    return Err(UserError::UsernameAlreadyExists(name.to_string()));
                }
            }
        }

        // Hash password if provided
        if let Some(password) = password {
            Self::validate_password(&password)?;
            let password_hash = PasswordService::generate(&password)?;
            update_data.password_hash = Some(password_hash);
        }

        // Update user in database
        UpdateUserQuery::run(pool, update_data)
            .await
            .map_err(UserError::DatabaseError)
    }
}
```

### Error Handling
The `UserError` enum already exists in `src/services/user_service.rs` and includes all necessary error variants:

```rust
#[derive(Debug)]
pub enum UserError {
    UsernameAlreadyExists(String),
    PasswordHashingFailed(PasswordError),
    DatabaseError(sqlx::Error),
    ValidationError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    UserNotFound(i64),
    SelfDeletion,
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