# Plan: Update UserWithRoleResponse Field Names and Refactor authMe Resolver

## Overview
Update `UserWithRoleResponse` field names to match the naming convention used in other resolvers (`id` instead of `userId`, `name` instead of `username`), and refactor `authMe` to return the user in a nested `user` property alongside session timestamp fields.

## Changes Required

### Phase 1: Update UserWithRoleResponse Type Definition

**File:** `src/graphql/types/user_response.rs`

Update the `UserWithRoleResponse` struct to rename fields:
- Change `user_id` to `id` with `#[graphql(name = "id")]` annotation
- Change `username` to `name` with `#[graphql(name = "name")]` annotation

```rust
pub struct UserWithRoleResponse {
  /// The user's ID
  #[graphql(name = "id")]
  pub id: i64,
  /// The user's username
  #[graphql(name = "name")]
  pub name: String,
  /// The user's role information
  pub role: UserRole,
  /// The user's UUID (public identifier) - optional for register responses
  pub uuid: Option<String>,
  /// Timestamp when the user was created - optional for login responses
  #[graphql(name = "createdTs")]
  pub created_ts: Option<i64>,
  /// Timestamp when the user was last updated - optional for login responses
  #[graphql(name = "updatedTs")]
  pub updated_ts: Option<i64>,
}
```

### Phase 2: Update authRegister Resolver

**File:** `src/graphql/resolvers/auth_register.rs`

Update the `AuthRegisterResponse::auth_register` method to use new field names:
- Change `user_id: register_result.user_id` to `id: register_result.user_id`
- Change `username: register_result.username` to `name: register_result.username`

Update tests in `#[cfg(test)]` module to query `id` and `name` instead of `userId` and `username`:
- Update GraphQL queries in all tests to use `id` and `name`
- Update assertions to check `id` and `name` fields

### Phase 3: Update authLogin Resolver

**File:** `src/graphql/resolvers/auth_login.rs`

Update the `AuthLoginResolver::auth_login` method to use new field names:
- Change `user_id: auth_result.user_id` to `id: auth_result.user_id`
- Change `username: auth_result.username` to `name: auth_result.username`

Update tests in `#[cfg(test)]` module to query `id` and `name` instead of `userId` and `username`:
- Update GraphQL queries in all tests to use `id` and `name`
- Update assertions to check `id` and `name` fields

### Phase 4: Refactor authMe Resolver

**File:** `src/graphql/resolvers/auth_me.rs`

#### 4a. Update AuthMeResponse struct

Create a new response type that nests user data:

```rust
#[derive(async_graphql::SimpleObject)]
pub struct AuthMeResponse {
  /// The authenticated user's information
  pub user: UserWithRoleResponse,
  /// When the current session was created (seconds since Unix epoch)
  #[graphql(name = "sessionIat")]
  pub session_iat: i64,
  /// When the current session expires (seconds since Unix epoch)
  #[graphql(name = "sessionExp")]
  pub session_exp: i64,
}
```

#### 4b. Update AuthMeResolver::auth_me method

Remove the old `AuthMeResponse` definition and update the resolver implementation:

```rust
#[instrument(skip(self, ctx))]
#[graphql(name = "authMe")]
async fn auth_me(&self, ctx: &Context<'_>) -> Result<Option<AuthMeResponse>> {
  let pool = ctx.data::<sqlx::SqlitePool>()?;

  // Try to get session context, but don't fail if it's missing
  let session_context = match SessionContext::from_context(ctx) {
    Ok(context) => context,
    Err(_) => return Ok(None),
  };

  match AuthOrchestrator::get_authenticated_user(pool, session_context.clone()).await {
    Ok(Some(auth_me_result)) => Ok(Some(AuthMeResponse {
      user: UserWithRoleResponse {
        id: auth_me_result.user_id,
        name: auth_me_result.username,
        role: UserRole::from(auth_me_result.role),
        uuid: Some(auth_me_result.uuid),
        created_ts: Some(auth_me_result.created_ts),
        updated_ts: Some(auth_me_result.updated_ts),
      },
      session_iat: auth_me_result.session_iat,
      session_exp: auth_me_result.session_exp,
    })),
    Ok(None) => Ok(None),
    Err(session_error) => {
      let user_message = map_session_error_to_user_message(&session_error);
      Err(async_graphql::Error::new(user_message))
    }
  }
}
```

#### 4c. Update documentation comments

Update the resolver documentation and examples to reflect the new nested structure:

