use crate::services::{AuthRegisterService, GenerateSessionCookieService, RegisterResult};
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

    let auth_result = AuthRegisterService::run(
      &mut conn,
      username,
      password,
      password_confirmation,
      &config.session_secret,
    )
    .await
    .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
