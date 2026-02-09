use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::roles::CreateRoleData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, CreateRoleService};
use crate::types::RoleError;

pub struct AddRoleOrchestrator;

impl AddRoleOrchestrator {
  /// Create a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Creates role with is_default set to false
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    create_data: CreateRoleData,
  ) -> Result<Role, RoleError> {
    let main_pool = databases.main();
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(RoleError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .map_err(RoleError::DatabaseError)?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      CheckUserPermissionService::run(&mut main_conn, &user, "can_manage_roles").await?;

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    let create_data_with_default_false = CreateRoleData {
      name: create_data.name,
      permissions: create_data.permissions,
      is_default: false,
    };
    CreateRoleService::run(&mut main_conn, create_data_with_default_false).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_databases, create_test_user_with_databases};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_admin_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data
    let create_data = CreateRoleData {
      name: "new_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "can_list_users".to_string(),
      ],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.name, "new_role");
    assert!(!role.is_default);
    assert!(role.created_ts > 0);
    assert_eq!(role.created_ts, role.updated_ts);

    let permissions = role.permissions;
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_empty_permissions() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data with empty permissions
    let create_data = CreateRoleData {
      name: "empty_permissions_role".to_string(),
      permissions: vec![],
      is_default: true,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.name, "empty_permissions_role");
    assert!(!role.is_default); // Always false for now
    assert_eq!(role.permissions.len(), 0);
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_unauthenticated() {
    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_nonexistent_user() {
    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_forbidden() {
    // Create user role without required permissions
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_validation_error() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data with empty name (validation error)
    let create_data = CreateRoleData {
      name: "".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => {
        assert_eq!(msg, "Role name cannot be empty");
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_invalid_permission() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data with invalid permission
    let create_data = CreateRoleData {
      name: "invalid_permission_role".to_string(),
      permissions: vec!["invalid_permission".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => {
        assert_eq!(permission, "invalid_permission");
      }
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_duplicate_name() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create an existing role first
    create_test_role_with_databases(&databases, "existing_role", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data with duplicate name
    let create_data = CreateRoleData {
      name: "existing_role".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNameAlreadyExists(name) => {
        assert_eq!(name, "existing_role");
      }
      _ => panic!("Expected RoleNameAlreadyExists error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_role_with_permission_check_all_valid_permissions() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Create role data with all valid permissions
    let all_permissions: Vec<String> = crate::models::ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let create_data = CreateRoleData {
      name: "all_permissions_role".to_string(),
      permissions: all_permissions.clone(),
      is_default: false,
    };

    let result = AddRoleOrchestrator::run(&databases, session_context, create_data).await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.name, "all_permissions_role");
    assert!(!role.is_default);

    let permissions = role.permissions;
    assert_eq!(permissions.len(), all_permissions.len());

    // Verify all permissions are present
    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }
}
