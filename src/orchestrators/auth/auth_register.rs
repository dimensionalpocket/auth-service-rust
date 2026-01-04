use crate::services::auth_service::RegisterResult;
use crate::services::{AuthService, CookieService};
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use sqlx::{Pool, Sqlite};

pub struct AuthRegisterOrchestrator;

impl AuthRegisterOrchestrator {
  pub async fn run(
    pool: &Pool<Sqlite>,
    username: &str,
    password: &str,
    password_confirmation: &str,
    config: &DpsAuthApiConfig,
  ) -> Result<(RegisterResult, String), SessionError> {
    let mut conn = pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result = AuthService::register(
      &mut conn,
      username,
      password,
      password_confirmation,
      &config.session_secret,
    )
    .await
    .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let cookie_value = CookieService::generate_session_cookie(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
