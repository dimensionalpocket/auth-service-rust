use crate::middleware::session::SESSION_COOKIE_NAME;
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Object, Result, SimpleObject};
use std::sync::Arc;
use tracing::instrument;

/// GraphQL output type for logout response
#[derive(SimpleObject)]
pub struct AuthLogoutResponse {
  /// Success message
  pub message: String,
}

/// GraphQL mutation for user logout
#[derive(Default)]
pub struct AuthLogoutResolver;

#[Object]
impl AuthLogoutResolver {
  /// Logout user by clearing the session cookie
  #[instrument(skip(self, ctx))]
  #[graphql(name = "authLogout")]
  async fn auth_logout(&self, ctx: &Context<'_>) -> Result<AuthLogoutResponse> {
    let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
    let cookie_domain = config.cookie_domain.clone();
    let insecure_cookie = config.insecure_cookie;

    // Set cookie to expire in the past to effectively delete it
    let cookie_value = format!(
      "{}=; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      SESSION_COOKIE_NAME,
      cookie_domain,
      if insecure_cookie { "" } else { "; Secure" }
    );

    // Use append to set the expired cookie
    let _ = ctx.append_http_header("set-cookie", cookie_value);

    Ok(AuthLogoutResponse {
      message: "Successfully logged out".to_string(),
    })
  }
}
