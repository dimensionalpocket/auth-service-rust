use axum::{body::Body, extract::Request, middleware, response::Response, routing::get, Router};
use dp_auth_service::middleware::session::{
  create_session_middleware, SessionContext, SESSION_COOKIE_NAME,
};
use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
use dp_auth_session_service::{DpAuthSessionPayload, DpAuthSessionService};
use std::{env, sync::Once};
use tower::ServiceExt;

static INIT: Once = Once::new();

fn setup_test_environment() {
  INIT.call_once(|| {
    env::set_var(
      "DP_AUTH_SECRET_KEY",
      "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
    );
  });
}

// Helper function to get test secret for token creation
// This should match the secret that gets loaded from the environment variable
fn get_test_secret() -> Vec<u8> {
  get_secret_from_env("DP_AUTH_SECRET_KEY", 32).expect("Failed to read test secret")
}

#[tokio::test]
async fn test_session_middleware_can_be_applied_to_router() {
  setup_test_environment();

  // Test that the session middleware compiles and can be applied to a router
  let secret = get_test_secret();
  let app = Router::new()
    .route("/graphql", get(|| async { Response::new(Body::empty()) }))
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create a simple request
  let request = Request::builder()
    .uri("/graphql")
    .body(Body::empty())
    .unwrap();

  // Send the request through the app
  let response = app.oneshot(request).await.unwrap();

  // Just verify it doesn't crash
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_context_helper_methods() {
  // Test SessionContext helper methods
  let empty_context = SessionContext::new(None);
  assert!(!empty_context.authenticated());
  assert_eq!(empty_context.user_id(), None);

  let payload = DpAuthSessionPayload {
    sub: 123,
    iat: 1000,
    exp: 2000,
  };
  let auth_context = SessionContext::new(Some(payload));
  assert!(auth_context.authenticated());
  assert_eq!(auth_context.user_id(), Some(123));
}

#[test]
fn test_session_cookie_name_constant() {
  // Test that the cookie name constant is correct
  assert_eq!(SESSION_COOKIE_NAME, "DpAuthSession");
}

#[tokio::test]
async fn test_session_middleware_with_valid_header_token() {
  setup_test_environment();

  // Create a valid session payload and token
  let secret = get_test_secret();
  let payload = DpAuthSessionService::create_payload(123);
  let token = DpAuthSessionService::encode_token(&payload, &secret).unwrap();

  // Create a test app with session middleware
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Check that session context was attached
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(context.authenticated());
        assert_eq!(context.user_id(), Some(123));
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with Authorization header
  let request = Request::builder()
    .uri("/graphql")
    .header("authorization", format!("Bearer {token}"))
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_with_valid_cookie_token() {
  setup_test_environment();

  // Create a valid session payload and token
  let secret = get_test_secret();
  let payload = DpAuthSessionService::create_payload(456);
  let token = DpAuthSessionService::encode_token(&payload, &secret).unwrap();

  // Create a test app with session middleware
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Check that session context was attached
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(context.authenticated());
        assert_eq!(context.user_id(), Some(456));
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with cookie
  let request = Request::builder()
    .uri("/graphql")
    .header("cookie", format!("{SESSION_COOKIE_NAME}={token}"))
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_prefers_header_over_cookie() {
  setup_test_environment();

  // Create two different tokens
  let secret = get_test_secret();
  let header_payload = DpAuthSessionService::create_payload(111);
  let header_token = DpAuthSessionService::encode_token(&header_payload, &secret).unwrap();

  let cookie_payload = DpAuthSessionService::create_payload(222);
  let cookie_token = DpAuthSessionService::encode_token(&cookie_payload, &secret).unwrap();

  // Create a test app with session middleware
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Should use header token (user_id 111), not cookie token (user_id 222)
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(context.authenticated());
        assert_eq!(context.user_id(), Some(111)); // Header token user_id
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with both header and cookie
  let request = Request::builder()
    .uri("/graphql")
    .header("authorization", format!("Bearer {header_token}"))
    .header("cookie", format!("{SESSION_COOKIE_NAME}={cookie_token}"))
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_invalid_header_no_cookie_fallback() {
  setup_test_environment();

  // Create a valid cookie token
  let secret = get_test_secret();
  let cookie_payload = DpAuthSessionService::create_payload(333);
  let cookie_token = DpAuthSessionService::encode_token(&cookie_payload, &secret).unwrap();

  // Create a test app with session middleware
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Should NOT fallback to cookie when header is invalid
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(!context.authenticated());
        assert_eq!(context.user_id(), None);
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with invalid header and valid cookie
  let request = Request::builder()
    .uri("/graphql")
    .header("authorization", "Bearer invalid-token")
    .header("cookie", format!("{SESSION_COOKIE_NAME}={cookie_token}"))
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_no_token() {
  setup_test_environment();

  // Create a test app with session middleware
  let secret = get_test_secret();
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Should have empty session context
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(!context.authenticated());
        assert_eq!(context.user_id(), None);
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with no authentication
  let request = Request::builder()
    .uri("/graphql")
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_expired_token() {
  setup_test_environment();

  // Create expired payload
  let current_time = chrono::Utc::now().timestamp();
  let expired_payload = DpAuthSessionPayload {
    sub: 789,
    iat: current_time - 3600, // 1 hour ago
    exp: current_time - 1800, // 30 minutes ago (expired)
  };
  let secret = get_test_secret();
  let expired_token = DpAuthSessionService::encode_token(&expired_payload, &secret).unwrap();

  // Create a test app with session middleware
  let app = Router::new()
    .route(
      "/graphql",
      get(|req: Request| async move {
        // Should have empty session context due to expired token
        let context = req.extensions().get::<SessionContext>().unwrap();
        assert!(!context.authenticated());
        assert_eq!(context.user_id(), None);
        Response::new(Body::empty())
      }),
    )
    .layer(middleware::from_fn(create_session_middleware(secret)));

  // Create request with expired token
  let request = Request::builder()
    .uri("/graphql")
    .header("authorization", format!("Bearer {expired_token}"))
    .body(Body::empty())
    .unwrap();

  // Send request through the app
  let response = app.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 200);
}
