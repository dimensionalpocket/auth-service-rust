use dp_auth_service::{
  middleware::session::{session_middleware, SessionContext, SESSION_COOKIE_NAME},
  services::SessionPayload,
};
use axum::{
  body::Body, 
  extract::Request, 
  response::Response,
  routing::get,
  Router,
  middleware,
};
use tower::ServiceExt;

#[tokio::test]
async fn test_session_middleware_can_be_applied_to_router() {
  // Test that the session middleware compiles and can be applied to a router
  let app = Router::new()
    .route("/graphql", get(|| async { Response::new(Body::empty()) }))
    .layer(middleware::from_fn(session_middleware));
  
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
  
  let payload = SessionPayload {
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

