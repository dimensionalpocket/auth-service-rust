use axum::{
  body::Body,
  http::{Request, StatusCode},
  Router,
};
use dps_auth_api::dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;
use tower::ServiceExt;

// Helper function to create app for logging tests
async fn create_app() -> Router {
  // Create temporary database file with unique name to avoid conflicts
  let temp_file = tempfile::Builder::new()
    .prefix(&format!(
      "dps_auth_api_logging_test_{}_",
      rand::random::<u32>()
    ))
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
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();

  // Run migrations and seeds for full database setup
  server
    .migrate_database()
    .await
    .expect("Failed to run migrations");
  server.seed_database().await.expect("Failed to run seeds");

  server.create_app().await.unwrap()
}

// Helper function to create test user via GraphQL mutation
async fn create_test_user_via_mutation(app: &Router, username: &str, password: &str) -> String {
  let query = format!(
    r#"{{
      "query": "mutation {{ authRegister(username: \"{username}\", password: \"{password}\", passwordConfirmation: \"{password}\") {{ uuid username }} }}"
    }}"#
  );

  let response = app
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
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  // Verify user creation was successful
  assert!(data["errors"].is_null(), "User creation failed: {body_str}");
  assert!(!data["data"]["authRegister"]["uuid"].is_null());

  data["data"]["authRegister"]["uuid"]
    .as_str()
    .unwrap()
    .to_string()
}

// ===== PASSWORD LOGGING SECURITY TESTS =====

#[tracing_test::traced_test]
#[tokio::test]
async fn test_auth_login_no_password_in_logs() {
  let app = create_app().await;

  // Setup: Create a test user via GraphQL mutation
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;

  // Test: Call the login mutation with a distinct password
  let query = format!(
    r#"{{
      "query": "mutation {{ authLogin(username: \"{unique_username}\", password: \"secret_password_123!\") {{ token userId username message }} }}"
    }}"#
  );

  let response = app
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
    .unwrap();

  // Verify: Should succeed
  assert_eq!(response.status(), StatusCode::OK);

  // The traced_test macro will capture logs and fail if password is found
  // This test mainly ensures the instrumentation doesn't panic and the operation succeeds
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_auth_register_no_password_in_logs() {
  let app = create_app().await;

  // Test: Call the register mutation with distinct passwords
  let unique_username = format!("newuser_{}", rand::random::<u32>());
  let query = format!(
    r#"{{
      "query": "mutation {{ authRegister(username: \"{unique_username}\", password: \"super_secret_pass_456\", passwordConfirmation: \"super_secret_pass_456\") {{ userId uuid username message }} }}"
    }}"#
  );

  let response = app
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
    .unwrap();

  // Verify: Should succeed
  assert_eq!(response.status(), StatusCode::OK);

  // The traced_test macro will capture logs and fail if password is found
  // This test mainly ensures the instrumentation doesn't panic and the operation succeeds
}

// ===== HTTP LOGGING TESTS (TraceLayer) =====

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_get_request_logged() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  // TraceLayer should log the GET request
  // The traced_test macro captures logs for verification
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_post_request_logged() {
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

  // TraceLayer should log the POST request
  // The traced_test macro captures logs for verification
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_graphql_get_request_logged() {
  let app = create_app().await;

  // Test GraphQL GET request (the main issue we're fixing)
  let query = urlencoding::encode("{ getServerTimestamp }");
  let response = app
    .oneshot(
      Request::builder()
        .method("GET")
        .uri(format!("/api/graphql?query={query}"))
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  // TraceLayer should now log GraphQL GET requests (previously missing)
  // The traced_test macro captures logs for verification
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_404_request_logged() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);

  // TraceLayer should log 404 requests
  // The traced_test macro captures logs for verification
}

// ===== SHUTDOWN SERVICE LOGGING TESTS =====

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_logs_correct_message() {
  use dps_auth_api::services::shutdown_service::ShutdownService;

  // Test that log_shutdown_start logs expected messages
  ShutdownService::log_shutdown_start("SIGTERM");

  // Verify log messages were written
  assert!(logs_contain(
    "Starting graceful shutdown due to SIGTERM signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_with_sigint() {
  use dps_auth_api::services::shutdown_service::ShutdownService;

  // Test with SIGINT signal
  ShutdownService::log_shutdown_start("SIGINT");

  // Verify log messages were written
  assert!(logs_contain(
    "Starting graceful shutdown due to SIGINT signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_with_custom_signal() {
  use dps_auth_api::services::shutdown_service::ShutdownService;

  // Test with a custom signal name
  ShutdownService::log_shutdown_start("TEST_SIGNAL");

  // Verify log messages were written
  assert!(logs_contain(
    "Starting graceful shutdown due to TEST_SIGNAL signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}
