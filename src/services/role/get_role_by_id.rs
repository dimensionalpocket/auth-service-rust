use crate::models::role::Role;
use crate::queries::roles::GetRoleByIdQuery;
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct GetRoleByIdService;

impl GetRoleByIdService {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, RoleError> {
    GetRoleByIdQuery::run(conn, role_id)
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
  async fn test_get_role_by_id_delegates_to_query() {
    let role = create_test_role_model_with_pool(&main_pool, "admin", &["is_admin"], false).await;
    let role_id = role.id;

    let mut conn = main_pool.acquire().await.unwrap();
    let role = GetRoleByIdService::run(&mut conn, role_id).await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert!(role.has_permission("is_admin"));
  }
}
