use crate::graphql::types::{UserRole, UserWithRoleResponse};
use crate::orchestrators::auth::AuthLoginOrchestrator;
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Error, Object, Result};
use sqlx::SqlitePool;
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
    let pool = ctx
      .data::<SqlitePool>()
      .map_err(|_| async_graphql::Error::new("Internal server error"))?;
    let config = ctx
      .data::<DpsAuthApiConfig>()
      .map_err(|_| async_graphql::Error::new("Internal server error"))?;

    match AuthLoginOrchestrator::run(pool, &username, &password, config).await {
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
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{
    create_test_database, create_test_mutation_schema, create_test_role_model,
  };

  // Test secret - 32 bytes for AES-256 (base64-decoded from QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=)
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  #[tokio::test]
  async fn test_auth_login_calls_service_with_correct_parameters() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Setup: Create a user
    {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      CreateUserService::run(&mut conn, "testuser", "password123")
        .await
        .unwrap();
    }

    // Create GraphQL schema with just the mutation
    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: TEST_SECRET.to_vec(),
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };
    let schema =
      create_test_mutation_schema(AuthLoginResolver, Some(pool), None, Some(test_config));

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

  #[tokio::test]
  async fn test_auth_login_maps_authentication_error() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: TEST_SECRET.to_vec(),
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };
    let schema =
      create_test_mutation_schema(AuthLoginResolver, Some(pool), None, Some(test_config));

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

  #[tokio::test]
  async fn test_auth_login_maps_database_error() {
    // Create a schema without database pool to trigger database error
    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: TEST_SECRET.to_vec(),
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };
    let schema = create_test_mutation_schema(AuthLoginResolver, None, None, Some(test_config));

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
      "Expected 'Internal server error' for missing database pool, but got: {error_message}"
    );

    // Verify it's not our mapped authentication error
    assert!(
      !error_message.contains("Invalid credentials"),
      "Should not get authentication error when database is missing"
    );
  }

  #[tokio::test]
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
