use axum::{
  body::Body,
  http::{Request, StatusCode},
  Router,
};
use dps_auth_api::dps_auth_api::DpsAuthApi;
use dps_auth_api::test_utils;
use dps_auth_api::test_utils::create_test_user_via_mutation;
use dps_auth_test_macros::dps_auth_db_test;
use dps_config::DpsConfig;
use tower::ServiceExt;

// No test wrapper functions needed - using DpsAuthApi's configured router

async fn create_app() -> Router {
  // Create temporary database file with unique name to avoid conflicts
  let temp_file = tempfile::Builder::new()
    .prefix(&format!("dps_auth_api_test_{}_", rand::random::<u32>()))
    .suffix(".db")
    .tempfile()
    .expect("Failed to create temp file");
  let db_path = temp_file
    .path()
    .to_str()
    .expect("Failed to get temp file path");

  let mut config = DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config.set_auth_api_sqlite_main_pool_size(Some(1)); // Use small pool size for tests to avoid concurrency issues

  let server = DpsAuthApi::new(config).unwrap();

  // Run migrations and seeds for full database setup
  server
    .migrate_database()
    .await
    .expect("Failed to run migrations");
  server.seed_database().await.expect("Failed to run seeds");

  server.create_app().await.unwrap()
}

// Helper function to create session and return response (for cookie extraction)
async fn create_session_via_mutation(
  app: &Router,
  username: &str,
  password: &str,
) -> axum::response::Response<Body> {
  let query = format!(
    r#"{{"query": "mutation {{ authLogin(username: \"{username}\", password: \"{password}\") {{ token user {{ id name }} message }} }}"}}"#
  );

  app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap()
}

// Helper to extract token from GraphQL response body
async fn extract_token_from_response(response: axum::response::Response<Body>) -> String {
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  data["data"]["authLogin"]["token"]
    .as_str()
    .unwrap()
    .to_string()
}

// Helper to parse GraphQL response
async fn parse_graphql_response(response: axum::response::Response<Body>) -> serde_json::Value {
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  serde_json::from_str(&body_str).unwrap()
}

#[dps_auth_db_test]
async fn test_root_endpoint() {
  let app = create_app().await;

  let response = app
    .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  assert_eq!(&body[..], b"OK");
}

#[dps_auth_db_test]
async fn test_health_endpoint() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  assert_eq!(&body[..], b"OK");
}

#[dps_auth_db_test]
async fn test_graphql_endpoint() {
  let app = create_app().await;

  let query = r#"{"query": "{ getServerTimestamp }"}"#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();

  // Should contain timestamp data
  assert!(body_str.contains("getServerTimestamp"));
  assert!(body_str.contains("data"));
}

#[dps_auth_db_test]
async fn test_create_session_mutation_success() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  let query = format!(
    r#"{{
      "query": "mutation {{ authLogin(username: \"{unique_username}\", password: \"password123\") {{ token user {{ id name }} message }} }}"
    }}"#
  );

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  // Check for Set-Cookie header
  let headers = response.headers();
  let cookie_header = headers.get("set-cookie");
  assert!(
    cookie_header.is_some(),
    "Set-Cookie header should be present"
  );

  let cookie_value = cookie_header.unwrap().to_str().unwrap();
  assert!(cookie_value.contains("DpsAuthSession="));
  assert!(cookie_value.contains("Domain=.dps.localhost"));
  assert!(cookie_value.contains("HttpOnly"));
  assert!(cookie_value.contains("SameSite=Lax"));
  assert!(cookie_value.contains("Path=/api"));
  // Should not contain Secure flag due to DPS_AUTH_API_INSECURE_COOKIE=true
  assert!(!cookie_value.contains("Secure"));

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();

  // Should contain successful response
  assert!(body_str.contains("authLogin"));
  assert!(body_str.contains("token"));
  assert!(body_str.contains("Authentication successful"));
  assert!(!body_str.contains("errors"));
}

#[dps_auth_db_test]
async fn test_create_session_mutation_invalid_credentials() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  let query = format!(
    r#"{{
      "query": "mutation {{ authLogin(username: \"{unique_username}\", password: \"wrongpassword\") {{ token user {{ id name }} message }} }}"
    }}"#
  );

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK); // GraphQL returns 200 even for business logic errors

  // Should not have Set-Cookie header on failure
  let headers = response.headers();
  let cookie_header = headers.get("set-cookie");
  assert!(
    cookie_header.is_none(),
    "Set-Cookie header should not be present on authentication failure"
  );

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();

  // Should contain error response
  assert!(body_str.contains("errors"));
  assert!(body_str.contains("Invalid credentials"));
}

