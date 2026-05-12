use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, GetRoleByIdService};
use crate::types::{DpsAuthApiConfig, RoleError};
use crate::utils::session_context_sub_to_user_id;

pub struct GetRoleOrchestrator;

impl GetRoleOrchestrator {
  /// Get a role by ID with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Returns the role with the given ID
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsAuthApiConfig,
    role_id: i64,
  ) -> Result<Role, RoleError> {
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

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get role by ID
    let role = GetRoleByIdService::run(&mut main_conn, role_id).await?;
    match role {
      Some(role) => Ok(role),
      None => Err(RoleError::RoleNotFound(role_id)),
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_role_by_id_with_permission_check_admin_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create a role to retrieve
    let test_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRoleOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      test_role_id,
    )
    .await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.id, test_role_id);
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }

  #[dps_auth_db_test]
  async fn test_get_role_by_id_with_permission_check_unauthenticated() {
    // Create a role to try to retrieve
    let test_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = GetRoleOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      test_role_id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_role_by_id_with_permission_check_nonexistent_user() {
    // Create a role to try to retrieve
    let test_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRoleOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      test_role_id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_role_by_id_with_permission_check_forbidden() {
    // Create user role without required permissions
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create a role to try to retrieve
    let test_role_id =
      create_test_role_with_databases(&databases, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRoleOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      test_role_id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_role_by_id_with_permission_check_not_found() {
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
      GetRoleOrchestrator::run(&databases, session_context, &create_test_config(), 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }
}
