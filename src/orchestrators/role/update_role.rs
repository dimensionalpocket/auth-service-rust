use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::roles::UpdateRoleData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, UpdateRoleService};
use crate::types::RoleError;
use sqlx::SqlitePool;

pub struct UpdateRoleOrchestrator;

impl UpdateRoleOrchestrator {
  /// Update a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Updates the role with the given ID
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    role_id: i64,
    update_data: UpdateRoleData,
  ) -> Result<Role, RoleError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await
      .map_err(RoleError::DatabaseError)?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      CheckUserPermissionService::run(&mut conn, &user, "can_manage_roles").await?;

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Update role
    UpdateRoleService::run(&mut conn, role_id, update_data).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_update_role_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Update the role
    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: Some("updated-user".to_string()),
      permissions: Some(vec![
        "can_edit_user".to_string(),
        "can_delete_user".to_string(),
      ]),
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "updated-user");
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_edit_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_partial_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id =
      create_test_role_with_pool(&pool, "editor", &["can_edit_content", "can_view_content"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Update only the name
    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: Some("senior-editor".to_string()),
      permissions: None,
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "senior-editor");
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 2); // unchanged
    assert!(permissions.contains(&"can_edit_content".to_string()));
    assert!(permissions.contains(&"can_view_content".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to update
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: Some("updated".to_string()),
      permissions: None,
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to update
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: Some("updated".to_string()),
      permissions: None,
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a role to try to update
    let test_role_id = create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: Some("updated".to_string()),
      permissions: None,
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let update_data = crate::queries::roles::UpdateRoleData {
      id: 999, // Non-existent role ID
      name: Some("updated".to_string()),
      permissions: None,
    };

    let result = UpdateRoleOrchestrator::run(&pool, session_context, 999, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_invalid_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Update with invalid permission
    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: None,
      permissions: Some(vec!["invalid_permission".to_string()]),
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => {
        assert_eq!(permission, "invalid_permission");
      }
      _ => panic!("Expected InvalidPermission"),
    }
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_empty_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Update permissions to empty array
    let update_data = crate::queries::roles::UpdateRoleData {
      id: test_role_id,
      name: None,
      permissions: Some(vec![]), // Set to empty array
    };

    let result =
      UpdateRoleOrchestrator::run(&pool, session_context, test_role_id, update_data).await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "user"); // unchanged
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 0); // should be empty now
  }
}