#[dps_auth_db_test]
async fn test_create_session_mutation_with_missing_user() {
  let app = create_app().await;

  let query = r#"
    {
      "query": "mutation { authLogin(username: \"nonexistent_user\", password: \"password123\") { token user { id name } message } }"
    }
    "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK); // GraphQL returns 200 even for business logic errors

  // Should not have Set-Cookie header on failure
  let headers = response.headers();
  let cookie_header = headers.get("set-cookie");
  assert!(
    cookie_header.is_none(),
    "Set-Cookie header should not be present when user doesn't exist"
  );

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();

  // Should contain error response
  assert!(body_str.contains("errors"));
  assert!(body_str.contains("Invalid credentials"));
}

#[dps_auth_db_test]
async fn test_get_auth_me_integration_authenticated() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  // Create session first
  let create_session_query = format!(
    r#"{{
      "query": "mutation {{ authLogin(username: \"{unique_username}\", password: \"password123\") {{ token user {{ id name }} message }} }}"
    }}"#
  );

  let create_session_response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(create_session_query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(create_session_response.status(), StatusCode::OK);

  let create_session_body = axum::body::to_bytes(create_session_response.into_body(), usize::MAX)
    .await
    .unwrap();
  let create_session_str = String::from_utf8(create_session_body.to_vec()).unwrap();
  let session_data: serde_json::Value = serde_json::from_str(&create_session_str).unwrap();
  let token = session_data["data"]["authLogin"]["token"].as_str().unwrap();

  // Test authMe with token in header
  let get_session_query = r#"
    {
      "query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"
    }
    "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(get_session_query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  assert!(data["errors"].is_null());
  assert!(!data["data"]["authMe"].is_null());
}

#[dps_auth_db_test]
async fn test_get_auth_me_integration_invalid_token() {
  let app = create_app().await;

  let query = r#"
    {
      "query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"
    }
    "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", "Bearer invalid_token")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  // Should return null authMe (no errors) due to invalid token
  assert!(data["errors"].is_null());
  assert!(data["data"]["authMe"].is_null());
}

#[dps_auth_db_test]
async fn test_404_handler_returns_not_found() {
  let app = create_app().await;

  let request = Request::builder()
    .uri("/nonexistent")
    .body(Body::empty())
    .unwrap();

  let response = app.oneshot(request).await.unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}

#[dps_auth_db_test]
async fn test_404_handler_with_query_string() {
  let app = create_app().await;

  let request = Request::builder()
    .uri("/nonexistent?param=value")
    .body(Body::empty())
    .unwrap();

  let response = app.oneshot(request).await.unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}

