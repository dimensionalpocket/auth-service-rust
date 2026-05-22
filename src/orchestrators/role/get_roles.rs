use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, GetAllRolesService};
use crate::types::RoleError;
use crate::utils::session_context_sub_to_user_id;
use dps_config::DpsConfig;

pub struct GetRolesOrchestrator;

impl GetRolesOrchestrator {
  /// Get all roles with permission check
  ///
  /// Validates that the user has either `can_manage_roles` OR `can_edit_user_role` permission
  /// before returning all roles.
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsConfig,
  ) -> Result<Vec<Role>, RoleError> {
    let main_pool = databases.main();
    // Authentication: Check if user is authenticated
    let user_id = session_context_sub_to_user_id(&session_context, config)
      .map_err(RoleError::AuthenticationError)?;

    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(RoleError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      CheckUserPermissionService::run(&mut main_conn, &user, "can_manage_roles").await?;
    let can_edit_user_role =
      CheckUserPermissionService::run(&mut main_conn, &user, "can_edit_user_role").await?;

    if !can_manage_roles && !can_edit_user_role {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all roles
    GetAllRolesService::run(&mut main_conn).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_dps_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_admin_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create some roles to retrieve
    create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    create_test_role_with_databases(&databases, "editor", &["can_edit_content"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    assert!(roles.len() >= 3); // admin, user, editor
    assert!(roles.iter().any(|r| r.name == "admin"));
    assert!(roles.iter().any(|r| r.name == "user"));
    assert!(roles.iter().any(|r| r.name == "editor"));
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_role_editor_success() {
    // Create role editor role and user
    let role_editor_id =
      create_test_role_with_databases(&databases, "role_editor", &["can_edit_user_role"]).await;
    let role_editor_user =
      create_test_user_with_databases(&databases, "role_editor", role_editor_id).await;

    // Create some roles to retrieve
    create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    create_test_role_with_databases(&databases, "editor", &["can_edit_content"]).await;

    // Create session context for role editor user
    let session_payload = DpsAuthSessionPayload {
      sub: role_editor_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    assert!(roles.len() >= 3); // role_editor, user, editor
    assert!(roles.iter().any(|r| r.name == "role_editor"));
    assert!(roles.iter().any(|r| r.name == "user"));
    assert!(roles.iter().any(|r| r.name == "editor"));
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_unauthenticated() {
    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_nonexistent_user() {
    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_forbidden() {
    // Create user role without required permissions
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_all_roles_with_permission_check_empty_database() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetRolesOrchestrator::run(&databases, session_context, &create_test_dps_config()).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    // Should return the admin role that was created
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "admin");
  }
}
