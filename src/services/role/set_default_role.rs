use crate::models::role::Role;
use crate::queries::roles::SetDefaultRoleQuery;
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct SetDefaultRoleService;

impl SetDefaultRoleService {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Role, RoleError> {
    match SetDefaultRoleQuery::run(conn, role_id).await {
      Ok(Some(role)) => Ok(role),
      Ok(None) => Err(RoleError::RoleNotFound(role_id)),
      Err(err) => Err(RoleError::DatabaseError(err)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::roles::GetDefaultRoleQuery;
  use crate::services::role::create_role::CreateRoleService;
  use crate::services::role::get_all_roles::GetAllRolesService;
  use crate::services::role::get_role_by_id::GetRoleByIdService;
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_set_default_role_success() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let mut conn = pool.acquire().await.unwrap();

    let role1_data = crate::queries::roles::CreateRoleData {
      name: "role1".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };
    let role1 = CreateRoleService::run(&mut conn, role1_data).await.unwrap();

    let role2_data = crate::queries::roles::CreateRoleData {
      name: "role2".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };
    let role2 = CreateRoleService::run(&mut conn, role2_data).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let updated_role1 = SetDefaultRoleService::run(&mut conn, role1.id)
      .await
      .unwrap();

    assert_eq!(updated_role1.id, role1.id);
    assert_eq!(updated_role1.name, "role1");
    assert!(updated_role1.is_default);
    assert!(updated_role1.updated_ts > role1.updated_ts);

    let default_role = GetDefaultRoleQuery::run(&mut conn).await.unwrap();
    assert!(default_role.is_some());
    assert_eq!(default_role.unwrap().id, role1.id);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let updated_role2 = SetDefaultRoleService::run(&mut conn, role2.id)
      .await
      .unwrap();

    assert_eq!(updated_role2.id, role2.id);
    assert_eq!(updated_role2.name, "role2");
    assert!(updated_role2.is_default);

    let default_role = GetDefaultRoleQuery::run(&mut conn).await.unwrap();
    assert!(default_role.is_some());
    assert_eq!(default_role.unwrap().id, role2.id);

    let current_role1 = GetRoleByIdService::run(&mut conn, role1.id).await.unwrap();
    assert!(current_role1.is_some());
    assert!(!current_role1.unwrap().is_default);
  }

  #[tokio::test]
  async fn test_set_default_role_not_found() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let mut conn = pool.acquire().await.unwrap();

    let result = SetDefaultRoleService::run(&mut conn, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound"),
    }
  }

  #[tokio::test]
  async fn test_set_default_role_atomic_behavior() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let mut conn = pool.acquire().await.unwrap();

    let role1_data = crate::queries::roles::CreateRoleData {
      name: "atomic-role1".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };
    CreateRoleService::run(&mut conn, role1_data).await.unwrap();

    let role2_data = crate::queries::roles::CreateRoleData {
      name: "atomic-role2".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };
    let role2 = CreateRoleService::run(&mut conn, role2_data).await.unwrap();

    let role3_data = crate::queries::roles::CreateRoleData {
      name: "atomic-role3".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role3 = CreateRoleService::run(&mut conn, role3_data).await.unwrap();

    SetDefaultRoleService::run(&mut conn, role2.id)
      .await
      .unwrap();

    let all_roles = GetAllRolesService::run(&mut conn).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role2.id);

    SetDefaultRoleService::run(&mut conn, role3.id)
      .await
      .unwrap();

    let all_roles = GetAllRolesService::run(&mut conn).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role3.id);

    let current_role2 = GetRoleByIdService::run(&mut conn, role2.id).await.unwrap();
    assert!(current_role2.is_some());
    assert!(!current_role2.unwrap().is_default);
  }
}
