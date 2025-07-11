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

#[cfg(test)]
mod tests {
  use super::*;
  use async_graphql::*;
  use crate::database::test_utils::create_test_database;

  // Minimal query struct for testing mutations in isolation
  #[derive(Default)]
  struct TestEmptyQuery;

  #[Object]
  impl TestEmptyQuery {
    async fn dummy(&self) -> &str {
      "test"
    }
  }

  #[tokio::test]
  async fn test_create_user_calls_service_with_correct_parameters() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert default role first
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)")
      .execute(&pool)
      .await
      .unwrap();

    let mutation = CreateUserMutation;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
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
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
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
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
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