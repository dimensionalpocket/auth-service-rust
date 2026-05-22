use crate::queries::users::GetUserByNameWithRoleQuery;
use crate::services::CreateSessionService;
use crate::types::SessionError;
use dps_config::DpsConfig;
use sqlx::SqliteConnection;
use tracing::instrument;

use super::types::AuthResult;

pub struct AuthLoginService;

impl AuthLoginService {
  #[instrument(skip(main_conn, config, password), fields(username = %username))]
  pub async fn run(
    main_conn: &mut SqliteConnection,
    username: &str,
    password: &str,
    config: &DpsConfig,
  ) -> Result<AuthResult, SessionError> {
    // Find user by username with role information first
    let user_with_role = GetUserByNameWithRoleQuery::run(main_conn, username)
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    // Check if user exists
    let user_with_role = user_with_role
      .ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

    // Create session which includes password verification
    let session_token = CreateSessionService::run(main_conn, username, password, config).await?;

    Ok(AuthResult {
      user_id: user_with_role.user.id,
      username: user_with_role.user.name,
      role: user_with_role.role,
      session_token,
    })
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{create_test_dps_config, create_test_role_model_with_databases};

  #[dps_auth_db_test]
  async fn test_auth_login_success() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let mut main_conn = main_pool.acquire().await.unwrap();
    CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();
    let config = create_test_dps_config();
    let result = AuthLoginService::run(&mut main_conn, "testuser", "password123", &config).await;

    assert!(result.is_ok());
    let auth_result = result.unwrap();
    assert_eq!(auth_result.username, "testuser");
    assert!(!auth_result.session_token.is_empty());
    assert!(auth_result.user_id > 0);
  }

  #[dps_auth_db_test]
  async fn test_auth_login_invalid_credentials() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let mut main_conn = main_pool.acquire().await.unwrap();
    CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();
    let config = create_test_dps_config();
    let result = AuthLoginService::run(&mut main_conn, "testuser", "wrongpassword", &config).await;

    assert!(result.is_err());
  }

  #[dps_auth_db_test]
  async fn test_auth_login_user_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let config = create_test_dps_config();
    let result = AuthLoginService::run(&mut main_conn, "nonexistent", "password123", &config).await;

    assert!(result.is_err());
  }
}
