# Phase 7: createUser GraphQL Mutation

**Date**: 2025-07-11  
**Phase**: 7 of ongoing phases  
**Goal**: Implement the `createUser` GraphQL mutation with comprehensive testing

## Overview

This phase implements the `createUser` GraphQL mutation that allows clients to create new users through the GraphQL API. The mutation will follow the established project patterns by calling the existing `UserService::create_user` method and returning the created user object.

## Tasks Breakdown

### 1. Create Mutations Infrastructure

Since this is the first mutation in the project, we need to establish the mutations infrastructure:

#### 1.1 Create mutations directory structure
- Create `src/graphql/mutations/` directory
- Create `src/graphql/mutations/mod.rs` file

#### 1.2 Update GraphQL module structure
- Update `src/graphql/mod.rs` to include mutations module
- Create mutation root object similar to the existing query structure

### 2. Implement createUser Mutation

#### 2.1 Create the mutation file
Create `src/graphql/mutations/create_user.rs` with:

```rust
use crate::models::User;
use crate::services::{UserService, UserError};
use async_graphql::{Context, Object, Result, InputObject};
use sqlx::SqlitePool;
use tracing::instrument;

/// Input type for creating a new user
#[derive(InputObject)]
pub struct CreateUserInput {
  /// Username for the new user (must be unique)
  pub username: String,
  /// Password for the new user (will be hashed)
  pub password: String,
}

/// GraphQL output type for user creation response
#[derive(async_graphql::SimpleObject)]
pub struct CreateUserResponse {
  /// The created user's UUID (public identifier)
  pub uuid: String,
  /// The created user's username
  pub username: String,
  /// The created user's role ID
  pub role_id: i64,
  /// Timestamp when the user was created
  pub created_ts: i64,
  /// Timestamp when the user was last updated
  pub updated_ts: i64,
}

/// User creation mutation resolver
#[derive(Default, Debug)]
pub struct CreateUserMutation;

#[Object]
impl CreateUserMutation {
  /// Creates a new user with the provided username and password.
  ///
  /// This mutation:
  /// - Validates the input (username and password)
  /// - Checks if the username is already in use (case-insensitive)
  /// - Hashes the password using Argon2
  /// - Assigns the default user role
  /// - Creates the user in the database
  /// - Returns the created user information (without password hash)
  ///
  /// # Arguments
  /// * `input` - CreateUserInput containing username and password
  ///
  /// # Returns
  /// * `CreateUserResponse` - The created user information
  ///
  /// # Errors
  /// * Returns GraphQL error if username already exists
  /// * Returns GraphQL error if input validation fails
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx, input), fields(username = %input.username))]
  async fn create_user(
    &self,
    ctx: &Context<'_>,
    input: CreateUserInput,
  ) -> Result<CreateUserResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    
    match UserService::create_user(pool, &input.username, &input.password).await {
      Ok(user) => Ok(CreateUserResponse {
        uuid: user.uuid,
        username: user.name,
        role_id: user.role_id,
        created_ts: user.created_ts,
        updated_ts: user.updated_ts,
      }),
      Err(UserError::UsernameAlreadyExists(username)) => {
        Err(async_graphql::Error::new(format!("Username '{}' is already in use", username)))
      },
      Err(UserError::ValidationError(msg)) => {
        Err(async_graphql::Error::new(format!("Validation error: {}", msg)))
      },
      Err(err) => {
        tracing::error!("Failed to create user: {}", err);
        Err(async_graphql::Error::new("Failed to create user"))
      }
    }
  }
}
```

#### 2.2 Update mutations module
Update `src/graphql/mutations/mod.rs`:

```rust
pub mod create_user;

pub use create_user::CreateUserMutation;
```

### 3. Update GraphQL Schema

#### 3.1 Create mutation root object
Create `src/graphql/mutation.rs`:

```rust
use crate::graphql::mutations::CreateUserMutation;
use async_graphql::MergedObject;

/// Root mutation object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL mutations. It combines all
/// individual mutation resolvers into a single unified interface using MergedObject.
///
/// Available mutations:
/// - createUser: Create a new user account
///
/// Future mutations will be added here as the service expands to include
/// user management, authentication, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Mutation(CreateUserMutation);

impl Mutation {
  pub fn new() -> Self {
    Self(CreateUserMutation)
  }
}
```

#### 3.2 Update GraphQL module
Update `src/graphql/mod.rs`:

```rust
pub mod mutations;
pub mod mutation;
pub mod queries;
pub mod query;
pub mod schema;
```

#### 3.3 Update schema creation
Update `src/graphql/schema.rs`:

```rust
use crate::graphql::query::Query;
use crate::graphql::mutation::Mutation;
use async_graphql::{EmptySubscription, Schema};

/// GraphQL schema type definition for the Dimensional Pocket Auth Service
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Creates and returns the complete GraphQL schema for the auth service.
///
/// This function initializes the GraphQL schema with:
/// - Query resolver: Handles all read operations (server timestamp, user queries)
/// - Mutation resolver: Handles all write operations (user creation, etc.)
/// - EmptySubscription: Placeholder for future real-time features
///
/// The schema is fully introspectable and self-documenting through GraphQL's
/// built-in introspection system, accessible via GraphQL Playground in development.
pub fn create_schema() -> AppSchema {
  Schema::build(Query::new(), Mutation::new(), EmptySubscription).finish()
}
```

### 4. Unit Tests for createUser Mutation

