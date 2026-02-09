use crate::queries::users::DeleteUserByIdQuery;
use crate::types::UserError;
use sqlx::SqliteConnection;

pub struct DeleteUserService;

impl DeleteUserService {
  pub async fn run(main_conn: &mut SqliteConnection, user_id: i64) -> Result<bool, UserError> {
    DeleteUserByIdQuery::run(main_conn, user_id)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::queries::users::GetUserByIdQuery;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::create_test_role_model_with_databases;

  #[dps_auth_db_test]
  async fn test_delete_user_success() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();
    let deleted = DeleteUserService::run(&mut main_conn, user.id)
      .await
      .unwrap();
    assert!(deleted);

    let result = GetUserByIdQuery::run(&mut main_conn, user.id)
      .await
      .unwrap();
    assert!(result.is_none());
  }

  #[dps_auth_db_test]
  async fn test_delete_user_not_found() {
    let mut main_conn = main_pool.acquire().await.unwrap();
    let deleted = DeleteUserService::run(&mut main_conn, 999).await.unwrap();
    assert!(!deleted);
  }
}
