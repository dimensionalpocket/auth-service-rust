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
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::create_test_role_model_with_pool;

  #[dps_auth_db_test]
  async fn test_get_user_by_id_query_works_correctly() {
    create_test_role_model_with_pool(&main_pool, "user", &["can_view_user_self"], true).await;

    let mut conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut conn, "testuser", "password123")
      .await
      .unwrap();
    let retrieved_user = GetUserByIdService::run(&mut conn, user.id).await.unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, user.name);
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_id_query_not_found() {
    let mut conn = main_pool.acquire().await.unwrap();

    let user = GetUserByIdService::run(&mut conn, 999).await.unwrap();
    assert!(user.is_none());
  }
}
