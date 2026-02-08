use crate::middleware::session::SessionContext;
use crate::models::role::Role;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, DeleteRoleService};
use crate::types::RoleError;
use sqlx::SqlitePool;

pub struct RemoveRoleOrchestrator;

impl RemoveRoleOrchestrator {
  /// Delete a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Deletes the role with the given ID
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

    // Business logic: Delete role
    DeleteRoleService::run(&mut conn, role_id).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::GetUserByIdQuery;
  use crate::services::GetRoleByIdService;
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_delete_role_with_permission_check_admin_success() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to delete
    let test_role_id =
      create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_ok());
    let deleted_role = result.unwrap();
    assert_eq!(deleted_role.id, test_role_id);
    assert_eq!(deleted_role.name, "test_role");

    // Verify role is actually deleted
    let mut conn = pool.acquire().await.unwrap();
    let check_result = GetRoleByIdService::run(&mut conn, test_role_id).await;
    assert!(check_result.is_ok());
    assert!(check_result.unwrap().is_none());
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_unauthenticated() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create a role to try to delete
    let test_role_id =
      create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_nonexistent_user() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create a role to try to delete
    let test_role_id =
      create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_forbidden() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a role to try to delete
    let test_role_id = create_test_role_with_pool(&pool, "test_role", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_not_found() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

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

    let result = RemoveRoleOrchestrator::run(&pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_role_in_use() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to delete
    let test_role_id =
      create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create a user with the role to be deleted
    let user_with_role = create_test_user_with_pool(&pool, "user_with_role", test_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleInUse(id) => {
        assert_eq!(id, test_role_id);
      }
      _ => panic!("Expected RoleInUse"),
    }

    // Verify the user still exists and has the role
    let check_user = {
      let mut conn = pool.acquire().await.unwrap();
      GetUserByIdQuery::run(&mut conn, user_with_role.id).await
    };
    assert!(check_user.is_ok());
    assert!(check_user.unwrap().is_some());

    // Verify the role still exists
    let mut conn = pool.acquire().await.unwrap();
    let check_role = GetRoleByIdService::run(&mut conn, test_role_id).await;
    assert!(check_role.is_ok());
    assert!(check_role.unwrap().is_some());
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_returns_deleted_data() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role with specific data to delete
    let test_role_id = create_test_role_with_pool(
      &pool,
      "detailed_role",
      &["can_edit_user", "can_delete_user"],
    )
    .await;

    // Get the role data before deletion for comparison
    let role_before = {
      let mut conn = pool.acquire().await.unwrap();
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

    let result = RemoveRoleOrchestrator::run(&pool, session_context, test_role_id).await;

    assert!(result.is_ok());
    let deleted_role = result.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_role.id, role_before.id);
    assert_eq!(deleted_role.name, role_before.name);
    assert_eq!(deleted_role.created_ts, role_before.created_ts);
    assert_eq!(deleted_role.updated_ts, role_before.updated_ts);
    assert_eq!(deleted_role.is_default, role_before.is_default);

    let permissions_before = role_before.permissions;
    let permissions_deleted = deleted_role.permissions;
    assert_eq!(permissions_before.len(), permissions_deleted.len());
    for permission in permissions_before {
      assert!(permissions_deleted.contains(&permission));
    }
  }
}
