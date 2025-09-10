use axum::{
  body::Body,
  http::{Request, StatusCode},
  Router,
};
use dp_auth_service::dp_auth_server::DpAuthServer;
use tower::ServiceExt;

// No test wrapper functions needed - using DpAuthServer's configured router

async fn create_app() -> Router {
  // Generate a random 32-byte session secret for this test to ensure test isolation
  let session_secret: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

  // Create temporary database file with unique name to avoid conflicts
  let temp_file = tempfile::Builder::new()
    .prefix(&format!("dp_auth_test_{}_", rand::random::<u32>()))
    .suffix(".db")
    .tempfile()
    .expect("Failed to create temp file");
  let db_path = temp_file
    .path()
    .to_str()
    .expect("Failed to get temp file path");

  let server = DpAuthServer::new()
    .session_secret(session_secret)
    .sqlite_file_path(db_path)
    .cookie_domain(".api.dp-auth.localhost")
    .insecure_cookie(true)
    .development_mode(true)
    .database_pool_size(1) // Use small pool size for tests to avoid concurrency issues
    .build()
    .unwrap();

  // Run migrations and seeds for full database setup
  server
    .migrate_database()
    .await
    .expect("Failed to run migrations");
  server.seed_database().await.expect("Failed to run seeds");

  server.create_app().await.unwrap()
}

// Helper function to create test users via GraphQL mutation (tests actual CreateUser mutation)
// Uses unique usernames to avoid any potential conflicts
async fn create_test_user_via_mutation(app: &Router, username: &str, password: &str) -> String {
  let query = format!(
    r#"{{
      "query": "mutation {{ createUser(input: {{ username: \"{username}\", password: \"{password}\" }}) {{ uuid username }} }}"
    }}"#
  );

  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  // Verify user creation was successful
  assert!(data["errors"].is_null(), "User creation failed: {body_str}");
  assert!(!data["data"]["createUser"]["uuid"].is_null());

  data["data"]["createUser"]["uuid"]
    .as_str()
    .unwrap()
    .to_string()
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn test_graphql_endpoint() {
  let app = create_app().await;

  let query = r#"{"query": "{ getServerTimestamp }"}"#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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

#[tokio::test]
async fn test_create_session_mutation_success() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  let query = format!(
    r#"{{
      "query": "mutation {{ createSession(input: {{ username: \"{unique_username}\", password: \"password123\" }}) {{ token message }} }}"
    }}"#
  );

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  // Check for Set-Cookie header
  let headers = response.headers();
  let cookie_header = headers.get("set-cookie");
  assert!(
    cookie_header.is_some(),
    "Set-Cookie header should be present"
  );

  let cookie_value = cookie_header.unwrap().to_str().unwrap();
  assert!(cookie_value.contains("DpAuthSession="));
  assert!(cookie_value.contains("Domain=.api.dp-auth.localhost"));
  assert!(cookie_value.contains("HttpOnly"));
  assert!(cookie_value.contains("SameSite=Strict"));
  // Should not contain Secure flag due to DP_AUTH_INSECURE_COOKIE=true
  assert!(!cookie_value.contains("Secure"));

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();

  // Should contain successful response
  assert!(body_str.contains("createSession"));
  assert!(body_str.contains("token"));
  assert!(body_str.contains("Authentication successful"));
  assert!(!body_str.contains("errors"));
}

#[tokio::test]
async fn test_create_session_mutation_invalid_credentials() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  let query = format!(
    r#"{{
      "query": "mutation {{ createSession(input: {{ username: \"{unique_username}\", password: \"wrongpassword\" }}) {{ token message }} }}"
    }}"#
  );

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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

#[tokio::test]
async fn test_create_session_mutation_with_missing_user() {
  let app = create_app().await;

  let query = r#"
    {
      "query": "mutation { createSession(input: { username: \"nonexistent\", password: \"password123\" }) { token message } }"
    }
  "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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

#[tokio::test]
async fn test_get_current_session_integration_authenticated() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation (tests actual CreateUser mutation)
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  // Create session first
  let create_session_query = format!(
    r#"{{
      "query": "mutation {{ createSession(input: {{ username: \"{unique_username}\", password: \"password123\" }}) {{ token message }} }}"
    }}"#
  );

  let create_session_response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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
  let token = session_data["data"]["createSession"]["token"]
    .as_str()
    .unwrap();

  // Test getCurrentSession with token in header
  let get_session_query = r#"
    {
      "query": "{ getCurrentSession { sub iat exp } }"
    }
  "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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
  assert!(!data["data"]["getCurrentSession"].is_null());
  assert!(data["data"]["getCurrentSession"]["sub"].as_i64().unwrap() > 0);
  assert!(data["data"]["getCurrentSession"]["iat"].as_i64().unwrap() > 0);
  assert!(data["data"]["getCurrentSession"]["exp"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_get_current_session_integration_unauthenticated() {
  let app = create_app().await;

  let query = r#"
    {
      "query": "{ getCurrentSession { sub iat exp } }"
    }
  "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
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
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  assert!(data["errors"].is_null());
  assert!(data["data"]["getCurrentSession"].is_null());
}

#[tokio::test]
async fn test_get_current_session_integration_invalid_token() {
  let app = create_app().await;

  let query = r#"
    {
      "query": "{ getCurrentSession { sub iat exp } }"
    }
  "#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .header("authorization", "Bearer invalid_token")
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
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  assert!(data["errors"].is_null());
  assert!(data["data"]["getCurrentSession"].is_null());
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
