use crate::models::role::Role;
use crate::queries::roles::DeleteRoleQuery;
use crate::types::RoleError;
use sqlx::{Row, SqliteConnection};

pub struct DeleteRoleService;

impl DeleteRoleService {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Role, RoleError> {
    // First check if any users are using this role
    let user_count = sqlx::query("SELECT COUNT(*) FROM users WHERE role_id = ?")
      .bind(role_id)
      .fetch_one(&mut *conn)
      .await
      .map_err(RoleError::DatabaseError)?;

    let count: i64 = user_count.get(0);
    if count > 0 {
      return Err(RoleError::RoleInUse(role_id));
    }

    match DeleteRoleQuery::run(conn, role_id).await {
      Ok(role) => Ok(role),
      Err(sqlx::Error::RowNotFound) => Err(RoleError::RoleNotFound(role_id)),
      Err(err) => Err(RoleError::DatabaseError(err)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::role::create_role::CreateRoleService;
  use crate::services::role::get_role_by_id::GetRoleByIdService;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_delete_role_success() {
    let mut conn = pool.acquire().await.unwrap();

    let create_data = crate::queries::roles::CreateRoleData {
      name: "test-delete-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = CreateRoleService::run(&mut conn, create_data)
      .await
      .unwrap();

    let deleted_role = DeleteRoleService::run(&mut conn, role.id).await.unwrap();

    assert_eq!(deleted_role.id, role.id);
    assert_eq!(deleted_role.name, role.name);
    assert_eq!(deleted_role.created_ts, role.created_ts);
    assert_eq!(deleted_role.updated_ts, role.updated_ts);
    assert_eq!(deleted_role.is_default, role.is_default);

    let result = GetRoleByIdService::run(&mut conn, role.id).await.unwrap();
    assert!(result.is_none());
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_delete_role_not_found() {
    let mut conn = pool.acquire().await.unwrap();

    let result = DeleteRoleService::run(&mut conn, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound error"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_delete_role_in_use() {
    let mut conn = pool.acquire().await.unwrap();

    let role_data = crate::queries::roles::CreateRoleData {
      name: "test-in-use-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = CreateRoleService::run(&mut conn, role_data).await.unwrap();

    let user_data = CreateUserData {
      uuid: uuid::Uuid::new_v4().to_string(),
      name: "test-user".to_string(),
      password_hash: "hashed_password".to_string(),
      role_id: Some(role.id),
      metadata_json: None,
    };
    CreateUserQuery::run(&mut conn, user_data).await.unwrap();

    let result = DeleteRoleService::run(&mut conn, role.id).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleInUse(id) => assert_eq!(id, role.id),
      _ => panic!("Expected RoleInUse"),
    }
  }
}
