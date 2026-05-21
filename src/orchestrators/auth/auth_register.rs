use crate::database::Databases;
use crate::services::{AuthRegisterService, GenerateSessionCookieService, RegisterResult};
use crate::types::SessionError;
use dps_config::DpsConfig;

pub struct AuthRegisterOrchestrator;

impl AuthRegisterOrchestrator {
  pub async fn run(
    databases: &Databases,
    username: &str,
    password: &str,
    password_confirmation: &str,
    config: &DpsConfig,
  ) -> Result<(RegisterResult, String), SessionError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result = AuthRegisterService::run(
      &mut main_conn,
      username,
      password,
      password_confirmation,
      &config.get_auth_api_session_secret_bytes().unwrap(),
    )
    .await
    .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
