use crate::services::GenerateLogoutCookieService;
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Object, Result, SimpleObject};
use tracing::instrument;

/// GraphQL output type for logout response
#[derive(SimpleObject)]
pub struct AuthLogoutResponse {
  /// Success message
  pub message: String,
}

/// GraphQL mutation for user logout
#[derive(Default)]
pub struct AuthLogoutResolver;

#[Object]
impl AuthLogoutResolver {
  /// Logout user by clearing the session cookie
  #[instrument(skip(self, ctx))]
  #[graphql(name = "authLogout")]
  async fn auth_logout(&self, ctx: &Context<'_>) -> Result<AuthLogoutResponse> {
    let config = ctx.data::<DpsAuthApiConfig>()?;

    // Set cookie to expire in the past to effectively delete it
    let cookie_value = GenerateLogoutCookieService::run(config);

    // Use append to set the expired cookie
    let _ = ctx.append_http_header("set-cookie", cookie_value);

    Ok(AuthLogoutResponse {
      message: "Successfully logged out".to_string(),
    })
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::create_test_mutation_schema;

  #[dps_auth_db_test]
  async fn test_auth_logout_success() {
    // Test config
    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: vec![0x42; 32],
      cookie_domain: ".dps.localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthLogoutResolver;
    let schema = create_test_mutation_schema(mutation, None, None, Some(test_config));

    let query = r#"
      mutation {
        authLogout {
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should succeed
    assert!(
      result.errors.is_empty(),
      "GraphQL errors: {:?}",
      result.errors
    );

    let data = result.data.into_json().unwrap();
    let auth_logout = &data["authLogout"];

    assert_eq!(
      auth_logout["message"].as_str().unwrap(),
      "Successfully logged out"
    );
  }

  #[dps_auth_db_test]
  async fn test_auth_logout_sets_cookie_header() {
    // Test config with secure cookie
    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: vec![0x42; 32],
      cookie_domain: ".example.com".to_string(),
      api_path: "/graphql".to_string(),
      insecure_cookie: false,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthLogoutResolver;
    let schema = create_test_mutation_schema(mutation, None, None, Some(test_config));

    let query = r#"
      mutation {
        authLogout {
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should succeed
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

    // Verify cookie structure
    assert!(
      cookie_header.starts_with("DpsAuthSession="),
      "Cookie header should start with session name"
    );
    assert!(
      cookie_header.contains("Domain=.example.com"),
      "Cookie header should contain domain"
    );
    assert!(
      cookie_header.contains("Path=/graphql"),
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
      cookie_header.contains("; Secure"),
      "Cookie header should contain Secure flag"
    );
    assert!(
      cookie_header.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"),
      "Cookie header should contain expiration date"
    );
  }

  #[dps_auth_db_test]
  async fn test_auth_logout_insecure_cookie() {
    // Test config with insecure cookie
    let test_config = DpsAuthApiConfig {
      port: 0,
      sqlite_main_file_path: "test.db".to_string(),
      session_secret: vec![0x42; 32],
      cookie_domain: ".localhost".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let mutation = AuthLogoutResolver;
    let schema = create_test_mutation_schema(mutation, None, None, Some(test_config));

    let query = r#"
      mutation {
        authLogout {
          message
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should succeed
    assert!(result.errors.is_empty());

    // Check that set-cookie header is present
    let headers = result.http_headers;
    let set_cookie_headers: Vec<_> = headers
      .get_all("set-cookie")
      .iter()
      .map(|h| h.to_str().unwrap())
      .collect();

    assert!(!set_cookie_headers.is_empty());

    let cookie_header = &set_cookie_headers[0];

    // Verify insecure cookie (no Secure flag)
    assert!(
      cookie_header.starts_with("DpsAuthSession="),
      "Cookie header should start with session name"
    );
    assert!(
      cookie_header.contains("Domain=.localhost"),
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
      !cookie_header.contains("; Secure"),
      "Cookie header should NOT contain Secure flag for insecure cookies"
    );
    assert!(
      cookie_header.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"),
      "Cookie header should contain expiration date"
    );
  }
}
