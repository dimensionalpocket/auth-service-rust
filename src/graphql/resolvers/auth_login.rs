use crate::database::Databases;
use crate::graphql::types::{UserRole, UserWithRoleResponse};
use crate::orchestrators::auth::AuthLoginOrchestrator;
use crate::types::SessionError;
use async_graphql::{Context, Error, Object, Result};
use dps_config::DpsConfig;
use std::sync::Arc;
use tracing::instrument;

/// GraphQL output type for authentication response
#[derive(async_graphql::SimpleObject)]
pub struct AuthLoginResponse {
  /// The session token for API authentication
  pub token: String,
  /// The authenticated user's information including role
  pub user: UserWithRoleResponse,
  /// Success message
  pub message: String,
}

/// GraphQL mutation for user authentication (login)
#[derive(Default)]
pub struct AuthLoginResolver;

#[Object]
impl AuthLoginResolver {
  /// Authenticate user credentials and create a session
  #[instrument(skip(self, ctx, password), fields(username = %username))]
  #[graphql(name = "authLogin")]
  async fn auth_login(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "username")] username: String,
    #[graphql(name = "password")] password: String,
  ) -> Result<AuthLoginResponse, Error> {
    let databases = ctx
      .data::<Databases>()
      .map_err(|_| async_graphql::Error::new("Internal server error"))?;
    let config = ctx
      .data::<Arc<DpsConfig>>()
      .map_err(|_| async_graphql::Error::new("Internal server error"))?;

    match AuthLoginOrchestrator::run(databases, &username, &password, config).await {
      Ok((auth_result, cookie_value)) => {
        let _ = ctx.append_http_header("set-cookie", cookie_value);

        Ok(AuthLoginResponse {
          token: auth_result.session_token,
          user: UserWithRoleResponse {
            id: auth_result.user_id,
            name: auth_result.username,
            role: UserRole::from(auth_result.role),
            uuid: None,
            created_ts: None,
            updated_ts: None,
          },
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
    SessionError::AuthSessionError(_) => "Internal server error",
    SessionError::ConfigurationError(_) => "Internal server error",
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{
    create_test_dps_config, create_test_mutation_schema, create_test_role_model_with_conn,
    TestEmptyQuery,
  };
  use async_graphql::{EmptySubscription, Schema};

  #[dps_auth_db_test]
  async fn test_auth_login_calls_service_with_correct_parameters() {
    // Setup: Create a user
    {
      let mut main_conn = main_pool.acquire().await.unwrap();
      create_test_role_model_with_conn(&mut main_conn, "user", &["can_view_user_self"], true).await;
      CreateUserService::run(&mut main_conn, "testuser", "password123")
        .await
        .unwrap();
    }

    // Create GraphQL schema with just the mutation
    let test_config = create_test_dps_config();
    let schema = create_test_mutation_schema(
      AuthLoginResolver,
      databases.clone(),
      None,
      Some(test_config),
    );

    // Test: Call the mutation
    let query = r#"
      mutation {
        authLogin(username: "testuser", password: "password123") {
          token
          user {
            id
            name
            role {
              id
              name
              permissions
            }
          }
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
    let auth_login = &data["authLogin"];

    assert!(!auth_login["token"].as_str().unwrap().is_empty());
    assert!(auth_login["user"]["id"].as_i64().unwrap() > 0);
    assert_eq!(auth_login["user"]["name"].as_str().unwrap(), "testuser");
    assert_eq!(auth_login["user"]["role"]["name"].as_str().unwrap(), "user");
    assert_eq!(
      auth_login["message"].as_str().unwrap(),
      "Authentication successful"
    );
  }

  #[dps_auth_db_test]
  async fn test_auth_login_maps_authentication_error() {
    let test_config = create_test_dps_config();
    let schema = create_test_mutation_schema(
      AuthLoginResolver,
      databases.clone(),
      None,
      Some(test_config),
    );

    // Test: Call with non-existent user
    let query = r#"
      mutation {
        authLogin(username: "testuser", password: "password123") {
          token
          user {
            id
            name
          }
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Should return user-friendly error message
    assert!(!result.errors.is_empty());
    assert_eq!(result.errors[0].message, "Invalid credentials");
  }

  #[dps_auth_db_test]
  async fn test_auth_login_maps_database_error() {
    // Create a schema without Databases schema data to trigger database error
    let test_config = create_test_dps_config();
    // NOTE: We intentionally do not inject `databases` here.
    let schema = Schema::build(TestEmptyQuery, AuthLoginResolver, EmptySubscription)
      .data(test_config)
      .finish();

    let query = r#"
      mutation {
        authLogin(username: "testuser", password: "password123") {
          token
          user {
            id
            name
          }
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return user-friendly internal server error
    assert!(!result.errors.is_empty());

    // The error should be mapped to a user-friendly message
    let error_message = &result.errors[0].message;
    assert_eq!(
      error_message, "Internal server error",
      "Expected 'Internal server error' for missing databases, but got: {error_message}"
    );

    // Verify it's not our mapped authentication error
    assert!(
      !error_message.contains("Invalid credentials"),
      "Should not get authentication error when database is missing"
    );
  }

  #[dps_auth_db_test]
  async fn test_map_session_error_to_user_message() {
    use dps_auth_session::DpsAuthSessionError;

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
      map_session_error_to_user_message(&SessionError::AuthSessionError(
        DpsAuthSessionError::EncodingError("test".to_string())
      )),
      "Internal server error"
    );
  }
}
