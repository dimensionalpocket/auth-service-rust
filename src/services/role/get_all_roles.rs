use crate::models::role::Role;
use crate::queries::roles::GetAllRolesQuery;
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct GetAllRolesService;

impl GetAllRolesService {
  pub async fn run(conn: &mut SqliteConnection) -> Result<Vec<Role>, RoleError> {
    GetAllRolesQuery::run(conn)
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
  async fn test_get_all_roles_delegates_to_query() {
    create_test_role_model_with_pool(
      &main_pool,
      "admin",
      &["is_admin", "can_manage_roles"],
      false,
    )
    .await;
    create_test_role_model_with_pool(&main_pool, "user", &["can_view_user_self"], true).await;

    let mut conn = main_pool.acquire().await.unwrap();
    let roles = GetAllRolesService::run(&mut conn).await.unwrap();

    assert_eq!(roles.len(), 2);

    assert_eq!(roles[0].name, "admin");
    assert!(!roles[0].is_default);
    assert!(roles[0].has_permission("is_admin"));
    assert!(roles[0].has_permission("can_manage_roles"));

    assert_eq!(roles[1].name, "user");
    assert!(roles[1].is_default);
    assert!(roles[1].has_permission("can_view_user_self"));
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_empty_table() {
    let mut conn = main_pool.acquire().await.unwrap();

    let roles = GetAllRolesService::run(&mut conn).await.unwrap();
    assert_eq!(roles.len(), 0);
  }
}
