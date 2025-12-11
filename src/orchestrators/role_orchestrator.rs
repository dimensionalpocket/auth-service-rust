use crate::middleware::session::SessionContext;
use crate::models::user_role::UserRole;
use crate::queries::users::GetUserByIdQuery;
use crate::services::user_role_service::{RoleError, UserRoleService};
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
  ) -> Result<Vec<UserRole>, RoleError> {
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
      UserRoleService::check_user_permission(pool, &user, "can_manage_roles").await?;
    let can_edit_user_role =
      UserRoleService::check_user_permission(pool, &user, "can_edit_user_role").await?;

    if !can_manage_roles && !can_edit_user_role {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all roles
    UserRoleService::get_all_roles(pool).await
  }

  /// Get a role by ID with permission check
  ///
  /// Validates that the user has `can_manage_roles` permission
  /// before returning the requested role.
  pub async fn get_role_by_id_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    role_id: i64,
  ) -> Result<UserRole, RoleError> {
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
      UserRoleService::check_user_permission(pool, &user, "can_manage_roles").await?;

    if !can_manage_roles {
      return Err(RoleError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get role by ID
    let role = UserRoleService::get_role_by_id(pool, role_id).await?;
    match role {
      Some(role) => Ok(role),
      None => Err(RoleError::RoleNotFound(role_id)),
    }
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
      INSERT INTO user_roles (name, created_ts, updated_ts, permissions_json, is_default)
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
}
