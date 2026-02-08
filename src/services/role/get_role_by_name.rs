use crate::models::role::Role;
use crate::queries::roles::GetRoleByNameQuery;
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct GetRoleByNameService;

impl GetRoleByNameService {
  pub async fn run(conn: &mut SqliteConnection, name: &str) -> Result<Option<Role>, RoleError> {
    GetRoleByNameQuery::run(conn, name)
      .await
      .map_err(RoleError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::create_test_role_model_with_pool;

  #[dps_auth_db_test]
  async fn test_get_role_by_name_delegates_to_query() {
    create_test_role_model_with_pool(&main_pool, "user", &["can_view_user_self"], true).await;

    let mut conn = main_pool.acquire().await.unwrap();
    let role = GetRoleByNameService::run(&mut conn, "user").await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }
}
