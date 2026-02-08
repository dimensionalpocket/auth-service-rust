use crate::graphql::types::{UserRole, UserWithRoleResponse};
use crate::orchestrators::auth::AuthRegisterOrchestrator;
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for user registration response
#[derive(async_graphql::SimpleObject)]
pub struct AuthRegisterResponse {
  /// The registered user's information including role
  pub user: UserWithRoleResponse,
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

    match AuthRegisterOrchestrator::run(pool, &username, &password, &password_confirmation, config)
      .await
    {
      Ok((register_result, cookie_value)) => {
        // Use append to allow multiple cookies; ignore the return value
        let _ = ctx.append_http_header("set-cookie", cookie_value);

        Ok(AuthRegisterResponse {
          user: UserWithRoleResponse {
            id: register_result.user_id,
            name: register_result.username,
            role: UserRole::from(register_result.role),
            uuid: Some(register_result.uuid),
            created_ts: Some(register_result.created_ts),
            updated_ts: Some(register_result.updated_ts),
          },
          message: "User registration successful".to_string(),
        })
      }
      Err(SessionError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
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
  use crate::test_utils::{create_test_mutation_schema, create_test_role_model};

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_auth_register_calls_service_with_correct_parameters() {
    // Insert default role first
    {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
    }

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
    let schema = create_test_mutation_schema(mutation, Some(pool), None, Some(test_config));

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          user {
            id
            uuid
            name
            role {
              id
              name
              permissions
            }
            createdTs
            updatedTs
          }
          message
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let auth_register = &data["authRegister"];
    let user_data = &auth_register["user"];

    assert!(user_data["id"].as_i64().unwrap() > 0);
    assert!(!user_data["uuid"].as_str().unwrap().is_empty());
    assert_eq!(user_data["name"].as_str().unwrap(), "testuser");
    assert_eq!(user_data["role"]["name"].as_str().unwrap(), "user");
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
    assert_eq!(
      auth_register["message"].as_str().unwrap(),
      "User registration successful"
    );
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_auth_register_returns_error_for_duplicate_username() {
    // Insert default role first
    {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
    }

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
    let schema = create_test_mutation_schema(mutation, Some(pool), None, Some(test_config));

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          user {
            id
            name
          }
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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_auth_register_validates_input() {
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
    let schema = create_test_mutation_schema(mutation, Some(pool), None, Some(test_config));

    // Test empty username
    let query = r#"
      mutation {
        authRegister(username: "", password: "testpass123", passwordConfirmation: "testpass123") {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_auth_register_password_confirmation_mismatch() {
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
    let schema = create_test_mutation_schema(mutation, Some(pool), None, Some(test_config));

    // Test password confirmation mismatch
    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "differentpass") {
          user {
            id
            name
          }
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Passwords do not match"));
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_auth_register_sets_cookie() {
    // Insert default role first
    {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
    }

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
    let schema = create_test_mutation_schema(mutation, Some(pool), None, Some(test_config));

    let query = r#"
      mutation {
        authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
          user {
            id
            name
          }
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