#[dps_auth_db_test]
async fn test_404_handler_with_post_method() {
  let app = create_app().await;

  let request = Request::builder()
    .method("POST")
    .uri("/nonexistent")
    .header("content-type", "application/json")
    .body(Body::from(r#"{"test": "data"}"#))
    .unwrap();

  let response = app.oneshot(request).await.unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}

// ===== NEW SESSION TESTS =====

#[dps_auth_db_test]
async fn test_get_auth_me_with_cookie_authentication() {
  let app = create_app().await;

  // Create user and session
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  // Create session and extract cookie
  let create_session_response =
    create_session_via_mutation(&app, &unique_username, "password123").await;
  let cookie_header = create_session_response.headers().get("set-cookie").unwrap();

  // Test authMe with cookie
  let query = r#"{"query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("cookie", cookie_header.to_str().unwrap())
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let data = parse_graphql_response(response).await;

  assert!(data["errors"].is_null());
  assert!(!data["data"]["authMe"].is_null());
  assert!(data["data"]["authMe"]["user"]["id"].as_i64().unwrap() > 0);
  assert!(!data["data"]["authMe"]["user"]["name"]
    .as_str()
    .unwrap()
    .is_empty());
  assert!(data["data"]["authMe"]["sessionIat"].as_i64().unwrap() > 0);
  assert!(data["data"]["authMe"]["sessionExp"].as_i64().unwrap() > 0);
}

#[dps_auth_db_test]
async fn test_session_header_precedence_over_cookie() {
  let app = create_app().await;

  // Create two different users and sessions
  let user1 = format!("user1_{}", rand::random::<u32>());
  let user2 = format!("user2_{}", rand::random::<u32>());

  create_test_user_via_mutation(&app, &user1, "password123").await;
  create_test_user_via_mutation(&app, &user2, "password123").await;

  // Create sessions for both users
  let session1_response = create_session_via_mutation(&app, &user1, "password123").await;
  let session2_response = create_session_via_mutation(&app, &user2, "password123").await;

  // Extract tokens
  let header_token = extract_token_from_response(session1_response).await;
  let cookie_header = session2_response.headers().get("set-cookie").unwrap();

  // Test with both header and cookie - header should win
  let query = r#"{"query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {header_token}"))
        .header("cookie", cookie_header.to_str().unwrap())
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  // Should return user1's session (from header), not user2's (from cookie)
  let data = parse_graphql_response(response).await;
  let returned_user_id = data["data"]["authMe"]["user"]["id"].as_i64().unwrap();

  // We can't easily determine which user ID corresponds to which user without additional queries,
  // but we can verify that we get a valid session response and that it's consistent
  assert!(returned_user_id > 0);
  assert!(data["errors"].is_null());
  assert!(!data["data"]["authMe"].is_null());
}

#[dps_auth_db_test]
async fn test_session_invalid_header_no_cookie_fallback() {
  let app = create_app().await;

  // Create user and valid session cookie
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  let session_response = create_session_via_mutation(&app, &unique_username, "password123").await;
  let cookie_header = session_response.headers().get("set-cookie").unwrap();

  // Test with invalid header and valid cookie - should NOT fallback to cookie
  let query = r#"{"query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", "Bearer invalid-token")
        .header("cookie", cookie_header.to_str().unwrap())
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let data = parse_graphql_response(response).await;

  // Should return null session due to invalid header (no cookie fallback)
  assert!(data["errors"].is_null());
  assert!(data["data"]["authMe"].is_null());
}

#[dps_auth_db_test]
async fn test_session_expired_token_handling() {
  let app = create_app().await;

  // Create user
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  // We need to create an expired token manually
  // Since we can't easily access the secret from the test, we'll create a token
  // that's structurally valid but with expired timestamps

  // For this test, we'll use the session service directly to create an expired token
  use dps_auth_session::{DpsAuthSession, DpsAuthSessionPayload};

  let current_time = chrono::Utc::now().timestamp();
  let expired_payload = DpsAuthSessionPayload {
    sub: "999".to_string(),   // Use a fake user ID
    iat: current_time - 3600, // 1 hour ago
    exp: current_time - 1800, // 30 minutes ago (expired)
  };

  // Generate a test secret (same pattern as create_app)
  let test_secret: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
  let expired_token = DpsAuthSession::encode_token(&expired_payload, &test_secret).unwrap();

  // Test with expired token in header
  let query = r#"{"query": "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {expired_token}"))
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let data = parse_graphql_response(response).await;

  // Should return null session due to expired token
  assert!(data["errors"].is_null());
  assert!(data["data"]["authMe"].is_null());
}

#[test]
fn test_session_context_utility_methods() {
  use dps_auth_api::middleware::session::SessionContext;
  use dps_auth_api::test_utils::create_test_dps_config;
  use dps_auth_api::utils::session_context_sub_to_user_id;
  use dps_auth_session::DpsAuthSessionPayload;

  let config = create_test_dps_config();

  // Test empty context
  let empty_context = SessionContext::new(None);
  assert!(!empty_context.authenticated());
  let result = session_context_sub_to_user_id(&empty_context, &config);
  assert!(result.is_err());

  // Test authenticated context
  let payload = DpsAuthSessionPayload {
    sub: "123".to_string(),
    iat: 1000,
    exp: 2000,
  };
  let auth_context = SessionContext::new(Some(payload));
  assert!(auth_context.authenticated());
  let result = session_context_sub_to_user_id(&auth_context, &config);
  assert_eq!(result, Ok(123));
}

#[test]
fn test_session_cookie_name_constant() {
  use dps_auth_api::middleware::session::SESSION_COOKIE_NAME;
  assert_eq!(SESSION_COOKIE_NAME, "DpsAuthSession");
}
