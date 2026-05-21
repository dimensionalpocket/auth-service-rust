use crate::middleware::session::SessionContext;
use crate::queries::users::get_user_by_id_with_role::GetUserByIdWithRoleQuery;
use crate::types::SessionError;
use crate::utils::session_context_sub_to_user_id;
use dps_config::DpsConfig;
use sqlx::SqliteConnection;
use tracing::instrument;

use super::types::AuthMeResult;

pub struct AuthGetCurrentUserService;

impl AuthGetCurrentUserService {
  #[instrument(skip(main_conn, config))]
  pub async fn run(
    main_conn: &mut SqliteConnection,
    session_context: &SessionContext,
    config: &DpsConfig,
  ) -> Result<AuthMeResult, SessionError> {
    let user_id = session_context_sub_to_user_id(session_context, config)
      .map_err(SessionError::AuthenticationError)?;

    // Get user details from database with role information
    let user_with_role = GetUserByIdWithRoleQuery::run(main_conn, user_id)
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?
      .ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

    let session_payload = session_context
      .payload
      .as_ref()
      .ok_or_else(|| SessionError::AuthenticationError("No valid session".to_string()))?;

    Ok(AuthMeResult {
      user_id: user_with_role.user.id,
      username: user_with_role.user.name,
      uuid: user_with_role.user.uuid,
      role: user_with_role.role,
      created_ts: user_with_role.user.created_ts,
      updated_ts: user_with_role.user.updated_ts,
      session_iat: session_payload.iat,
      session_exp: session_payload.exp,
    })
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_dps_config, create_test_role_model_with_conn, create_test_user_full_with_conn,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_current_user_with_role() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    let admin_role_id =
      create_test_role_model_with_conn(&mut main_conn, "admin", &["can_manage_users"], false)
        .await
        .id;
    let user = create_test_user_full_with_conn(
      &mut main_conn,
      "testuser",
      Some(admin_role_id),
      "test_password",
      None,
    )
    .await;
    let user_id = user.id;

    let payload = DpsAuthSessionPayload {
      sub: user_id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload));

    let result =
      AuthGetCurrentUserService::run(&mut main_conn, &session_context, &create_test_dps_config())
        .await;

    assert!(result.is_ok());
    let auth_me_result = result.unwrap();
    assert_eq!(auth_me_result.user_id, user_id);
    assert_eq!(auth_me_result.username, "testuser");
    assert_eq!(auth_me_result.role.name, "admin");
    assert_eq!(auth_me_result.session_iat, 1706356800);
    assert_eq!(auth_me_result.session_exp, 1706616000);
  }
}
