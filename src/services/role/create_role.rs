use crate::models::role::is_valid_role_permission;
use crate::models::role::Role;
use crate::queries::roles::{CreateRoleData, CreateRoleQuery, GetRoleByNameQuery};
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct CreateRoleService;

impl CreateRoleService {
  pub async fn run(
    conn: &mut SqliteConnection,
    create_data: CreateRoleData,
  ) -> Result<Role, RoleError> {
    // Validate role name format
    if create_data.name.trim().is_empty() {
      return Err(RoleError::ValidationError(
        "Role name cannot be empty".to_string(),
      ));
    }

    // Validate all permissions are valid
    for permission in &create_data.permissions {
      if !is_valid_role_permission(permission) {
        return Err(RoleError::InvalidPermission(permission.clone()));
      }
    }

    // Check if role name already exists and create role in one connection block
    let existing_role = GetRoleByNameQuery::run(conn, &create_data.name)
      .await
      .map_err(RoleError::DatabaseError)?;

    if existing_role.is_some() {
      return Err(RoleError::RoleNameAlreadyExists(create_data.name));
    }

    let create_data_with_default_false = CreateRoleData {
      name: create_data.name,
      permissions: create_data.permissions,
      is_default: false,
    };
    CreateRoleQuery::run(conn, create_data_with_default_false)
      .await
      .map_err(RoleError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::models::ROLE_PERMISSIONS;

  #[dps_auth_db_test]
  async fn test_create_role_success() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "can_list_users".to_string(),
      ],
      is_default: false,
    };

    let role = CreateRoleService::run(&mut conn, create_data)
      .await
      .unwrap();

    assert_eq!(role.name, "test_role");
    assert!(!role.is_default);
    assert!(role.created_ts > 0);
    assert_eq!(role.created_ts, role.updated_ts);

    let permissions = role.permissions;
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_create_role_empty_permissions() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "empty_permissions_role".to_string(),
      permissions: vec![],
      is_default: true,
    };

    let role = CreateRoleService::run(&mut conn, create_data)
      .await
      .unwrap();

    assert_eq!(role.name, "empty_permissions_role");
    assert!(!role.is_default);
    assert_eq!(role.permissions.len(), 0);
  }

  #[dps_auth_db_test]
  async fn test_create_role_duplicate_name_fails() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data1 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    CreateRoleService::run(&mut conn, create_data1)
      .await
      .unwrap();

    let create_data2 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };

    let result = CreateRoleService::run(&mut conn, create_data2).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNameAlreadyExists(name) => assert_eq!(name, "duplicate"),
      _ => panic!("Expected RoleNameAlreadyExists error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_empty_name_fails() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = CreateRoleService::run(&mut conn, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => assert_eq!(msg, "Role name cannot be empty"),
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_whitespace_name_fails() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "   ".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = CreateRoleService::run(&mut conn, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => assert_eq!(msg, "Role name cannot be empty"),
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_invalid_permission_fails() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "invalid_permission_role".to_string(),
      permissions: vec!["invalid_permission".to_string()],
      is_default: false,
    };

    let result = CreateRoleService::run(&mut conn, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_multiple_invalid_permissions_fails() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "multiple_invalid_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "invalid_permission1".to_string(),
        "invalid_permission2".to_string(),
      ],
      is_default: false,
    };

    let result = CreateRoleService::run(&mut conn, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission1"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_all_valid_permissions() {
    let mut conn = main_pool.acquire().await.unwrap();

    let all_permissions: Vec<String> = ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let create_data = CreateRoleData {
      name: "all_permissions_role".to_string(),
      permissions: all_permissions.clone(),
      is_default: false,
    };

    let role = CreateRoleService::run(&mut conn, create_data)
      .await
      .unwrap();

    assert_eq!(role.name, "all_permissions_role");
    assert!(!role.is_default);

    let permissions = role.permissions;
    assert_eq!(permissions.len(), all_permissions.len());

    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_default_role() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = CreateRoleData {
      name: "default_test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: true,
    };

    let role = CreateRoleService::run(&mut conn, create_data)
      .await
      .unwrap();

    assert_eq!(role.name, "default_test_role");
    assert!(!role.is_default);
    assert_eq!(role.permissions.len(), 1);
    assert!(role.permissions.contains(&"can_view_user_self".to_string()));
  }
}
