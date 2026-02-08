use crate::middleware::session::SessionContext;
use crate::services::AuthGetCurrentUserService;
use crate::types::SessionError;
use sqlx::SqlitePool;

pub struct AuthMeOrchestrator;

impl AuthMeOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Option<crate::services::AuthMeResult>, SessionError> {
    match &session_context.payload {
      Some(_payload) => {
        let mut conn = pool
          .acquire()
          .await
          .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        AuthGetCurrentUserService::run(&mut conn, &session_context)
          .await
          .map(Some)
      }
      None => Ok(None),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_full_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_authenticated_user_success() {
    let _user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let user =
      create_test_user_full_with_pool(&pool, "testuser", Some(1), "password123", None).await;

    let session_payload = DpsAuthSessionPayload {
      sub: user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = AuthMeOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let auth_me_result = result.unwrap();
    assert!(auth_me_result.is_some());
    let result_data = auth_me_result.unwrap();
    assert_eq!(result_data.user_id, user.id);
    assert_eq!(result_data.username, "testuser");
    assert_eq!(result_data.session_iat, 1000);
    assert_eq!(result_data.session_exp, 2000);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_authenticated_user_unauthenticated() {
    let session_context = SessionContext::new(None);

    let result = AuthMeOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }
}
