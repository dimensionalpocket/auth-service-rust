use crate::services::auth_service::AuthResult;
use crate::services::{AuthService, CookieService};
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use sqlx::{Pool, Sqlite};

pub struct AuthLoginOrchestrator;

impl AuthLoginOrchestrator {
  pub async fn run(
    pool: &Pool<Sqlite>,
    username: &str,
    password: &str,
    config: &DpsAuthApiConfig,
  ) -> Result<(AuthResult, String), SessionError> {
    let mut conn = pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result =
      AuthService::login(&mut conn, username, password, &config.session_secret).await?;

    let cookie_value = CookieService::generate_session_cookie(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
