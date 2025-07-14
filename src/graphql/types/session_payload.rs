use async_graphql::SimpleObject;
use dp_auth_session_service::DpAuthSessionPayload as ServiceSessionPayload;

/// GraphQL representation of a user session payload
///
/// Contains the essential information about an authenticated user's session,
/// including their identity and session timing details.
#[derive(SimpleObject)]
pub struct SessionPayload {
  /// The unique identifier of the authenticated user
  pub sub: i64,
  /// When this session was created (seconds since Unix epoch)
  pub iat: i64,
  /// When this session expires (seconds since Unix epoch)
  pub exp: i64,
}

impl From<ServiceSessionPayload> for SessionPayload {
  fn from(payload: ServiceSessionPayload) -> Self {
    Self {
      sub: payload.sub,
      iat: payload.iat,
      exp: payload.exp,
    }
  }
}
