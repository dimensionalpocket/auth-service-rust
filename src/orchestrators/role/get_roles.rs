use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, GetAllRolesService};
use crate::types::RoleError;
use sqlx::SqlitePool;

pub struct GetRolesOrchestrator;

impl GetRolesOrchestrator {
  /// Get all roles with permission check
  ///
  /// Validates that the user has either `can_manage_roles` OR `can_edit_user_role` permission
  /// before returning all roles.
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<Role>, RoleError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      CheckUserPermissionService::run(&mut conn, &user, "can_manage_roles").await?;
    let can_edit_user_role =
      CheckUserPermissionService::run(&mut conn, &user, "can_edit_user_role").await?;

    if !can_manage_roles && !can_edit_user_role {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all roles
    GetAllRolesService::run(&mut conn).await
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
  async fn test_get_all_roles_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create some roles to retrieve
    create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    assert!(roles.len() >= 3); // admin, user, editor
    assert!(roles.iter().any(|r| r.name == "admin"));
    assert!(roles.iter().any(|r| r.name == "user"));
    assert!(roles.iter().any(|r| r.name == "editor"));
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_role_editor_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role editor role and user
    let role_editor_id =
      create_test_role_with_pool(&pool, "role_editor", &["can_edit_user_role"]).await;
    let role_editor_user = create_test_user_with_pool(&pool, "role_editor", role_editor_id).await;

    // Create some roles to retrieve
    create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for role editor user
    let session_payload = DpsAuthSessionPayload {
      sub: role_editor_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    assert!(roles.len() >= 3); // role_editor, user, editor
    assert!(roles.iter().any(|r| r.name == "role_editor"));
    assert!(roles.iter().any(|r| r.name == "user"));
    assert!(roles.iter().any(|r| r.name == "editor"));
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_empty_database() {
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

    let result = GetRolesOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    // Should return the admin role that was created
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "admin");
  }
}
