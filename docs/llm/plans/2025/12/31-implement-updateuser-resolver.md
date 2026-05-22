# Plan: Implement updateUser Resolver

## Overview
Create a GraphQL resolver for the `updateUser` mutation following an "ideal pattern" for future refactoring. The orchestrator method `UserOrchestrator::update_user_with_permission_check` exists but needs to be updated to:
1. Return `UserWithRole` instead of `User` for consistency with other orchestrator methods
2. Accept a high-level input type from the resolver (establishes ideal pattern)
3. Convert the high-level input to low-level `UpdateUserData` internally (temporary workaround until service/query layer refactoring)

This establishes the correct architecture pattern for future resolvers/orchestrators while keeping changes limited to resolver and orchestrator layers only (no changes to UserService or queries).

## Files to Create/Modify

### 0. Create: `src/types/user/update_user_input.rs` (new module)
Create a new top-level `types` directory with domain-based subdirectories for application-level input/output types.

**Directory structure:**
```
src/types/
  mod.rs          // Module declarations and re-exports
  user/           // User-related app-level types
    mod.rs
    update_user_input.rs
  // Future: role/, site/, etc.
```

**File content: `src/types/user/update_user_input.rs`:**
```rust
/// High-level input type for updateUser mutation
///
/// This type represents application-level input data that is:
/// - Created by the resolver from GraphQL arguments
/// - Passed to the orchestrator for validation and business logic
/// - Not used directly in GraphQL schema (schema uses individual arguments)
///
/// This establishes a clean pattern: resolver creates app-level types,
/// orchestrator accepts app-level types, service layer performs business logic.
#[derive(Debug)]
pub struct UpdateUserInput {
  pub id: i64,
  pub name: Option<String>,
  pub role_id: Option<i64>,
  pub password: Option<String>,
  pub password_confirmation: Option<String>,
  pub metadata_json: Option<String>,
}
```

### 3. Modify: `src/lib.rs`
Add new types module to the crate root:

```rust
pub mod types;
```

### 4. Modify: `src/orchestrators/user_orchestrator.rs`

**Changes needed:**

1. Import high-level input type:
```rust
use crate::types::user::update_user_input::UpdateUserInput;
```

2. Update return type from `Result<User, UserError>` to `Result<UserWithRole, UserError>`
3. Update function signature to accept the high-level input type:
```rust
pub async fn update_user_with_permission_check(
  pool: &SqlitePool,
  session_context: SessionContext,
  input: UpdateUserInput,
) -> Result<UserWithRole, UserError>
```

4. Implement orchestrator logic with validation and conversion:
```rust
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

let allowed = RoleService::check_user_permission(&mut conn, &user, "can_edit_user").await?;

if !allowed {
  return Err(UserError::AuthorizationError("Forbidden".to_string()));
}

// Business logic: Validate password confirmation
let password_to_update = if input.password.is_some() || input.password_confirmation.is_some() {
  match (&input.password, &input.password_confirmation) {
    (Some(pw), Some(confirm_pw)) if pw == confirm_pw => {
      // Passwords match, proceed with update using pw
      Some(pw.clone())
    }
    (Some(_), None) | (None, Some(_)) => {
      return Err(UserError::ValidationError(
        "Password and password confirmation must both be provided".to_string()
      ));
    }
    (Some(_), Some(_)) => {
      return Err(UserError::ValidationError(
        "Password and password confirmation do not match".to_string()
      ));
    }
    _ => None, // Both are None, shouldn't happen due to outer check
  }
} else {
  None // Both are None, no password update
};

// Business logic: Convert high-level input to low-level UpdateUserData
let update_data = UpdateUserData {
  id: input.id,
  name: input.name,
  role_id: input.role_id,
  password_hash: None, // Service will set this after hashing
  metadata_json: input.metadata_json.map(Some),
};

// Business logic: Validate and update user
let _updated_user = UserService::update_user(&mut conn, input.id, update_data, password_to_update).await?;

// Business logic: Fetch updated user with role details
GetUserByIdWithRoleQuery::run(&mut conn, input.id)
  .await
  .map_err(UserError::DatabaseError)?
  .ok_or(UserError::UserNotFound(input.id))
```

5. Update documentation to reflect `UpdateUserInput` and `UserWithRole` return type:
   - Change parameter docs from individual fields to `input: UpdateUserInput`
   - Change "Returns: updated user" to "Returns: updated user with role information"
   - Change return type documentation to `Ok(UserWithRole)`

