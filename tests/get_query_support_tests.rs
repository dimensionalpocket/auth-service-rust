use axum::{
  body::Body,
  http::{Request, StatusCode},
};
use dps_auth_api::DpsAuthApi;
use tempfile::NamedTempFile;
use tower::ServiceExt;

#[dps_auth_test_macros::dps_auth_db_test(crate_path = dps_auth_api)]
async fn test_graphql_get_query_support() {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test GET request with query parameter
  let query = r#"{ getServerTimestamp }"#;
  let encoded_query = urlencoding::encode(query);

  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri(format!("/api/graphql?query={encoded_query}"))
        .body(Body::empty())
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

#[dps_auth_test_macros::dps_auth_db_test(crate_path = dps_auth_api)]
async fn test_graphql_post_still_works() {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test POST request still works
  let query = r#"{"query": "{ getServerTimestamp }"}"#;

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

  // Should contain timestamp data
  assert!(body_str.contains("getServerTimestamp"));
  assert!(body_str.contains("data"));
}

#[dps_auth_test_macros::dps_auth_db_test(crate_path = dps_auth_api)]
async fn test_graphql_get_with_variables() {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();

  let mut config = dps_config::DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_auth_api_sqlite_main_file_path(db_path);
  config.set_domain("dps.localhost");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config.set_auth_api_sqlite_main_pool_size(Some(1));

  let server = DpsAuthApi::new(config).unwrap();
  let app = server.create_app().await.unwrap();

  // Test GET request with query and variables (using a valid query that could use variables)
  let query = r#"query TestQuery { getServerTimestamp }"#;
  let variables = r#"{}"#;
  let encoded_query = urlencoding::encode(query);
  let encoded_variables = urlencoding::encode(variables);

  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("GET")
        .uri(format!(
          "/api/graphql?query={encoded_query}&variables={encoded_variables}"
        ))
        .body(Body::empty())
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
