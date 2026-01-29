use crate::services::{AuthLoginService, AuthResult, GenerateSessionCookieService};
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
      AuthLoginService::run(&mut conn, username, password, &config.session_secret).await?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