6. Add password confirmation validation tests:
   - Test with password but no passwordConfirmation → Expect "must both be provided" error
   - Test with passwordConfirmation but no password → Expect "must both be provided" error
   - Test with mismatching passwords → Expect "do not match" error
   - Test with matching passwords → Expect successful password update

7. Update test assertions to expect `UserWithRole`:
   - In `test_update_user_with_permission_check_success`: Update to construct `UpdateUserInput` and check `user_with_role.user.name`, `user_with_role.user.role_id`, `user_with_role.role.name`
   - Other tests that don't check role details can access `result.unwrap().user` instead of `result.unwrap()`
   - All tests must update result unwrapping: `result.unwrap()` → `result.unwrap().user` (for user fields) or `result.unwrap().role` (for role fields)
   - Update existing password update tests to construct `UpdateUserInput` with both password and password_confirmation

### 4. Create: `src/graphql/resolvers/update_user.rs`
Create a new resolver file following to "ideal pattern" for future refactoring.

**Key components:**

#### Response Type
```rust
#[derive(async_graphql::SimpleObject)]
pub struct UpdateUserResponse {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  pub role: UserRole,
  #[graphql(name = "metadataJson")]
  pub metadata_json: Option<String>,
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

#[derive(async_graphql::SimpleObject)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub permissions: Vec<String>,
}
```

#### Resolver Implementation
```rust
use crate::types::user::update_user_input::UpdateUserInput;
use crate::orchestrators::user_orchestrator::UserOrchestrator;
use crate::middleware::session::SessionContext;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(Default, Debug)]
pub struct UpdateUserResolver;

#[Object]
impl UpdateUserResolver {
  #[instrument(skip(ctx, password, password_confirmation), fields(id = %id))]
  #[allow(clippy::too_many_arguments)]
  #[graphql(name = "updateUser")]
  async fn update_user(
    &self,
    ctx: &Context<'_>,
    id: i64,
    name: Option<String>,
    #[graphql(name = "roleId")] role_id: Option<i64>,
    password: Option<String>,
    #[graphql(name = "passwordConfirmation")] password_confirmation: Option<String>,
    #[graphql(name = "metadataJson")] metadata_json: Option<String>,
  ) -> Result<UpdateUserResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context
    let session_context = SessionContext::from_context(ctx)?;

    // Create high-level input type from GraphQL arguments
    let input = UpdateUserInput {
      id,
      name,
      role_id,
      password,
      password_confirmation,
      metadata_json,
    };

    match UserOrchestrator::update_user_with_permission_check(
      pool,
      session_context.clone(),
      input,
    )
    .await
    {
      Ok(user_with_role) => Ok(UpdateUserResponse {
        id: user_with_role.user.id,
        uuid: user_with_role.user.uuid,
        name: user_with_role.user.name,
        role: UserRole {
          id: user_with_role.role.id,
          name: user_with_role.role.name,
          permissions: user_with_role.role.permissions,
        },
        metadata_json: user_with_role.user.metadata_json,
        created_ts: user_with_role.user.created_ts,
        updated_ts: user_with_role.user.updated_ts,
      }),
      Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
        "User with ID {user_id} not found"
      ))),
      Err(UserError::UsernameAlreadyExists(username)) => Err(async_graphql::Error::new(format!(
        "Username '{username}' is already in use"
      ))),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to update user: {}", err);
        Err(async_graphql::Error::new("Failed to update user"))
      }
    }
  }
}
```

**Key behaviors:**
- Extract `SqlitePool` from context
- Extract `SessionContext` from context
- Create high-level `UpdateUserInput` from GraphQL arguments
- Pass `UpdateUserInput` to orchestrator (not converting to low-level types)
- The orchestrator handles validation and conversion to `UpdateUserData`
- Map errors to GraphQL errors:
  - `UserError::AuthenticationError` → "Authentication required"
  - `UserError::AuthorizationError` → "Forbidden"
  - `UserError::UserNotFound` → "User with ID {id} not found"
  - `UserError::ValidationError` → "Validation error: {msg}" (includes password confirmation mismatch)
  - `UserError::UsernameAlreadyExists` → "Username '{username}' is already in use"
  - Other errors → Log and return generic error
- Return `UpdateUserResponse` on success

### 6. Modify: `src/graphql/resolvers/mod.rs`
Add export for new resolver:
```rust
pub mod update_user;

pub use update_user::UpdateUserResolver;
```

### 7. Modify: `src/graphql/schema.rs`
Add `UpdateUserResolver` to the Mutation struct:
```rust
use crate::graphql::resolvers::{
  // ... existing imports
  UpdateUserResolver,
  // ... existing imports
};

#[derive(MergedObject, Default)]
pub struct Mutation(
  // ... existing resolvers
  UpdateUserResolver,
  // ... existing resolvers
);
```

