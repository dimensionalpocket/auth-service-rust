use axum::{
  body::Body,
  http::{Request, StatusCode},
  Router,
};
use dp_auth_service::{
  database::test_utils::create_test_database,
  graphql::schema::create_schema,
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, not_found_handler, root_handler},
  },
  middleware::session::{create_session_middleware, init_session_secret},
  utils::get_secret_from_env::get_secret_from_env,
  services::UserService,
};
use sqlx::SqlitePool;
use std::{env, sync::Once};
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
    .fallback(not_found_handler)
    .layer(CorsLayer::permissive())
    .with_state(schema)
}

async fn create_app_with_database() -> (Router, SqlitePool, tempfile::NamedTempFile) {
  let (pool, temp_file) = create_test_database().await;

  // Create schema with database pool in the data context
  use async_graphql::{EmptySubscription, Schema};
  use dp_auth_service::graphql::{mutation::Mutation, query::Query};

  let schema = Schema::build(Query::new(), Mutation::new(), EmptySubscription)
    .data(pool.clone())
    .finish();

  // Get session secret for middleware
  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("Failed to read session secret for test");

  let app = Router::new()
    .route("/", axum::routing::get(root_handler))
    .route("/health", axum::routing::get(health_handler))
    .route(
      "/graphql",
      axum::routing::get(graphql_get_handler).post(graphql_post_handler),
    )
    .layer(axum::middleware::from_fn(create_session_middleware(session_secret)))
    .layer(CorsLayer::permissive())
    .with_state(schema);

  (app, pool, temp_file)
}

static INIT: Once = Once::new();

fn setup_test_environment() {
  INIT.call_once(|| {
    // Set up test environment variables
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
    );
    env::set_var("DP_AUTH_INSECURE_COOKIE", "true");
    env::set_var("DP_AUTH_COOKIE_DOMAIN", ".api.dp-auth.localhost");

    // Initialize session secret cache (for backward compatibility during Phase 2)
    // Initialize session secret - fail test if this fails
    init_session_secret().expect("Failed to initialize session secret for test");
  });
}

async fn setup_default_role(pool: &SqlitePool) {
  sqlx::query(
    "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
  )
  .execute(pool)
  .await
  .unwrap();
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

#[tokio::test]
async fn test_create_session_mutation_success() {
  setup_test_environment();
  let (app, pool, _temp_file) = create_app_with_database().await;

  // Setup: Create a user
  setup_default_role(&pool).await;
  UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  let query = r#"
    {
      "query": "mutation { createSession(input: { username: \"testuser\", password: \"password123\" }) { token message } }"
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
  setup_test_environment();
  let (app, pool, _temp_file) = create_app_with_database().await;

  // Setup: Create a user
  setup_default_role(&pool).await;
  UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  let query = r#"
    {
      "query": "mutation { createSession(input: { username: \"testuser\", password: \"wrongpassword\" }) { token message } }"
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
  setup_test_environment();
  let (app, _pool, _temp_file) = create_app_with_database().await;

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
  setup_test_environment();
  let (app, pool, _temp_file) = create_app_with_database().await;

  // Setup: Create a user
  setup_default_role(&pool).await;
  UserService::create_user(&pool, "testuser", "password123")
    .await
    .unwrap();

  // Create session first
  let create_session_query = r#"
    {
      "query": "mutation { createSession(input: { username: \"testuser\", password: \"password123\" }) { token message } }"
    }
  "#;

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
  setup_test_environment();
  let (app, _pool, _temp_file) = create_app_with_database().await;

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
  setup_test_environment();
  let (app, _pool, _temp_file) = create_app_with_database().await;

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
  let app = create_app();

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
  let app = create_app();

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
  let app = create_app();

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
