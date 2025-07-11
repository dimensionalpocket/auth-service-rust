use axum::{
  body::Body,
  http::{Request, StatusCode},
  Router,
};
use dp_auth_service::{
  graphql::schema::create_schema,
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, root_handler},
  },
};
use tower::ServiceExt;
use tower_http::cors::CorsLayer;

fn create_app() -> Router {
  let schema = create_schema();

  Router::new()
    .route("/", axum::routing::get(root_handler))
    .route("/health", axum::routing::get(health_handler))
    .route(
      "/graphql",
      axum::routing::get(graphql_get_handler).post(graphql_post_handler),
    )
    .layer(CorsLayer::permissive())
    .with_state(schema)
}

#[tokio::test]
async fn test_root_endpoint() {
  let app = create_app();

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
  let app = create_app();

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
  let app = create_app();

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

// Note: createUser mutation integration tests would require database setup
// which is not currently configured in this integration test file.
// The mutation tests are covered in the unit tests with proper database setup.
