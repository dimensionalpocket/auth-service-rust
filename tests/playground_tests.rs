use axum::{
  body::Body,
  http::{Request, StatusCode},
};
use dps_auth_api::DpsAuthApi;
use tempfile::NamedTempFile;
use tower::ServiceExt;

#[tokio::test]
async fn test_playground_development_mode_enabled() {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true); // Enable development mode
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test playground endpoint returns 200 with HTML content
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/api/playground")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  // Check that response contains HTML content
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert!(body_str.contains("GraphQL Playground"));
  assert!(body_str.contains("<!DOCTYPE html>"));

  // Test GET /graphql returns method not allowed
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/api/graphql")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_playground_development_mode_disabled() {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(false); // Disable development mode
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test playground endpoint returns 404 when development mode is disabled
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/api/playground")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);

  // Test GET /graphql still returns method not allowed
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/api/graphql")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_playground_default_mode() {
  // Test with default configuration (no explicit development mode setting)
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  // Don't set development_mode explicitly - use default
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test playground endpoint returns 404 with default configuration
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/api/playground")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
