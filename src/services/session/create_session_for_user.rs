use crate::models::user::User;
use crate::types::SessionError;
use dps_auth_session::DpsAuthSession;
use dps_config::DpsConfig;

pub struct CreateSessionForUserService;

impl CreateSessionForUserService {
  pub fn run(user: &User, config: &DpsConfig) -> Result<String, SessionError> {
    let user_record =
      serde_json::to_value(user).map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let sub = config.get_session_user_to_sub_fn()(&user_record)
      .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let payload = DpsAuthSession::create_payload(sub, None);
    let secret = config.get_auth_api_session_secret_bytes().ok_or_else(|| {
      SessionError::ConfigurationError("Session secret not configured".to_string())
    })?;
    let token = DpsAuthSession::encode_token(&payload, &secret)?;

    Ok(token)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{create_test_dps_config, create_test_role_model_with_databases};
  use dps_auth_session::DpsAuthSession;

  #[dps_auth_db_test]
  async fn test_create_session_for_user_success() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let user = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      CreateUserService::run(&mut main_conn, "testuser", "password123")
        .await
        .unwrap()
    };

    let config = create_test_dps_config();
    let token = CreateSessionForUserService::run(&user, &config).unwrap();

    assert!(!token.is_empty());

    let secret = config.get_auth_api_session_secret_bytes().unwrap();
    let payload = DpsAuthSession::decode_token(&token, &secret).unwrap();
    assert_eq!(payload.sub, user.id.to_string());
  }

  #[dps_auth_db_test]
  async fn test_create_session_for_user_different_users() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let user1 = CreateUserService::run(&mut main_conn, "user1", "password123")
      .await
      .unwrap();
    let user2 = CreateUserService::run(&mut main_conn, "user2", "password123")
      .await
      .unwrap();

    let config = create_test_dps_config();
    let token1 = CreateSessionForUserService::run(&user1, &config).unwrap();
    let token2 = CreateSessionForUserService::run(&user2, &config).unwrap();

    assert_ne!(token1, token2);

    let secret = config.get_auth_api_session_secret_bytes().unwrap();
    let payload1 = DpsAuthSession::decode_token(&token1, &secret).unwrap();
    let payload2 = DpsAuthSession::decode_token(&token2, &secret).unwrap();

    assert_eq!(payload1.sub, user1.id.to_string());
    assert_eq!(payload2.sub, user2.id.to_string());
  }
}
