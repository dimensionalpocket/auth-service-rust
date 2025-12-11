use crate::middleware::session::SessionContext;
use crate::models::role::Role;
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
    let user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;
    let can_edit_user_role =
      RoleService::check_user_permission(pool, &user, "can_edit_user_role").await?;

    if !can_manage_roles && !can_edit_user_role {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all roles
    RoleService::get_all_roles(pool).await
  }

  /// Get a role by ID with permission check
  ///
  /// Validates that the user has `can_manage_roles` permission
  /// before returning the requested role.
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
    let user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

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
  /// Validates that the user has `can_manage_roles` permission
  /// before updating the requested role.
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
    let user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Update role
    RoleService::update_role(pool, role_id, update_data).await
  }

  /// Create a role with permission check
  ///
  /// Validates that the user has `can_manage_roles` permission
  /// before creating the new role.
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
    let user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

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
  /// Validates that the user has `can_manage_roles` permission
  /// before deleting the requested role.
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
    let user = GetUserByIdQuery::run(pool, user_id)
      .await?
      .ok_or(RoleError::AuthenticationError("User not found".to_string()))?;

    let can_manage_roles =
      RoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Delete role
    RoleService::delete_role(pool, role_id).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::PasswordService;
  use dps_auth_session::DpsAuthSessionPayload;
  use sqlx::SqlitePool;
  use uuid::Uuid;

  async fn create_test_user(
    pool: &SqlitePool,
    username: &str,
    role_id: i64,
  ) -> crate::models::User {
    let password_hash = PasswordService::generate("password123").unwrap();
    let create_data = CreateUserData {
      uuid: Uuid::new_v4().to_string(),
      name: username.to_string(),
      role_id: Some(role_id),
      password_hash,
      metadata_json: None,
    };
    CreateUserQuery::run(pool, create_data).await.unwrap()
  }

  async fn create_test_role(pool: &SqlitePool, name: &str, permissions: &[&str]) -> i64 {
    let permissions_json = serde_json::json!(permissions);
    let result = sqlx::query(
      r#"
      INSERT INTO roles (name, created_ts, updated_ts, permissions_json, is_default)
      VALUES (?, ?, ?, ?, FALSE)
      "#,
    )
    .bind(name)
    .bind(1234567890i64)
    .bind(1234567890i64)
    .bind(permissions_json)
    .execute(pool)
    .await
    .unwrap();

    result.last_insert_rowid()
  }

  #[tokio::test]
  async fn test_get_all_roles_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create some roles to retrieve
    create_test_role(&pool, "user", &["can_view_user_self"]).await;
    create_test_role(&pool, "editor", &["can_edit_content"]).await;

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
    let role_editor_id = create_test_role(&pool, "role_editor", &["can_edit_user_role"]).await;
    let role_editor_user = create_test_user(&pool, "role_editor", role_editor_id).await;

    // Create some roles to retrieve
    create_test_role(&pool, "user", &["can_view_user_self"]).await;
    create_test_role(&pool, "editor", &["can_edit_content"]).await;

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
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to retrieve
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

    // Create a role to try to retrieve
    let test_role_id = create_test_role(&pool, "editor", &["can_edit_content"]).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_edit_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_partial_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id =
      create_test_role(&pool, "editor", &["can_edit_content", "can_view_content"]).await;

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
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2); // unchanged
    assert!(permissions.contains(&"can_edit_content".to_string()));
    assert!(permissions.contains(&"can_view_content".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role to try to update
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

    // Create a role to try to update
    let test_role_id = create_test_role(&pool, "editor", &["can_edit_content"]).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to update
    let test_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

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
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 0); // should be empty now
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[tokio::test]
  async fn test_create_role_with_permission_check_empty_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    assert_eq!(role.permissions().len(), 0);
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
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create an existing role first
    create_test_role(&pool, "existing_role", &["can_view_user_self"]).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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

    let permissions = role.permissions();
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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to delete
    let test_role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;

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
    let test_role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;

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
    let test_role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;

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
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

    // Create a role to try to delete
    let test_role_id = create_test_role(&pool, "test_role", &["can_edit_content"]).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role to delete
    let test_role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;

    // Create a user with the role to be deleted
    let user_with_role = create_test_user(&pool, "user_with_role", test_role_id).await;

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
    let check_user = GetUserByIdQuery::run(&pool, user_with_role.id).await;
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
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create a role with specific data to delete
    let test_role_id = create_test_role(
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

    let permissions_before = role_before.permissions();
    let permissions_deleted = deleted_role.permissions();
    assert_eq!(permissions_before.len(), permissions_deleted.len());
    for permission in permissions_before {
      assert!(permissions_deleted.contains(&permission));
    }
  }
}
