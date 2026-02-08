use crate::models::user::User;
use crate::types::SessionError;
use dps_auth_session::DpsAuthSession;

pub struct CreateSessionForUserService;

impl CreateSessionForUserService {
  pub fn run(user: &User, secret: &[u8]) -> Result<String, SessionError> {
    let payload = DpsAuthSession::create_payload(user.id, None);
    let token = DpsAuthSession::encode_token(&payload, secret)?;

    Ok(token)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::CreateUserService;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};
  use dps_auth_session::DpsAuthSession;

  // Test secret - 32 bytes for AES-256
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  #[tokio::test]
  async fn test_create_session_for_user_success() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let user = {
      let mut conn = pool.acquire().await.unwrap();
      CreateUserService::run(&mut conn, "testuser", "password123")
        .await
        .unwrap()
    };

    let token = CreateSessionForUserService::run(&user, TEST_SECRET).unwrap();

    assert!(!token.is_empty());

    let payload = DpsAuthSession::decode_token(&token, TEST_SECRET).unwrap();
    assert_eq!(payload.sub, user.id);
  }

  #[tokio::test]
  async fn test_create_session_for_user_different_users() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    let user1 = CreateUserService::run(&mut conn, "user1", "password123")
      .await
      .unwrap();
    let user2 = CreateUserService::run(&mut conn, "user2", "password123")
      .await
      .unwrap();

    let token1 = CreateSessionForUserService::run(&user1, TEST_SECRET).unwrap();
    let token2 = CreateSessionForUserService::run(&user2, TEST_SECRET).unwrap();

    assert_ne!(token1, token2);

    let payload1 = DpsAuthSession::decode_token(&token1, TEST_SECRET).unwrap();
    let payload2 = DpsAuthSession::decode_token(&token2, TEST_SECRET).unwrap();

    assert_eq!(payload1.sub, user1.id);
    assert_eq!(payload2.sub, user2.id);
  }
}
