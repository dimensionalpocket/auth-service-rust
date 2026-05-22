use crate::queries::users::GetUserByNameWithRoleQuery;
use crate::services::{CreateSessionForUserService, CreateUserService};
use crate::types::UserError;
use dps_config::DpsConfig;
use sqlx::SqliteConnection;
use tracing::instrument;

use super::types::RegisterResult;

pub struct AuthRegisterService;

impl AuthRegisterService {
  #[instrument(skip(main_conn, config), fields(username = %username))]
  pub async fn run(
    main_conn: &mut SqliteConnection,
    username: &str,
    password: &str,
    password_confirmation: &str,
    config: &DpsConfig,
  ) -> Result<RegisterResult, UserError> {
    // Validate password confirmation matches
    if password != password_confirmation {
      return Err(UserError::ValidationError(
        "Passwords do not match".to_string(),
      ));
    }

    // Create user which includes validation and password hashing
    let user = CreateUserService::run(main_conn, username, password).await?;

    // Get user with role information
    let user_with_role = GetUserByNameWithRoleQuery::run(main_conn, username)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or_else(|| UserError::UserNotFound(user.id))?;

    // Create session for the newly created user
    let session_token = CreateSessionForUserService::run(&user, config)
      .map_err(|e| UserError::SessionError(e.to_string()))?;

    Ok(RegisterResult {
      user_id: user.id,
      username: user.name,
      uuid: user.uuid,
      role: user_with_role.role,
      created_ts: user.created_ts,
      updated_ts: user.updated_ts,
      session_token,
    })
  }
}
