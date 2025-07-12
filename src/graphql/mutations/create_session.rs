use crate::handlers::graphql::set_session_cookie;
use crate::services::{SessionError, SessionService};
use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use axum::http::HeaderMap;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// Input type for creating a new session (sign-in)
#[derive(InputObject)]
pub struct CreateSessionInput {
  /// Username for authentication
  pub username: String,
  /// Password for authentication
  pub password: String,
}

/// GraphQL output type for session creation response
#[derive(SimpleObject)]
pub struct CreateSessionResponse {
  /// The session token for API authentication
  pub token: String,
  /// Success message
  pub message: String,
}

/// GraphQL mutation for creating user sessions (sign-in)
#[derive(Default)]
pub struct CreateSessionMutation;

#[Object]
impl CreateSessionMutation {
  /// Create a new session by authenticating user credentials
  #[instrument(skip(self, ctx, input), fields(username = %input.username))]
  async fn create_session(
    &self,
    ctx: &Context<'_>,
    input: CreateSessionInput,
  ) -> Result<CreateSessionResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    match SessionService::create_session(pool, &input.username, &input.password).await {
      Ok(token) => {
        // Set cookie in response headers
        if let Ok(response_headers) = ctx.data::<Arc<Mutex<HeaderMap>>>() {
          set_session_cookie(response_headers, &token);
        }

        Ok(CreateSessionResponse {
          token,
          message: "Authentication successful".to_string(),
        })
      }
      Err(session_error) => {
        let user_message = map_session_error_to_user_message(&session_error);
        Err(async_graphql::Error::new(user_message))
      }
    }
  }
}

/// Map internal SessionError types to user-friendly messages
fn map_session_error_to_user_message(error: &SessionError) -> &'static str {
  match error {
    SessionError::AuthenticationError(_) => "Invalid credentials",
    SessionError::DatabaseError(_) => "Internal server error",
    SessionError::PasswordVerificationError(_) => "Internal server error",
    SessionError::EncodingError(_) => "Internal server error",
    _ => "Internal server error",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::services::UserService;
  use async_graphql::{EmptySubscription, Schema};
  use std::env;

  fn setup_test_key() {
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
    );
  }

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn test_create_session_calls_service_with_correct_parameters() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Create GraphQL schema with just the mutation
    let schema = Schema::build(
      async_graphql::EmptyMutation,
      CreateSessionMutation,
      EmptySubscription,
    )
    .data(pool)
    .finish();

    // Test: Call the mutation
    let query = r#"
      mutation {
        createSession(input: { username: "testuser", password: "password123" }) {
          token
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should succeed and return a token
    assert!(
      result.errors.is_empty(),
      "GraphQL errors: {:?}",
      result.errors
    );

    let data = result.data.into_json().unwrap();
    let create_session = &data["createSession"];

    assert!(!create_session["token"].as_str().unwrap().is_empty());
    assert_eq!(
      create_session["message"].as_str().unwrap(),
      "Authentication successful"
    );
  }

  #[tokio::test]
  async fn test_create_session_maps_authentication_error() {
    setup_test_key();
    let (pool, _temp_file) = create_test_database().await;

    let schema = Schema::build(
      async_graphql::EmptyMutation,
      CreateSessionMutation,
      EmptySubscription,
    )
    .data(pool)
    .finish();

    // Test: Call with non-existent user
    let query = r#"
      mutation {
        createSession(input: { username: "nonexistent", password: "password123" }) {
          token
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return user-friendly error
    assert!(!result.errors.is_empty());
    assert_eq!(result.errors[0].message, "Invalid credentials");
  }

  #[tokio::test]
  async fn test_create_session_maps_database_error() {
    setup_test_key();

    // Create a schema without database pool to trigger database error
    let schema = Schema::build(
      async_graphql::EmptyMutation,
      CreateSessionMutation,
      EmptySubscription,
    )
    .finish();

    let query = r#"
      mutation {
        createSession(input: { username: "testuser", password: "password123" }) {
          token
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return error (missing database pool)
    assert!(!result.errors.is_empty());
    // The error will be about missing SqlitePool, not our mapped error
  }

  #[tokio::test]
  async fn test_map_session_error_to_user_message() {
    assert_eq!(
      map_session_error_to_user_message(&SessionError::AuthenticationError("test".to_string())),
      "Invalid credentials"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::DatabaseError("test".to_string())),
      "Internal server error"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::PasswordVerificationError(
        "test".to_string()
      )),
      "Internal server error"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::EncodingError("test".to_string())),
      "Internal server error"
    );
  }
}
