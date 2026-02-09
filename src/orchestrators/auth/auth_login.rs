use crate::database::Databases;
use crate::services::{AuthLoginService, AuthResult, GenerateSessionCookieService};
use crate::types::SessionError;
use crate::DpsAuthApiConfig;

pub struct AuthLoginOrchestrator;

impl AuthLoginOrchestrator {
  pub async fn run(
    databases: &Databases,
    username: &str,
    password: &str,
    config: &DpsAuthApiConfig,
  ) -> Result<(AuthResult, String), SessionError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result =
      AuthLoginService::run(&mut main_conn, username, password, &config.session_secret).await?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
