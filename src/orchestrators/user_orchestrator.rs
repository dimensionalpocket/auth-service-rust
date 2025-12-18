use crate::middleware::session::SessionContext;
use crate::models::user::{User, UserWithRole};
use crate::queries::users::GetAllUsersWithRolesQuery;
use crate::queries::users::GetUserByIdQuery;
use crate::queries::users::GetUserByIdWithRoleQuery;
use crate::queries::users::UpdateUserData;
use crate::services::role_service::RoleError;
use crate::services::{RoleService, UserError, UserService};
use sqlx::SqlitePool;

impl From<RoleError> for UserError {
  fn from(err: RoleError) -> Self {
    match err {
      RoleError::DatabaseError(db_err) => UserError::DatabaseError(db_err),
      RoleError::AuthenticationError(msg) => UserError::AuthenticationError(msg),
      RoleError::AuthorizationError(msg) => UserError::AuthorizationError(msg),
      // Other RoleError variants shouldn't occur in user operations,
      // but we'll handle them as database errors for safety
      _ => UserError::DatabaseError(sqlx::Error::Protocol(format!("Role error: {err}"))),
    }
  }
}

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

    let allowed = {
      let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?
    };

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get all users
    let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

    GetAllUsersWithRolesQuery::run(&mut conn)
      .await
      .map_err(UserError::DatabaseError)
  }

  /// Get a single user's details with permission check
  ///
  /// This method:
  /// - Checks if the user is authenticated
  /// - Verifies the user has "can_view_user_details" permission
  /// - Returns the requested user's details with role information
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context for authentication/authorization
  /// * `target_user_id` - ID of the user to retrieve
  ///
  /// # Returns
  /// * `Ok(UserWithRole)` - User details with role information
  /// * `Err(UserError)` - Authentication, authorization, or database error
  pub async fn get_user_details_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    target_user_id: i64,
  ) -> Result<UserWithRole, UserError> {
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

    let allowed = {
      let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_view_user_details").await?
    };

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get the target user with role information
    GetUserByIdWithRoleQuery::run(pool, target_user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(target_user_id))
  }

  /// Delete a user with permission check and self-deletion prevention
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_delete_user" permission
  /// - Prevents users from deleting themselves
  /// - Deletes the target user
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context for authentication/authorization
  /// * `target_user_id` - ID of user to delete
  ///
  /// # Returns
  /// * `Ok(())` - User was successfully deleted
  /// * `Err(UserError)` - Authentication, authorization, validation, or database error
  pub async fn delete_user_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    target_user_id: i64,
  ) -> Result<(), UserError> {
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

    let allowed = {
      let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_delete_user").await?
    };

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Prevent self-deletion
    if user_id == target_user_id {
      return Err(UserError::SelfDeletion);
    }

    // Business logic: Delete user
    let deleted = UserService::delete_user(pool, target_user_id).await?;

    if !deleted {
      return Err(UserError::UserNotFound(target_user_id));
    }

    Ok(())
  }

  /// Update a user with permission check and validation
  ///
  /// This method:
  /// - Checks if user is authenticated
  /// - Verifies user has "can_edit_user" permission
  /// - Validates username uniqueness if name is being updated
  /// - Validates password if provided
  /// - Updates only the provided fields (partial update)
  /// - Returns the updated user
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `session_context` - Session context for authentication/authorization
  /// * `target_user_id` - ID of user to update
  /// * `update_data` - Partial user data to update (with Option<Option<T>> for nullable fields)
  /// * `password` - Optional new password (plain text, will be hashed)
  ///
  /// # Returns
  /// * `Ok(User)` - Successfully updated user
  /// * `Err(UserError)` - Authentication, authorization, validation, or database error
  pub async fn update_user_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    target_user_id: i64,
    update_data: UpdateUserData,
    password: Option<String>,
  ) -> Result<User, UserError> {
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

    let allowed = {
      let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

      RoleService::check_user_permission(&mut conn, &user, "can_edit_user").await?
    };

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Validate and update user
    UserService::update_user(pool, target_user_id, update_data, password).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::UpdateUserData;
  use crate::test_utils::{
    create_test_database, create_test_database_with_pool_size, create_test_role, create_test_user,
  };
  use dps_auth_session::DpsAuthSessionPayload;

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

  #[tokio::test]
  async fn test_get_user_details_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create roles
    let admin_role_id = create_test_role(&pool, "admin", &["can_view_user_details"]).await;
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create users
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let target_user = create_test_user(&pool, "target_user", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test getting user details
    let result = UserOrchestrator::get_user_details_with_permission_check(
      &pool,
      session_context,
      target_user.id,
    )
    .await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, target_user.id);
    assert_eq!(user_details.user.name, "target_user");
    assert_eq!(user_details.role.name, "user");
    assert_eq!(user_details.user.role_id, user_role_id);
  }

  #[tokio::test]
  async fn test_get_user_details_without_authentication() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user ID (not authenticated)
    let session_context = SessionContext::new(None);

    // Test getting user details
    let result =
      UserOrchestrator::get_user_details_with_permission_check(&pool, session_context, 123).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_get_user_details_without_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role without can_view_user_details permission
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

    // Test getting user details
    let result = UserOrchestrator::get_user_details_with_permission_check(
      &pool,
      session_context,
      regular_user.id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_get_user_details_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test getting non-existent user details
    let result =
      UserOrchestrator::get_user_details_with_permission_check(&pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_get_user_details_nonexistent_session_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create target user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let target_user = create_test_user(&pool, "target_user", user_role_id).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test getting user details
    let result = UserOrchestrator::get_user_details_with_permission_check(
      &pool,
      session_context,
      target_user.id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_get_user_details_self_access() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test getting own user details
    let result = UserOrchestrator::get_user_details_with_permission_check(
      &pool,
      session_context,
      admin_user.id,
    )
    .await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, admin_user.id);
    assert_eq!(user_details.user.name, "admin");
    assert_eq!(user_details.role.name, "admin");
  }

  #[tokio::test]
  async fn test_delete_user_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create roles
    let admin_role_id = create_test_role(&pool, "admin", &["can_delete_user"]).await;
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create users
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let target_user = create_test_user(&pool, "target_user", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test deleting user
    let result =
      UserOrchestrator::delete_user_with_permission_check(&pool, session_context, target_user.id)
        .await;

    assert!(result.is_ok());

    // Verify user is deleted
    let deleted_user = GetUserByIdQuery::run(&pool, target_user.id).await.unwrap();
    assert!(deleted_user.is_none());
  }

  #[tokio::test]
  async fn test_delete_user_without_authentication() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user ID (not authenticated)
    let session_context = SessionContext::new(None);

    // Test deleting user
    let result =
      UserOrchestrator::delete_user_with_permission_check(&pool, session_context, 123).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_delete_user_without_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role without can_delete_user permission
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create regular user
    let regular_user = create_test_user(&pool, "user1", user_role_id).await;
    let target_user = create_test_user(&pool, "target_user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test deleting user
    let result =
      UserOrchestrator::delete_user_with_permission_check(&pool, session_context, target_user.id)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_delete_user_self_deletion_prevented() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test trying to delete self
    let result = UserOrchestrator::delete_user_with_permission_check(
      &pool,
      session_context,
      admin_user.id, // Same as session user ID
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::SelfDeletion => {
        // Expected error
      }
      _ => panic!("Expected SelfDeletion error"),
    }

    // Verify user still exists
    let user_still_exists = GetUserByIdQuery::run(&pool, admin_user.id).await.unwrap();
    assert!(user_still_exists.is_some());
  }

  #[tokio::test]
  async fn test_delete_user_nonexistent_target() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role(&pool, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test deleting non-existent user
    let result = UserOrchestrator::delete_user_with_permission_check(
      &pool,
      session_context,
      999, // Non-existent user ID
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_delete_user_nonexistent_session_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create target user
    let user_role_id = create_test_role(&pool, "user", &[]).await;
    let target_user = create_test_user(&pool, "target_user", user_role_id).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999, // Non-existent user ID
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test deleting user
    let result =
      UserOrchestrator::delete_user_with_permission_check(&pool, session_context, target_user.id)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999); // Session user ID should be reported
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  // Tests for update_user_with_permission_check method

  #[tokio::test]
  async fn test_update_user_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create admin role with can_edit_user permission
    let admin_role_id = create_admin_role(&pool).await;
    eprintln!("Created admin role with ID: {admin_role_id}");

    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let target_user = create_test_user(&pool, "targetuser", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Update target user
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: Some(admin_role_id),
      password_hash: None,
      metadata_json: Some(Some(r#"{"updated": true}"#.to_string())),
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      Some("newpassword123".to_string()),
    )
    .await;

    assert!(result.is_ok());
    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "updatedname");
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"updated": true}"#.to_string())
    );
    assert_ne!(updated_user.password_hash, target_user.password_hash); // Password should be updated
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_unauthenticated() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create target user
    let user_role_id = create_user_role(&pool).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;

    // Create session context without user (unauthenticated)
    let session_context = SessionContext::new(None);

    // Test: Try to update user without authentication
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert_eq!(msg, "Authentication required");
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_session_user_not_found() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create target user
    let user_role_id = create_user_role(&pool).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;

    // Create session context with non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999, // Non-existent user ID
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update user with non-existent session user
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999); // Session user ID should be reported
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create regular user without can_edit_user permission
    let user_role_id = create_user_role(&pool).await;

    let regular_user = create_test_user(&pool, "regular", user_role_id).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update user without permission
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert_eq!(msg, "Forbidden");
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_target_user_not_found() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create admin user with can_edit_user permission
    let admin_role_id = create_admin_role(&pool).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update non-existent user
    let update_data = UpdateUserData {
      id: 999, // Non-existent target user
      name: Some("updatedname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      999,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999); // Target user ID should be reported
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_validation_error() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create admin user with can_edit_user permission
    let admin_role_id = create_admin_role(&pool).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    let user_role_id = create_user_role(&pool).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update user with invalid username (too short)
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("ab".to_string()), // Invalid: too short
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username must be at least 3 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_password_validation_error() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create admin user with can_edit_user permission
    let admin_role_id = create_admin_role(&pool).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    let user_role_id = create_user_role(&pool).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update user with invalid password (too short)
    let update_data = UpdateUserData {
      id: target_user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      Some("123".to_string()), // Invalid: too short
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password must be at least 6 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_with_permission_check_username_conflict() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create admin user with can_edit_user permission
    let admin_role_id = create_admin_role(&pool).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    let user_role_id = create_user_role(&pool).await;
    let target_user = create_test_user(&pool, "targetuser", user_role_id).await;
    let _existing_user = create_test_user(&pool, "existinguser", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test: Try to update user with existing username
    let update_data = UpdateUserData {
      id: target_user.id,
      name: Some("existinguser".to_string()), // Already exists
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserOrchestrator::update_user_with_permission_check(
      &pool,
      session_context,
      target_user.id,
      update_data,
      None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => {
        assert_eq!(username, "existinguser");
      }
      _ => panic!("Expected UsernameAlreadyExists"),
    }
  }

  // Helper functions for orchestrator tests
  async fn create_admin_role(pool: &SqlitePool) -> i64 {
    create_test_role(pool, "admin", &["can_edit_user"]).await
  }

  async fn create_user_role(pool: &SqlitePool) -> i64 {
    create_test_role(pool, "user", &[]).await
  }
}
