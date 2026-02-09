use crate::middleware::session::SessionContext;
use crate::models::role::ROLE_PERMISSIONS;
use crate::queries::users::GetUserByIdQuery;
use crate::services::CheckUserPermissionService;
use crate::types::RoleError;
use sqlx::SqlitePool;

pub struct GetRolePermissionsOrchestrator;

impl GetRolePermissionsOrchestrator {
  /// Get all role permissions with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission (OR user has "is_admin")
  /// - Checks if user has "can_manage_admin_role_permission" to include "is_admin" in results
  /// - Returns filtered list of permissions
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context for authentication/authorization
  ///
  /// # Returns
  /// * `Ok(Vec<String>)` - List of available role permissions (filtered by user's admin rights)
  /// * `Err(RoleError)` - Authentication or authorization error
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<String>, RoleError> {
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

    // Check permissions using scoped connection
    let allowed = CheckUserPermissionService::run(&mut conn, &user, "can_manage_roles").await?;
    let can_manage_admin =
      CheckUserPermissionService::run(&mut conn, &user, "can_manage_admin_role_permission").await?;

    if !allowed {
      return Err(RoleError::AuthorizationError(
        "Forbidden: Insufficient permissions".to_string(),
      ));
    }

    // Business logic: Filter permissions based on user's admin management rights
    let permissions: Vec<String> = ROLE_PERMISSIONS
      .iter()
      .filter(|&&perm| can_manage_admin || perm != "is_admin")
      .map(|s| s.to_string())
      .collect();

    Ok(permissions)
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
  async fn test_get_all_role_permissions_with_permission_check_without_manage_roles() {
    // Create regular user without can_manage_roles
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

    let result = GetRolePermissionsOrchestrator::run(&main_pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden: Insufficient permissions"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_all_role_permissions_with_permission_check_role_manager() {
    // Create role manager with can_manage_roles but not can_manage_admin_role_permission
    let role_manager_id =
      create_test_role_with_databases(&databases, "role_manager", &["can_manage_roles"]).await;
    let role_manager_user =
      create_test_user_with_databases(&databases, "role_manager", role_manager_id).await;

    // Create session context for role manager
    let session_payload = DpsAuthSessionPayload {
      sub: role_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolePermissionsOrchestrator::run(&main_pool, session_context).await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(!permissions.contains(&"is_admin".to_string()));
    assert!(permissions.contains(&"can_manage_roles".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len() - 1);
  }

  #[dps_auth_db_test]
  async fn test_get_all_role_permissions_with_permission_check_admin_manager() {
    // Create admin manager with both permissions
    let admin_manager_id = create_test_role_with_databases(
      &databases,
      "admin_manager",
      &["can_manage_roles", "can_manage_admin_role_permission"],
    )
    .await;
    let admin_manager_user =
      create_test_user_with_databases(&databases, "admin_manager", admin_manager_id).await;

    // Create session context for admin manager
    let session_payload = DpsAuthSessionPayload {
      sub: admin_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolePermissionsOrchestrator::run(&main_pool, session_context).await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(permissions.contains(&"is_admin".to_string()));
    assert!(permissions.contains(&"can_manage_roles".to_string()));
    assert!(permissions.contains(&"can_manage_admin_role_permission".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len());
  }

  #[dps_auth_db_test]
  async fn test_get_all_role_permissions_with_permission_check_admin() {
    // Create admin user (with is_admin, bypasses can_manage_roles requirement)
    let admin_role_id = create_test_role_with_databases(&databases, "admin", &["is_admin"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolePermissionsOrchestrator::run(&main_pool, session_context).await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(permissions.contains(&"is_admin".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len());
  }

  #[dps_auth_db_test]
  async fn test_get_all_role_permissions_with_permission_check_unauthenticated() {
    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = GetRolePermissionsOrchestrator::run(&main_pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }
}