Create comprehensive unit tests in `src/graphql/mutations/create_user.rs`:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use async_graphql::*;
  use crate::database::create_test_database;

  #[tokio::test]
  async fn test_create_user_calls_service_with_correct_parameters() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert default role first
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(&pool)
      .await
      .unwrap();

    let mutation = CreateUserMutation;
    let schema = Schema::build(EmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    let query = r#"
      mutation {
        createUser(input: { username: "testuser", password: "testpass123" }) {
          uuid
          username
          roleId
          createdTs
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let user_data = &data["createUser"];
    
    assert!(!user_data["uuid"].as_str().unwrap().is_empty());
    assert_eq!(user_data["username"].as_str().unwrap(), "testuser");
    assert!(user_data["roleId"].as_i64().unwrap() > 0);
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[tokio::test]
  async fn test_create_user_returns_error_for_duplicate_username() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert default role first
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(&pool)
      .await
      .unwrap();

    let mutation = CreateUserMutation;
    let schema = Schema::build(EmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    let query = r#"
      mutation {
        createUser(input: { username: "testuser", password: "testpass123" }) {
          uuid
          username
        }
      }
    "#;

    // First creation should succeed
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    // Second creation with same username should fail
    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("already in use"));
  }

  #[tokio::test]
  async fn test_create_user_validates_input() {
    let (pool, _temp_file) = create_test_database().await;
    
    let mutation = CreateUserMutation;
    let schema = Schema::build(EmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    // Test empty username
    let query = r#"
      mutation {
        createUser(input: { username: "", password: "testpass123" }) {
          uuid
          username
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
  }
}
```

### 5. Integration Tests

Create integration tests in `tests/integration_tests.rs` (add to existing file):

```rust
#[tokio::test]
async fn test_create_user_mutation_integration() {
  let app = create_test_app().await;
  
  let mutation = r#"
    mutation {
      createUser(input: { username: "integrationuser", password: "testpass123" }) {
        uuid
        username
        roleId
        createdTs
        updatedTs
      }
    }
  "#;

  let response = app
    .oneshot(
      Request::builder()
        .method(http::Method::POST)
        .uri("/graphql")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          serde_json::json!({
            "query": mutation
          }).to_string()
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  
  let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
  let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
  
  assert!(json["errors"].is_null());
  let user_data = &json["data"]["createUser"];
  
  assert!(!user_data["uuid"].as_str().unwrap().is_empty());
  assert_eq!(user_data["username"].as_str().unwrap(), "integrationuser");
  assert!(user_data["roleId"].as_i64().unwrap() > 0);
  assert!(user_data["createdTs"].as_i64().unwrap() > 0);
  assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_create_user_mutation_duplicate_username() {
  let app = create_test_app().await;
  
  let mutation = r#"
    mutation {
      createUser(input: { username: "duplicateuser", password: "testpass123" }) {
        uuid
        username
      }
    }
  "#;

  let request_body = serde_json::json!({
    "query": mutation
  }).to_string();

  // First request should succeed
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method(http::Method::POST)
        .uri("/graphql")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.clone()))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  
  let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
  let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
  assert!(json["errors"].is_null());

  // Second request with same username should fail
  let response = app
    .oneshot(
      Request::builder()
        .method(http::Method::POST)
        .uri("/graphql")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  
  let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
  let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
  
  assert!(!json["errors"].is_null());
  assert!(json["errors"][0]["message"].as_str().unwrap().contains("already in use"));
}
```

## Files to be Created/Modified

### New Files
- `src/graphql/mutations/mod.rs` - Mutations module declaration
- `src/graphql/mutations/create_user.rs` - CreateUser mutation implementation
- `src/graphql/mutation.rs` - Root mutation object

### Modified Files
- `src/graphql/mod.rs` - Add mutations module
- `src/graphql/schema.rs` - Replace EmptyMutation with Mutation
- `tests/integration_tests.rs` - Add integration tests for createUser mutation

## Testing Strategy

### Unit Tests
- Test that the mutation calls `UserService::create_user` with correct parameters
- Test error handling for duplicate usernames
- Test input validation
- Test successful user creation response format

### Integration Tests
- Test complete GraphQL mutation flow through HTTP endpoint
- Test error responses are properly formatted
- Test database integration works correctly

## Success Criteria

1. ✅ `createUser` mutation is accessible via GraphQL endpoint
2. ✅ Mutation calls `UserService::create_user` with correct parameters
3. ✅ Returns created user object with all required fields
4. ✅ Proper error handling for duplicate usernames and validation errors
5. ✅ All unit tests pass with 100% coverage for mutation logic
6. ✅ Integration tests demonstrate end-to-end functionality
7. ✅ GraphQL schema introspection shows the new mutation
8. ✅ Logging is properly implemented for the mutation

## Dependencies

This phase depends on:
- ✅ Phase 6: User Service implementation (completed)
- ✅ Phase 5: User Roles and Permissions (completed)
- ✅ Phase 4: Database Configuration (completed)
- ✅ Phase 3: Password Service (completed)
- ✅ Phase 2: Logging (completed)

## Notes

- The mutation follows the established project patterns for GraphQL resolvers
- Error handling converts service errors to appropriate GraphQL errors
- The response type excludes sensitive information like password hashes
- Input validation is handled by the service layer, with GraphQL providing the interface
- Logging includes the username for traceability while excluding sensitive data
- The mutation is designed to be easily testable and maintainable