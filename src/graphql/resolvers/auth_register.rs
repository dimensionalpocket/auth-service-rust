use crate::services::{AuthService, UserError};
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for user registration response
#[derive(async_graphql::SimpleObject)]
pub struct AuthRegisterResponse {
  /// The registered user's ID
  #[graphql(name = "userId")]
  pub user_id: i64,
  /// The registered user's UUID (public identifier)
  pub uuid: String,
  /// The registered user's username
  pub username: String,
  /// The registered user's role ID
  #[graphql(name = "roleId")]
  pub role_id: i64,
  /// Timestamp when the user was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the user was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
  /// Success message
  pub message: String,
}

/// GraphQL mutation for user registration
#[derive(Default, Debug)]
pub struct AuthRegisterResolver;

#[Object]
impl AuthRegisterResolver {
  /// Register a new user account with the provided username, password, and password confirmation.
  ///
  /// This mutation:
  /// - Validates the input (username, password, and password confirmation)
  /// - Checks if the username is already in use (case-insensitive)
  /// - Verifies that password and password confirmation match
  /// - Hashes the password using Argon2
  /// - Assigns the default user role
  /// - Creates the user in the database
  /// - Returns the created user information (without password hash)
  ///
  /// # Arguments
  /// * `username` - Username for new user account (must be unique)
  /// * `password` - Password for new user account (will be hashed)
  /// * `password_confirmation` - Password confirmation to ensure password is entered correctly
  ///
  /// # Returns
  /// * `AuthRegisterResponse` - The registered user information
  ///
  /// # Errors
  /// * Returns GraphQL error if username already exists
  /// * Returns GraphQL error if password and confirmation don't match
  /// * Returns GraphQL error if input validation fails
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(self, ctx), fields(username = %username))]
  #[graphql(name = "authRegister")]
  async fn auth_register(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "username")] username: String,
    #[graphql(name = "password")] password: String,
    #[graphql(name = "passwordConfirmation")] password_confirmation: String,
  ) -> Result<AuthRegisterResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    match AuthService::register(pool, &username, &password, &password_confirmation).await {
      Ok(register_result) => Ok(AuthRegisterResponse {
        user_id: register_result.user_id,
        uuid: register_result.uuid,
        username: register_result.username,
        role_id: register_result.role_id,
        created_ts: register_result.created_ts,
        updated_ts: register_result.updated_ts,
        message: "User registration successful".to_string(),
      }),
      Err(UserError::UsernameAlreadyExists(username)) => Err(async_graphql::Error::new(format!(
        "Username '{username}' is already in use"
      ))),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(err) => {
        tracing::error!("Failed to register user: {}", err);
        Err(async_graphql::Error::new("Failed to register user"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use async_graphql::*;

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
  async fn test_auth_register_calls_service_with_correct_parameters() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          userId
          uuid
          username
          roleId
          createdTs
          updatedTs
          message
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["authRegister"];

    assert!(user_data["userId"].as_i64().unwrap() > 0);
    assert!(!user_data["uuid"].as_str().unwrap().is_empty());
    assert_eq!(user_data["username"].as_str().unwrap(), "testuser");
    assert!(user_data["roleId"].as_i64().unwrap() > 0);
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
    assert_eq!(
      user_data["message"].as_str().unwrap(),
      "User registration successful"
    );
  }

  #[tokio::test]
  async fn test_auth_register_returns_error_for_duplicate_username() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          userId
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
  async fn test_auth_register_validates_input() {
    let (pool, _temp_file) = create_test_database().await;

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    // Test empty username
    let query = r#"
      mutation {
        authRegister(username: "", password: "testpass123", passwordConfirmation: "testpass123") {
          userId
          username
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
  }

  #[tokio::test]
  async fn test_auth_register_password_confirmation_mismatch() {
    let (pool, _temp_file) = create_test_database().await;

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .finish();

    // Test password confirmation mismatch
    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "differentpass") {
          userId
          username
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Passwords do not match"));
  }
}
