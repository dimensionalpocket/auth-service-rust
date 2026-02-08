use crate::models::User;
use crate::queries::users::GetUserByNameQuery;
use sqlx::SqliteConnection;

pub struct GetUserByNameService;

impl GetUserByNameService {
  pub async fn run(conn: &mut SqliteConnection, name: &str) -> Result<Option<User>, sqlx::Error> {
    GetUserByNameQuery::run(conn, name).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::create_test_role_model_with_pool;

  #[dps_auth_db_test]
  async fn test_get_user_by_name_delegates_to_query() {
    create_test_role_model_with_pool(&main_pool, "user", &["can_view_user_self"], true).await;

    let mut conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let retrieved_user = GetUserByNameService::run(&mut conn, "testuser")
      .await
      .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, user.name);
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_name_case_insensitive() {
    create_test_role_model_with_pool(&main_pool, "user", &["can_view_user_self"], true).await;

    let mut conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "TestUser", "password123")
      .await
      .unwrap();
    let retrieved_user = GetUserByNameService::run(&mut conn, "testuser")
      .await
      .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, "TestUser");
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_name_not_found() {
    let mut conn = main_pool.acquire().await.unwrap();
    let user = GetUserByNameService::run(&mut conn, "nonexistent")
      .await
      .unwrap();
    assert!(user.is_none());
  }
}
