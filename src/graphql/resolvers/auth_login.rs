use crate::middleware::session::SESSION_COOKIE_NAME;
use crate::services::{AuthService, SessionError};
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Object, Result, SimpleObject};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for authentication response
#[derive(SimpleObject)]
pub struct AuthLoginResponse {
  /// The session token for API authentication
  pub token: String,
  /// The authenticated user's ID
  #[graphql(name = "userId")]
  pub user_id: i64,
  /// The authenticated user's username
  pub username: String,
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
  ) -> Result<AuthLoginResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let config = ctx.data::<DpsAuthApiConfig>()?;
    let session_secret = config.session_secret.clone();
    let cookie_domain = config.cookie_domain.clone();
    let insecure_cookie = config.insecure_cookie;

    match AuthService::login(pool, &username, &password, &session_secret).await {
      Ok(auth_result) => {
        // Set session cookie
        let cookie_value = format!(
          "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
          SESSION_COOKIE_NAME,
          auth_result.session_token,
          cookie_domain,
          config.api_path,
          if insecure_cookie { "" } else { "; Secure" },
          config.session_ttl_seconds
        );

        // Use append to allow multiple cookies; ignore the return value
        let _ = ctx.append_http_header("set-cookie", cookie_value);

        Ok(AuthLoginResponse {
          token: auth_result.session_token,
          user_id: auth_result.user_id,
          username: auth_result.username,
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
  use crate::test_utils::test_utils::create_test_database;
  use crate::services::UserService;
  use async_graphql::{EmptySubscription, Schema};

  // Test secret - 32 bytes for AES-256 (base64-decoded from QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=)
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn test_auth_login_calls_service_with_correct_parameters() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

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
    let schema = Schema::build(
      async_graphql::EmptyMutation,
      AuthLoginResolver,
      EmptySubscription,
    )
    .data(pool)
    .data(test_config)
    .finish();

    // Test: Call the mutation
    let query = r#"
      mutation {
        authLogin(username: "testuser", password: "password123") {
          token
          userId
          username
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
    assert!(auth_login["userId"].as_i64().unwrap() > 0);
    assert_eq!(auth_login["username"].as_str().unwrap(), "testuser");
    assert_eq!(
      auth_login["message"].as_str().unwrap(),
      "Authentication successful"
    );
  }

  #[tokio::test]
  async fn test_auth_login_maps_authentication_error() {
    let (pool, _temp_file) = create_test_database().await;

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
    let schema = Schema::build(
      async_graphql::EmptyMutation,
      AuthLoginResolver,
      EmptySubscription,
    )
    .data(pool)
    .data(test_config)
    .finish();

    // Test: Call with non-existent user
    let query = r#"
      mutation {
        authLogin(username: "testuser", password: "password123") {
          token
          userId
          username
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
    let schema = Schema::build(
      async_graphql::EmptyMutation,
      AuthLoginResolver,
      EmptySubscription,
    )
    .data(test_config)
    .finish();

    let query = r#"
      mutation {
        authLogin(input: { username: "testuser", password: "password123" }) {
          token
          userId
          username
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
