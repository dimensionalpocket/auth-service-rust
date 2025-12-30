use crate::middleware::session::SessionContext;
use crate::models::role::{Role, ROLE_PERMISSIONS};
use crate::queries::roles::{CreateRoleData, UpdateRoleData};
use crate::queries::users::GetUserByIdQuery;
use crate::services::role_service::{RoleError, RoleService};
use sqlx::SqlitePool;

pub struct RoleOrchestrator;

impl RoleOrchestrator {
  /// Get all roles with permission check
  ///
  /// Validates that the user has either `can_manage_roles` OR `can_edit_user_role` permission
  /// before returning all roles.
  pub async fn get_all_roles_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<Role>, RoleError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let (can_manage_roles, can_edit_user_role) = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      let can_manage_roles =
        RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?;
      let can_edit_user_role =
        RoleService::check_user_permission(&mut conn, &user, "can_edit_user_role").await?;

      (can_manage_roles, can_edit_user_role)
    };

    if !can_manage_roles && !can_edit_user_role {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all roles
    RoleService::get_all_roles(pool).await
  }

  /// Get a role by ID with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Returns the role with the given ID
  pub async fn get_role_by_id_with_permission_check(
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

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let can_manage_roles = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
    };

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get role by ID
    let role = RoleService::get_role_by_id(pool, role_id).await?;
    match role {
      Some(role) => Ok(role),
      None => Err(RoleError::RoleNotFound(role_id)),
    }
  }

  /// Update a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Updates the role with the given ID
  pub async fn update_role_with_permission_check(
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

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let can_manage_roles = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
    };

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Update role
    RoleService::update_role(pool, role_id, update_data).await
  }

  /// Create a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Creates the role with is_default set to false
  pub async fn create_role_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    create_data: CreateRoleData,
  ) -> Result<Role, RoleError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(RoleError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let can_manage_roles = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
    };

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    let create_data_with_default_false = CreateRoleData {
      name: create_data.name,
      permissions: create_data.permissions,
      is_default: false,
    };
    RoleService::create_role(pool, create_data_with_default_false).await
  }

  /// Delete a role with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Deletes the role with the given ID
  pub async fn delete_role_with_permission_check(
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

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let can_manage_roles = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
    };

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Delete role
    RoleService::delete_role(pool, role_id).await
  }

  /// Set a role as default with permission check
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_manage_roles" permission
  /// - Sets the role with the given ID as default
  pub async fn set_default_role_with_permission_check(
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

    // Authorization: Get user and check permissions
    let user = {
      let mut conn = pool.acquire().await?;
      GetUserByIdQuery::run(&mut conn, user_id)
        .await?
        .ok_or(RoleError::AuthenticationError("User not found".to_string()))?
    };

    let can_manage_roles = {
      let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
    };

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Set default role
    RoleService::set_default_role(pool, role_id).await
  }

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
  pub async fn get_all_role_permissions_with_permission_check(
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
    let allowed = RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?;
    let can_manage_admin =
      RoleService::check_user_permission(&mut conn, &user, "can_manage_admin_role_permission")
        .await?;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

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

    let result =
      RoleOrchestrator::get_all_roles_with_permission_check(&pool, session_context).await;

    assert!(result.is_ok());
    let roles = result.unwrap();
    // Should return the admin role that was created
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "admin");
  }

  #[tokio::test]
  async fn test_get_role_by_id_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to retrieve
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_role_by_id_with_permission_check(&pool, session_context, test_role_id)
        .await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.id, test_role_id);
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_get_role_by_id_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to retrieve
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result =
      RoleOrchestrator::get_role_by_id_with_permission_check(&pool, session_context, test_role_id)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_get_role_by_id_with_permission_check_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to retrieve
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_role_by_id_with_permission_check(&pool, session_context, test_role_id)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_get_role_by_id_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a role to try to retrieve
    let test_role_id = create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_role_by_id_with_permission_check(&pool, session_context, test_role_id)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_get_role_by_id_with_permission_check_not_found() {
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

    let result =
      RoleOrchestrator::get_role_by_id_with_permission_check(&pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
    )
    .await;

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
    )
    .await;

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
    )
    .await;

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
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

    let result =
      RoleOrchestrator::update_role_with_permission_check(&pool, session_context, 999, update_data)
        .await;

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
    )
    .await;

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

    let result = RoleOrchestrator::update_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
      update_data,
    )
    .await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "user"); // unchanged
    let permissions = updated_role.permissions;
    assert_eq!(permissions.len(), 0); // should be empty now
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_admin_success() {
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

    // Create role data
    let create_data = CreateRoleData {
      name: "new_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "can_list_users".to_string(),
      ],
      is_default: false,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

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

  #[tokio::test]
  async fn test_create_role_with_permission_check_empty_permissions() {
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

    // Create role data with empty permissions
    let create_data = CreateRoleData {
      name: "empty_permissions_role".to_string(),
      permissions: vec![],
      is_default: true,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_ok());
    let role = result.unwrap();
    assert_eq!(role.name, "empty_permissions_role");
    assert!(!role.is_default); // Always false for now
    assert_eq!(role.permissions.len(), 0);
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

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

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_forbidden() {
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

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_validation_error() {
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

    // Create role data with empty name (validation error)
    let create_data = CreateRoleData {
      name: "".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => {
        assert_eq!(msg, "Role name cannot be empty");
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_invalid_permission() {
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

    // Create role data with invalid permission
    let create_data = CreateRoleData {
      name: "invalid_permission_role".to_string(),
      permissions: vec!["invalid_permission".to_string()],
      is_default: false,
    };

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => {
        assert_eq!(permission, "invalid_permission");
      }
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_duplicate_name() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create an existing role first
    create_test_role_with_pool(&pool, "existing_role", &["can_view_user_self"]).await;

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

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNameAlreadyExists(name) => {
        assert_eq!(name, "existing_role");
      }
      _ => panic!("Expected RoleNameAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_all_valid_permissions() {
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

    let result =
      RoleOrchestrator::create_role_with_permission_check(&pool, session_context, create_data)
        .await;

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

  #[tokio::test]
  async fn test_delete_role_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

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

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

    assert!(result.is_ok());
    let deleted_role = result.unwrap();
    assert_eq!(deleted_role.id, test_role_id);
    assert_eq!(deleted_role.name, "test_role");

    // Verify role is actually deleted
    let check_result = RoleService::get_role_by_id(&pool, test_role_id).await;
    assert!(check_result.is_ok());
    assert!(check_result.unwrap().is_none());
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to delete
    let test_role_id =
      create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

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
    let (pool, _temp_file) = create_test_database().await;

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

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

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
    let (pool, _temp_file) = create_test_database().await;

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

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

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

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, 999).await;

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
    let (pool, _temp_file) = create_test_database().await;

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

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

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
    let check_role = RoleService::get_role_by_id(&pool, test_role_id).await;
    assert!(check_role.is_ok());
    assert!(check_role.unwrap().is_some());
  }

  #[tokio::test]
  async fn test_delete_role_with_permission_check_returns_deleted_data() {
    let (pool, _temp_file) = create_test_database().await;

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
    let role_before = RoleService::get_role_by_id(&pool, test_role_id)
      .await
      .unwrap()
      .unwrap();

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::delete_role_with_permission_check(&pool, session_context, test_role_id)
        .await;

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

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to set as default
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
    )
    .await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert_eq!(updated_role.name, "user");
    assert!(updated_role.is_default);
    assert!(updated_role.has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to set as default
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to set as default
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
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

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a role to try to set as default
    let test_role_id = create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
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

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_not_found() {
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

    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
      999, // Non-existent role ID
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => {
        assert_eq!(id, 999);
      }
      _ => panic!("Expected RoleNotFound"),
    }
  }

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_atomic_behavior() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create multiple roles
    let role1_id = create_test_role_with_pool(&pool, "role1", &["can_view_user_self"]).await;
    let role2_id = create_test_role_with_pool(&pool, "role2", &["can_list_users"]).await;
    let role3_id = create_test_role_with_pool(&pool, "role3", &["can_manage_roles"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Set role1 as default
    let result1 = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context.clone(),
      role1_id,
    )
    .await;
    assert!(result1.is_ok());
    let updated_role1 = result1.unwrap();
    assert!(updated_role1.is_default);

    // Verify only role1 is default
    let all_roles = RoleService::get_all_roles(&pool).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role1_id);

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role2 as default (should unset role1)
    let result2 = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context.clone(),
      role2_id,
    )
    .await;
    assert!(result2.is_ok());
    let updated_role2 = result2.unwrap();
    assert!(updated_role2.is_default);

    // Verify only role2 is default now
    let all_roles = RoleService::get_all_roles(&pool).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role2_id);

    // Verify role1 is no longer default
    let current_role1 = RoleService::get_role_by_id(&pool, role1_id).await.unwrap();
    assert!(current_role1.is_some());
    assert!(!current_role1.unwrap().is_default);

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role3 as default (should unset role2)
    let result3 =
      RoleOrchestrator::set_default_role_with_permission_check(&pool, session_context, role3_id)
        .await;
    assert!(result3.is_ok());
    let updated_role3 = result3.unwrap();
    assert!(updated_role3.is_default);

    // Verify only role3 is default now
    let all_roles = RoleService::get_all_roles(&pool).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role3_id);

    // Verify role2 is no longer default
    let current_role2 = RoleService::get_role_by_id(&pool, role2_id).await.unwrap();
    assert!(current_role2.is_some());
    assert!(!current_role2.unwrap().is_default);
  }

  #[tokio::test]
  async fn test_set_default_role_with_permission_check_timestamp_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role to set as default
    let test_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Get the role before setting as default to compare timestamps
    let role_before = RoleService::get_role_by_id(&pool, test_role_id)
      .await
      .unwrap()
      .unwrap();

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
    let result = RoleOrchestrator::set_default_role_with_permission_check(
      &pool,
      session_context,
      test_role_id,
    )
    .await;

    assert!(result.is_ok());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, test_role_id);
    assert!(updated_role.is_default);
    assert!(updated_role.updated_ts > role_before.updated_ts);
    assert_eq!(updated_role.created_ts, role_before.created_ts);
  }

  #[tokio::test]
  async fn test_get_all_role_permissions_with_permission_check_without_manage_roles() {
    let (pool, _temp_file) = create_test_database().await;

    // Create regular user role without can_manage_roles
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_all_role_permissions_with_permission_check(&pool, session_context)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden: Insufficient permissions"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_get_all_role_permissions_with_permission_check_role_manager() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role manager with can_manage_roles but not can_manage_admin_role_permission
    let role_manager_id =
      create_test_role_with_pool(&pool, "role_manager", &["can_manage_roles"]).await;
    let role_manager_user =
      create_test_user_with_pool(&pool, "role_manager", role_manager_id).await;

    // Create session context for role manager
    let session_payload = DpsAuthSessionPayload {
      sub: role_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_all_role_permissions_with_permission_check(&pool, session_context)
        .await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(!permissions.contains(&"is_admin".to_string()));
    assert!(permissions.contains(&"can_manage_roles".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len() - 1);
  }

  #[tokio::test]
  async fn test_get_all_role_permissions_with_permission_check_admin_manager() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin manager with both permissions
    let admin_manager_id = create_test_role_with_pool(
      &pool,
      "admin_manager",
      &["can_manage_roles", "can_manage_admin_role_permission"],
    )
    .await;
    let admin_manager_user =
      create_test_user_with_pool(&pool, "admin_manager", admin_manager_id).await;

    // Create session context for admin manager
    let session_payload = DpsAuthSessionPayload {
      sub: admin_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_all_role_permissions_with_permission_check(&pool, session_context)
        .await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(permissions.contains(&"is_admin".to_string()));
    assert!(permissions.contains(&"can_manage_roles".to_string()));
    assert!(permissions.contains(&"can_manage_admin_role_permission".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len());
  }

  #[tokio::test]
  async fn test_get_all_role_permissions_with_permission_check_admin() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin user (with is_admin, bypasses can_manage_roles requirement)
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["is_admin"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create session context for admin
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      RoleOrchestrator::get_all_role_permissions_with_permission_check(&pool, session_context)
        .await;

    assert!(result.is_ok());
    let permissions = result.unwrap();
    assert!(permissions.contains(&"is_admin".to_string()));
    assert_eq!(permissions.len(), ROLE_PERMISSIONS.len());
  }

  #[tokio::test]
  async fn test_get_all_role_permissions_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let result =
      RoleOrchestrator::get_all_role_permissions_with_permission_check(&pool, session_context)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }
}
