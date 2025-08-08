use dp_auth_session_service::DpAuthSessionService;

// Re-export for backward compatibility
use async_graphql::Context;
use axum::{extract::Request, middleware::Next, response::Response};
pub use dp_auth_session_service::DpAuthSessionPayload as SessionPayload;

/// Session context that gets attached to GraphQL requests
#[derive(Debug, Clone)]
pub struct SessionContext {
  pub payload: Option<SessionPayload>,
}

impl SessionContext {
  pub fn new(payload: Option<SessionPayload>) -> Self {
    Self { payload }
  }

  pub fn authenticated(&self) -> bool {
    self.payload.is_some()
  }

  pub fn user_id(&self) -> Option<i64> {
    self.payload.as_ref().map(|p| p.sub)
  }

  /// Get session context from GraphQL context
  ///
  /// This method should always succeed since the session middleware always sets the context.
  /// If the context is missing, it indicates a configuration error and returns an internal server error.
  pub fn from_context<'a>(
    ctx: &'a Context<'a>,
  ) -> Result<&'a SessionContext, async_graphql::Error> {
    match ctx.data_opt::<SessionContext>() {
      Some(context) => Ok(context),
      None => {
        tracing::error!("Session context not available in GraphQL resolver - middleware may not be configured properly");
        Err("Internal server error".into())
      }
    }
  }
}

/// Cookie name for session tokens
pub const SESSION_COOKIE_NAME: &str = "DpAuthSession";

/// Create session middleware with the provided secret
/// Returns a middleware function that can be used with axum
pub fn create_session_middleware(
  secret: Vec<u8>
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
  move |request: Request, next: Next| {
    let secret = secret.clone();
    Box::pin(async move {
      let session_context = extract_and_validate_session_sync(&request, &secret);
      let mut request = request;
      request.extensions_mut().insert(session_context);
      next.run(request).await
    })
  }
}



/// Extract session token from request and validate it (synchronous version)
fn extract_and_validate_session_sync(request: &Request, secret: &[u8]) -> SessionContext {

  // Try header first
  if let Some(token) = extract_token_from_header(request) {
    if let Ok(payload) = DpAuthSessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
    // If header token is invalid, don't try cookie
    return SessionContext::new(None);
  }

  // Try cookie if no header
  if let Some(token) = extract_token_from_cookie(request) {
    if let Ok(payload) = DpAuthSessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
  }

  SessionContext::new(None)
}

/// Extract token from Authorization header
fn extract_token_from_header(request: &Request) -> Option<String> {
  request
    .headers()
    .get("authorization")?
    .to_str()
    .ok()?
    .strip_prefix("Bearer ")
    .map(|token| token.to_string())
}

/// Extract token from cookie
fn extract_token_from_cookie(request: &Request) -> Option<String> {
  let cookie_header = request.headers().get("cookie")?.to_str().ok()?;

  for cookie in cookie_header.split(';') {
    let cookie = cookie.trim();
    if let Some(value) = cookie.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
      return Some(value.to_string());
    }
  }

  None
}