Add to Mutation documentation:
```
- updateUser: Update an existing user (requires can_edit_user permission)
```

### 8. Modify: `README.md`
Add `updateUser` to the Mutations table:
```
| updateUser | Update existing user. Input: id (Int!), name (String), roleId (Int), password (String), passwordConfirmation (String), metadataJson (String). Returns: id (Int), uuid (String), name (String), role { id (Int), name (String), permissions ([String]) }, metadataJson (String), createdTs (Int), updatedTs (Int). Requires can_edit_user permission. |
```

## Implementation Details

### Error Handling Pattern
Follow the established pattern from `update_site.rs` and `update_role.rs`:

**Note:** The complete implementation is shown in the resolver section above. The error handling pattern uses the orchestrator's new signature accepting `UpdateUserInput`.

### Input Handling for Password
The orchestrator accepts two password parameters (`password` and `password_confirmation`) and validates them before passing to UserService. The resolver should:
1. Accept `password` and `password_confirmation` as `Option<String>` from GraphQL
2. Pass both directly to the orchestrator without modification
3. The orchestrator's validation logic:
   - If both `password` and `password_confirmation` are None/empty → No password update
   - If one is present but the other is not → Return `ValidationError: "Password and password confirmation must both be provided"`
   - If both are present but don't match → Return `ValidationError: "Password and password confirmation do not match"`
   - If both are present and match → Pass password to `UserService::update_user`
4. The `UserService::update_user` method will:
   - Validate password length (min 6 characters) if provided
   - Hash the password using `PasswordService::generate()`
   - Update the user with the hashed password

This pattern minimizes accidental password updates by requiring explicit confirmation from admins.

### Password Confirmation Validation Logic
The orchestrator uses a match pattern to validate password confirmation:

```rust
// Business logic: Validate password confirmation
if password.is_some() || password_confirmation.is_some() {
  match (password, password_confirmation) {
    (Some(pw), Some(confirm_pw)) if pw == confirm_pw => {
      // Passwords match, proceed with update using pw
    }
    (Some(_), None) | (None, Some(_)) => {
      return Err(UserError::ValidationError(
        "Password and password confirmation must both be provided".to_string()
      ));
    }
    (Some(_), Some(_)) => {
      return Err(UserError::ValidationError(
        "Password and password confirmation do not match".to_string()
      ));
    }
    _ => {
      // Both are None, proceed with no password update
    }
  }
}

// Determine password to pass to service (Some if provided and match, None otherwise)
let password_to_update = match (password, password_confirmation) {
  (Some(pw), Some(confirm_pw)) if pw == confirm_pw => Some(pw),
  _ => None,
};
```

This approach ensures:
- No silent failures when only one password field is provided
- Clear error messages when passwords don't match
- Graceful handling when both fields are omitted (no password update)
- Type-safe validation using Rust's pattern matching

### Metadata Null Handling
The GraphQL input `metadata_json: Option<String>` must be converted to `Option<Option<String>>` for the database query:
- `None` → `None` (field not updated)
- `Some(string_value)` → `Some(Some(string_value))` (set to JSON value)
- To set to NULL, the caller would need to pass an explicit null value, which GraphQL handles automatically

## Testing

### Test Cases to Implement
Following the pattern from `update_site.rs` and `update_role.rs`:

1. **Success case**: Update user with all fields
    - Create admin user with can_edit_user permission
    - Create target user
    - Call mutation with id, name, roleId, password, passwordConfirmation, metadataJson
    - Verify all fields updated correctly
    - Verify password hash changed

2. **Partial update**: Update only name field
    - Create admin and target users
    - Call mutation with only id and name
    - Verify only name changed

3. **Partial update**: Update only roleId
    - Create admin, target user, and multiple roles
    - Call mutation with only id and roleId
    - Verify only roleId changed

4. **Partial update**: Update only password (with confirmation)
    - Create admin and target users
    - Call mutation with id, password, and matching passwordConfirmation
    - Verify password hash changed, other fields unchanged

5. **Partial update**: Update only metadata
    - Create admin and target users with existing metadata
    - Call mutation with only id and metadataJson
    - Verify metadata changed, other fields unchanged

6. **Authentication error**: Unauthenticated request
   - Create target user
   - Create session context without user
   - Call mutation
   - Verify error message "Authentication required"

