use crate::middleware::session::SessionContext;
use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{UserError, UserService};
use sqlx::SqlitePool;

pub struct AuthOrchestrator;

impl AuthOrchestrator {
  pub async fn change_authenticated_user_password(
    pool: &SqlitePool,
    session_context: SessionContext,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    // Authentication: Check if user is authenticated
    let session_payload = session_context
      .payload
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let user_id = session_payload.sub;

    // Authorization: Verify user exists
    let _user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(UserError::ValidationError("User not found".to_string()))?;

    // Business logic: Update password
    UserService::update_password(
      pool,
      user_id,
      current_password,
      new_password,
      new_password_confirmation,
    )
    .await
  }
}
