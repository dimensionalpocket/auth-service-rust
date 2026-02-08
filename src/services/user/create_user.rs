use crate::models::User;
use crate::queries::users::{CreateUserData, CreateUserQuery, GetUserByNameQuery};
use crate::services::GeneratePasswordHashService;
use crate::types::UserError;
use sqlx::SqliteConnection;
use uuid::Uuid;

use super::{ValidateUserNameService, ValidateUserPasswordService};

pub struct CreateUserService;

impl CreateUserService {
  pub async fn run(
    conn: &mut SqliteConnection,
    username: &str,
    password: &str,
  ) -> Result<User, UserError> {
    ValidateUserNameService::run(username)?;
    ValidateUserPasswordService::run(password)?;

    // Check if username already exists
    if let Some(_existing_user) = GetUserByNameQuery::run(conn, username).await? {
      return Err(UserError::UsernameAlreadyExists(username.to_string()));
    }

    // Hash the password
    let password_hash = GeneratePasswordHashService::run(password)?;

    // Generate UUID for the user
    let user_uuid = Uuid::new_v4().to_string();

    let create_data = CreateUserData {
      uuid: user_uuid,
      name: username.to_string(),
      role_id: None,
      password_hash,
      metadata_json: None,
    };

    CreateUserQuery::run(conn, create_data)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::create_test_role_model_with_pool;

  #[dps_auth_db_test]
  async fn test_create_user_success() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();

    assert_eq!(user.name, "testuser");
    assert!(user.id > 0);
    assert!(!user.uuid.is_empty());
    assert!(!user.password_hash.is_empty());
    assert!(user.password_hash.starts_with("$argon2"));
    assert!(user.created_ts > 0);
    assert_eq!(user.created_ts, user.updated_ts);
  }

  #[dps_auth_db_test]
  async fn test_create_user_username_already_exists() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let result = CreateUserService::run(&mut conn, "testuser", "password456").await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => assert_eq!(username, "testuser"),
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_user_case_insensitive_username_check() {
    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let result = CreateUserService::run(&mut conn, "TestUser", "password456").await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => assert_eq!(username, "TestUser"),
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }
}
