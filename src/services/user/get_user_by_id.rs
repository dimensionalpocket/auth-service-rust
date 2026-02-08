use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use sqlx::SqliteConnection;

pub struct GetUserByIdService;

impl GetUserByIdService {
  pub async fn run(conn: &mut SqliteConnection, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    GetUserByIdQuery::run(conn, user_id).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  #[tokio::test]
  async fn test_get_user_by_id_query_works_correctly() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let retrieved_user = GetUserByIdService::run(&mut conn, user.id).await.unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, user.name);
  }

  #[tokio::test]
  async fn test_get_user_by_id_query_not_found() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let mut conn = pool.acquire().await.unwrap();

    let user = GetUserByIdService::run(&mut conn, 999).await.unwrap();
    assert!(user.is_none());
  }
}