```rust
/// Returns information about the currently authenticated user.
///
/// This query retrieves detailed user information for the authenticated user,
/// including both user profile data and current session information.
/// The response contains:
/// - User profile (nested): ID, UUID, username, role information (id, name, permissions), timestamps
/// - Session data: When session was created and when it expires
///
/// Returns `null` if no valid session token was provided in the request.
///
/// # Authentication
///
/// This query requires a valid session token to be provided either:
/// - In the `Authorization` header as `Bearer <token>`
/// - In the `DpsAuthSession` cookie
///
/// # Examples
///
/// **Authenticated request:**
/// ```graphql
/// query {
///   authMe {
///     user {
///       id
///       uuid
///       name
///       role {
///         id
///         name
///         permissions
///       }
///       createdTs
///       updatedTs
///     }
///     sessionIat
///     sessionExp
///   }
/// }
/// ```
///
/// **Response for authenticated user:**
/// ```json
/// {
///   "data": {
///     "authMe": {
///       "user": {
///         "id": 123,
///         "uuid": "550e8400-e29b-41d4-a716-446655440000",
///         "name": "johndoe",
///         "role": {
///           "id": "2",
///           "name": "user",
///           "permissions": ["can_view_user_self"]
///         },
///         "createdTs": 1706356800,
///         "updatedTs": 1706356800
///       },
///       "sessionIat": 1706356800,
///       "sessionExp": 1706616000
///     }
///   }
/// }
/// ```
///
/// **Response for unauthenticated user:**
/// ```json
/// {
///   "data": {
///     "authMe": null
///   }
/// }
/// ```
```

#### 4d. Update tests

Update all tests in the `#[cfg(test)]` module to query the new nested structure:

```rust
#[tokio::test]
async fn test_auth_me_with_authenticated_user() {
  // ... (existing setup code) ...

  let result = schema
    .execute(
      "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }",
    )
    .await;

  assert!(result.errors.is_empty());
  let data = result.data.into_json().unwrap();
  assert_eq!(data["authMe"]["user"]["id"], user_id);
  assert_eq!(data["authMe"]["user"]["name"], "testuser");
  assert_eq!(data["authMe"]["user"]["role"]["name"], "user");
  assert_eq!(data["authMe"]["sessionIat"], 1706356800);
  assert_eq!(data["authMe"]["sessionExp"], 1706616000);
}

#[tokio::test]
async fn test_auth_me_with_unauthenticated_user() {
  // ... update query to use nested structure ...
  let result = schema.execute("{ authMe { user { id uuid name } } }").await;
  // ... (rest remains the same)
}

#[tokio::test]
async fn test_auth_me_missing_context() {
  // ... update query to use nested structure ...
  let result = schema.execute("{ authMe { user { id uuid name } } }").await;
  // ... (rest remains the same)
}
```

### Phase 5: Update README GraphQL Table

**File:** `README.md`

Update the Queries table entries for:

1. **authMe** query (line 32):
   Change from:
   ```
   | `authMe` | Get current authenticated user profile. Returns userId (Int), uuid (String), username (String), role { id (String), name (String), permissions ([String]) }, createdTs (Int), updatedTs (Int), sessionIat (Int), sessionExp (Int). Requires valid session cookie. |
   ```
   To:
   ```
   | `authMe` | Get current authenticated user profile. Returns user { id (Int), uuid (String), name (String), role { id (String), name (String), permissions ([String]) }, createdTs (Int), updatedTs (Int) }, sessionIat (Int), sessionExp (Int). Requires valid session cookie. |
   ```

2. **authRegister** mutation (line 45):
   Change from:
   ```
   | `authRegister` | Register new user account. Input: username (String!), password (String!), passwordConfirmation (String!). Returns: user { userId (Int), uuid (String), username (String), role { id (String), name (String), permissions ([String]) }, createdTs (Int), updatedTs (Int) }, message (String). No authentication required. |
   ```
   To:
   ```
   | `authRegister` | Register new user account. Input: username (String!), password (String!), passwordConfirmation (String!). Returns: user { id (Int), uuid (String), name (String), role { id (String), name (String), permissions ([String]) }, createdTs (Int), updatedTs (Int) }, message (String). No authentication required. |
   ```

3. **authLogin** mutation (line 46):
   Change from:
   ```
   | `authLogin` | Authenticate user and create session. Input: username (String!), password (String!). Returns: token (String), user { userId (Int), username (String), role { id (String), name (String), permissions ([String]) } }, message (String). No authentication required (sets cookie). |
   ```
   To:
   ```
   | `authLogin` | Authenticate user and create session. Input: username (String!), password (String!). Returns: token (String), user { id (Int), name (String), role { id (String), name (String), permissions ([String]) } }, message (String). No authentication required (sets cookie). |
   ```

## Implementation Notes

- No new crates or dependencies are required
- Field renames are straightforward - only GraphQL output names change
- The nested structure for `authMe` is more consistent with other mutations that return user data
- All existing service layer code remains unchanged - only GraphQL presentation layer is updated
- Test updates ensure the new field names and structure are properly validated

## Testing Strategy

After each phase:
1. Run `cargo test --quiet` to ensure all tests pass
2. Verify GraphQL schema reflects the changes by running the local server and checking the playground

After all phases complete:
1. Run `cargo test --quiet` to verify all tests pass
2. Run `cargo clippy --allow-dirty --fix && cargo fmt` to fix any linting issues
