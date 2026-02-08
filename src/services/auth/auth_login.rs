use crate::queries::users::GetUserByNameWithRoleQuery;
use crate::services::CreateSessionService;
use crate::types::SessionError;
use sqlx::SqliteConnection;
use tracing::instrument;

use super::types::AuthResult;

pub struct AuthLoginService;

impl AuthLoginService {
  #[instrument(skip(conn, session_secret, password), fields(username = %username))]
  pub async fn run(
    conn: &mut SqliteConnection,
    username: &str,
    password: &str,
    session_secret: &[u8],
  ) -> Result<AuthResult, SessionError> {
    // Find user by username with role information first
    let user_with_role = GetUserByNameWithRoleQuery::run(conn, username)
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    // Check if user exists
    let user_with_role = user_with_role
      .ok_or_else(|| SessionError::AuthenticationError("User not found".to_string()))?;

    // Create session which includes password verification
    let session_token = CreateSessionService::run(conn, username, password, session_secret).await?;

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
  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  // Test secret - 32 bytes for AES-256
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  #[tokio::test]
  async fn test_auth_login_success() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let result = AuthLoginService::run(&mut conn, "testuser", "password123", TEST_SECRET).await;

    assert!(result.is_ok());
    let auth_result = result.unwrap();
    assert_eq!(auth_result.username, "testuser");
    assert!(!auth_result.session_token.is_empty());
    assert!(auth_result.user_id > 0);
  }

  #[tokio::test]
  async fn test_auth_login_invalid_credentials() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let result = AuthLoginService::run(&mut conn, "testuser", "wrongpassword", TEST_SECRET).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_auth_login_user_not_found() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let mut conn = pool.acquire().await.unwrap();
    let result = AuthLoginService::run(&mut conn, "nonexistent", "password123", TEST_SECRET).await;

    assert!(result.is_err());
  }
}