7. **Authorization error**: User without can_edit_user permission
   - Create regular user without can_edit_user permission
   - Create target user
   - Call mutation as regular user
   - Verify error message "Forbidden"

8. **User not found**: Update non-existent user
   - Create admin user
   - Call mutation with non-existent user ID
   - Verify error contains "not found"

9. **Validation error**: Invalid username (too short)
   - Create admin and target users
   - Call mutation with name shorter than 3 characters
   - Verify error contains "Validation error"

10. **Validation error**: Invalid password (too short)
    - Create admin and target users
    - Call mutation with password shorter than 6 characters
    - Verify error contains "Validation error"

11. **Username conflict**: Update to existing username
    - Create admin user
    - Create two users with different usernames
    - Update one user to have the other's username
    - Verify error contains "already in use"

12. **No updates**: Call mutation with all optional fields as None
     - Create admin and target users
     - Call mutation with only id (all other fields None)
     - Verify user unchanged (except possibly timestamp)

13. **Password confirmation validation**: Only password provided
     - Create admin and target users
     - Call mutation with password but no passwordConfirmation
     - Verify error contains "must both be provided"

14. **Password confirmation validation**: Only passwordConfirmation provided
     - Create admin and target users
     - Call mutation with passwordConfirmation but no password
     - Verify error contains "must both be provided"

15. **Password confirmation validation**: Passwords don't match
     - Create admin and target users
     - Call mutation with password="newpass123" and passwordConfirmation="different123"
     - Verify error contains "do not match"

16. **Password update with valid confirmation**: Both provided and match
     - Create admin and target users
     - Call mutation with password="newpass123" and passwordConfirmation="newpass123"
     - Verify password hash changed

### Test Implementation Pattern
Use `create_test_mutation_schema` from test_utils. Orchestrator tests need to construct `UpdateUserInput`:

```rust
#[tokio::test]
async fn test_update_user_success() {
  let (pool, _temp_file) = create_test_database().await;

  // Setup: Create admin role with can_edit_user permission
  let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
  let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

  // Create target user
  let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
  let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

  // Create session context for admin user
  let session_payload = ServiceSessionPayload {
    sub: admin_user.id,
    iat: 1706356800,
    exp: 1706616000,
  };
  let session_context = SessionContext::new(Some(session_payload));

  let mutation = UpdateUserResolver;
  let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

  let query = r#"
    mutation {
      updateUser(
        id: $USER_ID,
        name: "updateduser",
        roleId: $ROLE_ID,
        password: "newpassword123",
        passwordConfirmation: "newpassword123",
        metadataJson: "{\"updated\": true}"
      ) {
        id
        uuid
        name
        role {
          id
          name
          permissions
        }
        metadataJson
        createdTs
        updatedTs
      }
    }
  "#
  .replace("$USER_ID", &target_user.id.to_string())
  .replace("$ROLE_ID", &admin_role_id.to_string());

  let result = schema.execute(query).await;
  assert!(result.errors.is_empty());

  let data = result.data.into_json().unwrap();
  let user_data = &data["updateUser"];

  assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
  assert_eq!(user_data["name"].as_str().unwrap(), "updateduser");
  assert_eq!(user_data["role"]["id"].as_i64().unwrap(), admin_role_id);
  assert_eq!(user_data["role"]["name"].as_str().unwrap(), "admin");
  assert!(user_data["createdTs"].as_i64().unwrap() > 0);
  assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
}
```

**For orchestrator unit tests**, construct `UpdateUserInput` and pass to orchestrator:
```rust
use crate::types::user::update_user_input::UpdateUserInput;

let input = UpdateUserInput {
  id: target_user.id,
  name: Some("updatedname".to_string()),
  role_id: Some(admin_role_id),
  password: Some("newpassword123".to_string()),
  password_confirmation: Some("newpassword123".to_string()),
  metadata_json: Some(r#"{"updated": true}"#.to_string()),
};

let result = UserOrchestrator::update_user_with_permission_check(
  &pool,
  session_context,
  input,
).await;
```

## Notes

### Password Security
- Both `password` and `password_confirmation` fields are skipped from logging via `#[instrument]` macro in resolver
- The resolver uses `skip(ctx, password, password_confirmation)` to prevent logging sensitive data
- The orchestrator also does not log password values (already configured)
- Requiring password confirmation minimizes accidental password updates by admins

### Ideal Architecture Pattern
This plan establishes an "ideal pattern" for future resolver/orchestrator refactoring:

**Current (Problematic) Pattern:**
```
Resolver Layer         → Individual GraphQL args
      ↓ manually creates low-level types
Orchestrator Layer    → Takes low-level queries-layer types
      ↓
Service Layer          → Takes low-level queries-layer types
      ↓
Query Layer           → Low-level DB types
```

**Ideal Pattern (Established by this plan):**
```
src/types/
  user/
    update_user_input.rs      → Application-level input types (shared across app)
  role/                           // Future: Role-related types
  site/                           // Future: Site-related types
  ...
      ↓
Resolver Layer         → Individual GraphQL args (for GraphQL schema)
      ↓ creates src/types/user/update_user_input::UpdateUserInput
Orchestrator Layer    → Takes src/types/user/update_user_input::UpdateUserInput
      ↓ converts to low-level types (temporary workaround)
Service Layer          → Takes low-level types
      ↓
Query Layer           → Low-level DB types
```

**Implementation for this PR:**
- `src/types/`: New module structure with domain-based organization (user/, role/, site/, etc.)
- Resolver: Constructs `UpdateUserInput` from GraphQL args using `crate::types::user::update_user_input::UpdateUserInput`
- Orchestrator: Takes `UpdateUserInput`, converts to `UpdateUserData` (low-level)
- Service/Query: Unchanged (still use `UpdateUserData`)

**Future Refactor:**
When refactoring other resolvers/orchestrators to follow this pattern:
1. Service layer methods should take high-level input types (not queries-layer types)
2. Query layer methods should convert high-level types to low-level DB types
3. Remove temporary conversion logic from orchestrator layer
4. Each layer has a clear boundary with appropriate data types

This approach:
- Establishes to correct pattern without requiring a large breaking change
- Keeps service and query layers unchanged (scoped to this PR)
- Makes it easy to identify where conversion should happen in future refactors
- Follows separation of concerns: resolver (GraphQL), orchestrator (app logic), service (business logic), query (data access)

### Domain-Based Type Organization
The `src/types/` directory uses domain-based subdirectories for long-term maintainability:

```
src/types/
  mod.rs          // Top-level exports
  user/
    mod.rs          // User type exports
    update_user_input.rs  // User update input
    user_create_input.rs  // Future: User creation input
    // ... more user-related types
  role/
    mod.rs          // Role type exports
    // ... role-related types
  site/
    mod.rs          // Site type exports
    // ... site-related types
```

**Benefits:**
- Types are grouped by domain (user, role, site, etc.)
- Easy to find related types
- Prevents `types/` directory from becoming unmanageable with many files
- Clear separation: each domain has its own subdirectory
- Re-exports happen at each domain level (`src/types/user/mod.rs`) and top level (`src/types/mod.rs`)

### Why Password Remains Separate from UpdateUserData
The orchestrator still passes separate `password` and `password_confirmation` to `UserService::update_user` because:

1. **Clear responsibility separation**:
   - `UpdateUserData` represents database-ready fields (includes `password_hash`)
   - `password` parameter represents user input needing transformation
   - Service handles the transformation in one place

2. **Avoids confusion**:
   - Having both `password` (plain text) and `password_hash` (hashed) in same struct would be confusing
   - Current design makes it explicit: password needs special handling

3. **Password is fundamentally different**:
   - Requires validation (length check)
   - Requires hashing before storage
   - Should never be logged
   - Not just another field to update

**In the ideal pattern** (future refactor), the service would accept a high-level input type with `password` field and handle all validation/hashing internally. For now, orchestrator does the conversion as a temporary workaround.

### Orchestrator Method Signature
The orchestrator method at `src/orchestrators/user_orchestrator.rs:184` needs to be updated to:

1. Define high-level `UpdateUserInput` type within the orchestrator module
2. Accept `UpdateUserInput` instead of individual parameters
3. Return `UserWithRole` instead of `User`

```rust
pub async fn update_user_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    input: UpdateUserInput,  // High-level input type
) -> Result<UserWithRole, UserError>
```

This signature:
1. Accepts a single high-level input object containing all update fields
2. Validates password and password_confirmation match internally
3. Converts to low-level `UpdateUserData` for service layer (temporary workaround)
4. Fetches updated user with role details before returning

The resolver constructs `UpdateUserInput` from GraphQL arguments and passes it to the orchestrator without any conversion.

### Response Type Consistency
The `UpdateUserResponse` now includes a `role` object with id, name, and permissions. This is consistent with other resolver responses like `authMe`, `user`, and `users` which also include role information.

The orchestrator now returns `UserWithRole` (matching `get_user_details_with_permission_check` and `list_users_with_permission_check`), ensuring consistency across the orchestration layer.
