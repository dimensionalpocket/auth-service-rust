use crate::middleware::session::SESSION_COOKIE_NAME;
use crate::services::{AuthService, UserError};
use crate::DpsAuthApiConfig;
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
  #[instrument(skip(self, ctx, password, password_confirmation), fields(username = %username))]
  #[graphql(name = "authRegister")]
  async fn auth_register(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "username")] username: String,
    #[graphql(name = "password")] password: String,
    #[graphql(name = "passwordConfirmation")] password_confirmation: String,
  ) -> Result<AuthRegisterResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let config = ctx.data::<DpsAuthApiConfig>()?;
    let session_secret = config.session_secret.clone();
    let cookie_domain = config.cookie_domain.clone();
    let insecure_cookie = config.insecure_cookie;

    match AuthService::register(
      pool,
      &username,
      &password,
      &password_confirmation,
      &session_secret,
    )
    .await
    {
      Ok(register_result) => {
        // Set session cookie
        let cookie_value = format!(
          "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
          SESSION_COOKIE_NAME,
          register_result.session_token,
          cookie_domain,
          config.api_path,
          if insecure_cookie { "" } else { "; Secure" },
          config.session_ttl_seconds
        );

        // Use append to allow multiple cookies; ignore the return value
        let _ = ctx.append_http_header("set-cookie", cookie_value);

        Ok(AuthRegisterResponse {
          user_id: register_result.user_id,
          uuid: register_result.uuid,
          username: register_result.username,
          role_id: register_result.role_id,
          created_ts: register_result.created_ts,
          updated_ts: register_result.updated_ts,
          message: "User registration successful".to_string(),
        })
      }
      Err(UserError::UsernameAlreadyExists(username)) => Err(async_graphql::Error::new(format!(
        "Username '{username}' is already in use"
      ))),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(UserError::SessionError(msg)) => Err(async_graphql::Error::new(format!(
        "Session creation failed: {msg}"
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

    // Test secret - 32 bytes for AES-256
    let test_secret = vec![
      0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4,
      0x8d, 0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74,
      0x4d, 0xb8,
    ];

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: test_secret,
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(test_config)
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

    // Test secret - 32 bytes for AES-256
    let test_secret = vec![
      0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4,
      0x8d, 0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74,
      0x4d, 0xb8,
    ];

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: test_secret,
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(test_config)
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

    // Test secret - 32 bytes for AES-256
    let test_secret = vec![
      0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4,
      0x8d, 0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74,
      0x4d, 0xb8,
    ];

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: test_secret,
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(test_config)
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

    // Test secret - 32 bytes for AES-256
    let test_secret = vec![
      0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4,
      0x8d, 0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74,
      0x4d, 0xb8,
    ];

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: test_secret,
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(test_config)
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

  #[tokio::test]
  async fn test_auth_register_sets_cookie() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Test secret - 32 bytes for AES-256
    let test_secret = vec![
      0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4,
      0x8d, 0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74,
      0x4d, 0xb8,
    ];

    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: test_secret,
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthRegisterResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(test_config)
      .finish();

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          userId
          username
          message
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    // Check that set-cookie header is present
    let headers = result.http_headers;
    let set_cookie_headers: Vec<_> = headers
      .get_all("set-cookie")
      .iter()
      .map(|h| h.to_str().unwrap())
      .collect();

    assert!(!set_cookie_headers.is_empty(), "No set-cookie header found");

    let cookie_header = &set_cookie_headers[0];
    assert!(
      cookie_header.contains("DpsAuthSession="),
      "Cookie header should contain session token"
    );
    assert!(
      cookie_header.contains("Domain=.dps.localhost"),
      "Cookie header should contain domain"
    );
    assert!(
      cookie_header.contains("Path=/api"),
      "Cookie header should contain path"
    );
    assert!(
      cookie_header.contains("HttpOnly"),
      "Cookie header should contain HttpOnly"
    );
    assert!(
      cookie_header.contains("SameSite=Lax"),
      "Cookie header should contain SameSite"
    );
    assert!(
      cookie_header.contains("Secure"),
      "Cookie header should contain Secure flag"
    );
    assert!(
      cookie_header.contains("Max-Age=3600"),
      "Cookie header should contain max-age"
    );
  }
}
