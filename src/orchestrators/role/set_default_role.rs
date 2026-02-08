use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, SetDefaultRoleService};
use crate::types::RoleError;
use sqlx::SqlitePool;

pub struct SetDefaultRoleOrchestrator;

impl SetDefaultRoleOrchestrator {
  /// Set a role as default with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Sets the role with the given ID as default
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    role_id: i64,
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

    // Business logic: Set default role
    SetDefaultRoleService::run(&mut conn, role_id).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::services::{GetAllRolesService, GetRoleByIdService};
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_admin_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;

    // Create a role to set as default
    let test_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, test_role_id).await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "user");
    assert!(updated_role.is_default);
    assert!(updated_role.has_permission("can_view_user_self"));
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_unauthenticated() {
    // Create a role to try to set as default
    let test_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_nonexistent_user() {
    // Create a role to try to set as default
    let test_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_forbidden() {
    // Create user role without required permissions
    let user_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&main_pool, "user", user_role_id).await;

    // Create a role to try to set as default
    let test_role_id =
      create_test_role_with_pool(&main_pool, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_not_found() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_atomic_behavior() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;

    // Create multiple roles
    let role1_id = create_test_role_with_pool(&main_pool, "role1", &["can_view_user_self"]).await;
    let role2_id = create_test_role_with_pool(&main_pool, "role2", &["can_list_users"]).await;
    let role3_id = create_test_role_with_pool(&main_pool, "role3", &["can_manage_roles"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Set role1 as default
    let result1 =
      SetDefaultRoleOrchestrator::run(&main_pool, session_context.clone(), role1_id).await;
    assert!(result1.is_ok());
    let updated_role1 = result1.unwrap();
    assert!(updated_role1.is_default);

    // Verify only role1 is default
    {
      let mut conn = main_pool.acquire().await.unwrap();
      let all_roles = GetAllRolesService::run(&mut conn).await.unwrap();
      let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
      assert_eq!(default_roles.len(), 1);
      assert_eq!(default_roles[0].id, role1_id);
    }

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role2 as default (should unset role1)
    let result2 =
      SetDefaultRoleOrchestrator::run(&main_pool, session_context.clone(), role2_id).await;
    assert!(result2.is_ok());
    let updated_role2 = result2.unwrap();
    assert!(updated_role2.is_default);

    // Verify only role2 is default now
    {
      let mut conn = main_pool.acquire().await.unwrap();
      let all_roles = GetAllRolesService::run(&mut conn).await.unwrap();
      let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
      assert_eq!(default_roles.len(), 1);
      assert_eq!(default_roles[0].id, role2_id);
    }

    // Verify role1 is no longer default
    {
      let mut conn = main_pool.acquire().await.unwrap();
      let current_role1 = GetRoleByIdService::run(&mut conn, role1_id).await.unwrap();
      assert!(current_role1.is_some());
      assert!(!current_role1.unwrap().is_default);
    }

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role3 as default (should unset role2)
    let result3 = SetDefaultRoleOrchestrator::run(&main_pool, session_context, role3_id).await;
    assert!(result3.is_ok());
    let updated_role3 = result3.unwrap();
    assert!(updated_role3.is_default);

    // Verify only role3 is default now
    {
      let mut conn = main_pool.acquire().await.unwrap();
      let all_roles = GetAllRolesService::run(&mut conn).await.unwrap();
      let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
      assert_eq!(default_roles.len(), 1);
      assert_eq!(default_roles[0].id, role3_id);
    }

    // Verify role2 is no longer default
    {
      let mut conn = main_pool.acquire().await.unwrap();
      let current_role2 = GetRoleByIdService::run(&mut conn, role2_id).await.unwrap();
      assert!(current_role2.is_some());
      assert!(!current_role2.unwrap().is_default);
    }
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_with_permission_check_timestamp_update() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;

    // Create a role to set as default
    let test_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;

    // Get role before setting as default to compare timestamps
    let role_before = {
      let mut conn = main_pool.acquire().await.unwrap();
      GetRoleByIdService::run(&mut conn, test_role_id)
        .await
        .unwrap()
        .unwrap()
    };

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role as default
    let result = SetDefaultRoleOrchestrator::run(&main_pool, session_context, test_role_id).await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert!(updated_role.is_default);
    assert!(updated_role.updated_ts > role_before.updated_ts);
    assert_eq!(updated_role.created_ts, role_before.created_ts);
  }
}
