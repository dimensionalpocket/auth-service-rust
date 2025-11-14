use crate::graphql::types::SessionPayload;
use crate::middleware::session::SessionContext;
use async_graphql::{Context, Object, Result};

/// Current session query resolver providing authenticated user session information
#[derive(Default, Debug)]
pub struct GetCurrentSessionResolver;

#[Object]
impl GetCurrentSessionResolver {
  /// Returns the current session payload if the user is authenticated.
  ///
  /// This query retrieves the session information that was set by the session middleware
  /// when processing the request. The session contains:
  /// - User ID (sub): The unique identifier of the authenticated user
  /// - Issued at (iat): When the session token was created (seconds since epoch)
  /// - Expiration (exp): When the session token expires (seconds since epoch)
  ///
  /// Returns `null` if no valid session token was provided in the request.
  ///
  /// # Authentication
  ///
  /// This query requires a valid session token to be provided either:
  /// - In the `Authorization` header as `Bearer <token>`
  /// - In the `DpsAuthSession` cookie
  ///
  /// # Examples
  ///
  /// **Authenticated request:**
  /// ```graphql
  /// query {
  ///   getCurrentSession {
  ///     sub
  ///     iat
  ///     exp
  ///   }
  /// }
  /// ```
  ///
  /// **Response for authenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "getCurrentSession": {
  ///       "sub": 123,
  ///       "iat": 1706356800,
  ///       "exp": 1706616000
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// **Response for unauthenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "getCurrentSession": null
  ///   }
  /// }
  /// ```
  async fn get_current_session(&self, ctx: &Context<'_>) -> Result<Option<SessionPayload>> {
    let session_context = SessionContext::from_context(ctx)?;

    match &session_context.payload {
      Some(payload) => Ok(Some(payload.clone().into())),
      None => Ok(None),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use async_graphql::*;
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_get_current_session_with_authenticated_user() {
    let query = GetCurrentSessionResolver;
    let payload = ServiceSessionPayload {
      sub: 123,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload.clone()));

    let schema = Schema::build(query, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();

    let result = schema
      .execute("{ getCurrentSession { sub iat exp } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["getCurrentSession"]["sub"], 123);
    assert_eq!(data["getCurrentSession"]["iat"], 1706356800);
    assert_eq!(data["getCurrentSession"]["exp"], 1706616000);
  }

  #[tokio::test]
  async fn test_get_current_session_with_unauthenticated_user() {
    let query = GetCurrentSessionResolver;
    let session_context = SessionContext::new(None);

    let schema = Schema::build(query, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();

    let result = schema
      .execute("{ getCurrentSession { sub iat exp } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["getCurrentSession"].is_null());
  }

  #[tokio::test]
  async fn test_get_current_session_missing_context() {
    let query = GetCurrentSessionResolver;

    let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
    let result = schema
      .execute("{ getCurrentSession { sub iat exp } }")
      .await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Internal server error"));
  }
}
