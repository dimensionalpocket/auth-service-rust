use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::services::AuthGetCurrentUserService;
use crate::types::{DpsAuthApiConfig, SessionError};

pub struct AuthMeOrchestrator;

impl AuthMeOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsAuthApiConfig,
  ) -> Result<Option<crate::services::AuthMeResult>, SessionError> {
    let main_pool = databases.main();
    match &session_context.payload {
      Some(_payload) => {
        let mut main_conn = main_pool
          .acquire()
          .await
          .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        AuthGetCurrentUserService::run(&mut main_conn, &session_context, config)
          .await
          .map(Some)
      }
      None => Ok(None),
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_role_with_databases, create_test_user_full_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_authenticated_user_success() {
    let _user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let user =
      create_test_user_full_with_databases(&databases, "testuser", Some(1), "password123", None)
        .await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthMeOrchestrator::run(&databases, session_context, &create_test_config()).await;

    assert!(result.is_ok());
    let auth_me_result = result.unwrap();
    assert!(auth_me_result.is_some());
    let result_data = auth_me_result.unwrap();
    assert_eq!(result_data.user_id, user.id);
    assert_eq!(result_data.username, "testuser");
    assert_eq!(result_data.session_iat, 1000);
    assert_eq!(result_data.session_exp, 2000);
  }

  #[dps_auth_db_test]
  async fn test_get_authenticated_user_unauthenticated() {
    let session_context = SessionContext::new(None);

    let result = AuthMeOrchestrator::run(&databases, session_context, &create_test_config()).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }
}
