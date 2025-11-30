use crate::middleware::session::SessionContext;
use crate::models::user::UserWithRole;
use crate::queries::users::GetAllUsersWithRolesQuery;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{UserError, UserRoleService};
use sqlx::SqlitePool;

pub struct UserOrchestrator;

impl UserOrchestrator {
  pub async fn list_users_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<UserWithRole>, UserError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_list_users")
      .await
      .map_err(UserError::DatabaseError)?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all users
    GetAllUsersWithRolesQuery::run(pool)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::models::User;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::PasswordService;
  use dps_auth_session::DpsAuthSessionPayload;
  use sqlx::SqlitePool;
  use uuid::Uuid;

  async fn create_test_user(pool: &SqlitePool, username: &str, role_id: i64) -> User {
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
      INSERT INTO user_roles (name, created_ts, permissions_json, is_default)
      VALUES (?, ?, ?, FALSE)
      "#,
    )
    .bind(name)
    .bind(1234567890i64)
    .bind(permissions_json)
    .execute(pool)
    .await
    .unwrap();

    result.last_insert_rowid()
  }

  #[tokio::test]
  async fn test_list_users_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create roles
    let admin_role_id = create_test_role(&pool, "admin", &["can_list_users"]).await;
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create users
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let regular_user = create_test_user(&pool, "user1", user_role_id).await;
    let _another_user = create_test_user(&pool, "user2", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test listing users
    let result = UserOrchestrator::list_users_with_permission_check(&pool, session_context).await;

    assert!(result.is_ok());
    let users = result.unwrap();
    assert_eq!(users.len(), 3); // All users should be returned

    // Verify admin user is in the list
    let admin_in_list = users.iter().any(|u| u.user.id == admin_user.id);
    assert!(admin_in_list);

    // Verify regular user is in the list
    let user_in_list = users.iter().any(|u| u.user.id == regular_user.id);
    assert!(user_in_list);
  }

  #[tokio::test]
  async fn test_list_users_without_authentication() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user ID (not authenticated)
    let session_context = SessionContext::new(None);

    // Test listing users
    let result = UserOrchestrator::list_users_with_permission_check(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_list_users_without_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role without can_list_users permission
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create regular user
    let regular_user = create_test_user(&pool, "user1", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test listing users
    let result = UserOrchestrator::list_users_with_permission_check(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_list_users_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test listing users
    let result = UserOrchestrator::list_users_with_permission_check(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_list_users_empty_database() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["can_list_users"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test listing users
    let result = UserOrchestrator::list_users_with_permission_check(&pool, session_context).await;

    assert!(result.is_ok());
    let users = result.unwrap();
    assert_eq!(users.len(), 1); // Only admin user should be returned
    assert_eq!(users[0].user.id, admin_user.id);
  }
}
