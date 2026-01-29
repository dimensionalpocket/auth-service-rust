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
  use super::*;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  #[tokio::test]
  async fn test_get_user_by_name_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
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

  #[tokio::test]
  async fn test_get_user_by_name_case_insensitive() {
    let (pool, _temp_file) = create_test_database().await;

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
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

  #[tokio::test]
  async fn test_get_user_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let user = GetUserByNameService::run(&mut conn, "nonexistent")
      .await
      .unwrap();
    assert!(user.is_none());
  }
}
