use axum::{extract::Request, middleware::Next, response::Response};
use async_graphql::Context;
use crate::services::{SessionPayload, SessionService};

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
  pub fn from_graphql_context<'a>(ctx: &'a Context<'a>) -> Option<&'a SessionContext> {
    ctx.data_opt::<SessionContext>()
  }
  
  /// Get session context from GraphQL context with error logging
  /// 
  /// This is the recommended method for resolvers as it logs missing context
  /// as a server error and returns a user-friendly error message.
  pub fn from_graphql_context_or_error<'a>(ctx: &'a Context<'a>) -> Result<&'a SessionContext, async_graphql::Error> {
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


/// Session middleware for all requests (only processes GraphQL requests)
pub async fn session_middleware(mut request: Request, next: Next) -> Response {
  // Only process session tokens for GraphQL requests
  if request.uri().path() == "/graphql" {
    let session_context = extract_and_validate_session_sync(&request);
    
    // Attach session context to request extensions
    request.extensions_mut().insert(session_context);
  }
  
  next.run(request).await
}

/// Extract session token from request and validate it (synchronous version)
fn extract_and_validate_session_sync(request: &Request) -> SessionContext {
  // Try header first
  if let Some(token) = extract_token_from_header(request) {
    if let Ok(payload) = SessionService::decode_token(&token) {
      return SessionContext::new(Some(payload));
    }
    // If header token is invalid, don't try cookie
    return SessionContext::new(None);
  }
  
  // Try cookie if no header
  if let Some(token) = extract_token_from_cookie(request) {
    if let Ok(payload) = SessionService::decode_token(&token) {
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
    if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
      return Some(value.to_string());
    }
  }
  
  None
}