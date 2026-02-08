use crate::queries::users::DeleteUserByIdQuery;
use crate::types::UserError;
use sqlx::SqliteConnection;

pub struct DeleteUserService;

impl DeleteUserService {
  pub async fn run(conn: &mut SqliteConnection, user_id: i64) -> Result<bool, UserError> {
    DeleteUserByIdQuery::run(conn, user_id)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::users::GetUserByIdQuery;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  #[tokio::test]
  async fn test_delete_user_success() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let mut conn = pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let deleted = DeleteUserService::run(&mut conn, user.id).await.unwrap();
    assert!(deleted);

    let result = GetUserByIdQuery::run(&mut conn, user.id).await.unwrap();
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_delete_user_not_found() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let mut conn = pool.acquire().await.unwrap();
    let deleted = DeleteUserService::run(&mut conn, 999).await.unwrap();
    assert!(!deleted);
  }
}
