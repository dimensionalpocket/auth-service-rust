use crate::models::role::is_valid_role_permission;
use crate::models::role::Role;
use crate::queries::roles::{UpdateRoleData, UpdateRoleQuery};
use crate::types::RoleError;
use sqlx::SqliteConnection;

pub struct UpdateRoleService;

impl UpdateRoleService {
  pub async fn run(
    conn: &mut SqliteConnection,
    role_id: i64,
    update_data: UpdateRoleData,
  ) -> Result<Role, RoleError> {
    // Validate permissions if provided
    if let Some(ref permissions) = update_data.permissions {
      for permission in permissions {
        if !is_valid_role_permission(permission) {
          return Err(RoleError::InvalidPermission(permission.clone()));
        }
      }
    }

    // Create update data with the correct ID
    let update_data_with_id = UpdateRoleData {
      id: role_id,
      name: update_data.name,
      permissions: update_data.permissions,
    };

    // Run the update query
    let updated_role = UpdateRoleQuery::run(conn, update_data_with_id)
      .await
      .map_err(RoleError::DatabaseError)?;

    match updated_role {
      Some(role) => Ok(role),
      None => Err(RoleError::RoleNotFound(role_id)),
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::models::ROLE_PERMISSIONS;
  use crate::test_utils::create_test_role_model_with_conn;

  #[dps_auth_db_test]
  async fn test_update_role_success() {
    let mut conn = main_pool.acquire().await.unwrap();

    let role =
      create_test_role_model_with_conn(&mut conn, "test-role", &["can_view_user_self"], false)
        .await;
    let role_id = role.id;

    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("updated-role".to_string()),
      permissions: Some(vec![
        "can_edit_user".to_string(),
        "can_delete_user".to_string(),
      ]),
    };

    let updated_role = UpdateRoleService::run(&mut conn, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "updated-role");
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_edit_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_update_role_partial_update() {
    let mut conn = main_pool.acquire().await.unwrap();

    let role = create_test_role_model_with_conn(
      &mut conn,
      "test-role",
      &["can_view_user_self", "can_list_users"],
      false,
    )
    .await;
    let role_id = role.id;

    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("partial-updated".to_string()),
      permissions: None,
    };

    let updated_role = UpdateRoleService::run(&mut conn, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "partial-updated");
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_update_role_not_found() {
    let mut conn = main_pool.acquire().await.unwrap();

    let update_data = UpdateRoleData {
      id: 999,
      name: Some("nonexistent".to_string()),
      permissions: None,
    };

    let result = UpdateRoleService::run(&mut conn, 999, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_role_invalid_permission() {
    let mut conn = main_pool.acquire().await.unwrap();

    let role =
      create_test_role_model_with_conn(&mut conn, "test-role", &["can_view_user_self"], false)
        .await;
    let role_id = role.id;

    let update_data = UpdateRoleData {
      id: role_id,
      name: None,
      permissions: Some(vec!["invalid_permission".to_string()]),
    };

    let result = UpdateRoleService::run(&mut conn, role_id, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_role_empty_permissions() {
    let mut conn = main_pool.acquire().await.unwrap();

    let role = create_test_role_model_with_conn(
      &mut conn,
      "empty-permissions-role",
      &["can_view_user_self"],
      false,
    )
    .await;
    let role_id = role.id;

    let update_data = UpdateRoleData {
      id: role_id,
      name: None,
      permissions: Some(vec![]),
    };

    let updated_role = UpdateRoleService::run(&mut conn, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "empty-permissions-role");
    assert_eq!(updated_role.permissions.len(), 0);
  }

  #[dps_auth_db_test]
  async fn test_update_role_all_valid_permissions() {
    let mut conn = main_pool.acquire().await.unwrap();

    let role = create_test_role_model_with_conn(
      &mut conn,
      "all-permissions-role",
      &["can_view_user_self"],
      false,
    )
    .await;
    let role_id = role.id;

    let all_permissions: Vec<String> = ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("all-permissions-updated".to_string()),
      permissions: Some(all_permissions.clone()),
    };

    let updated_role = UpdateRoleService::run(&mut conn, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.name, "all-permissions-updated");
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), all_permissions.len());

    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }
}
