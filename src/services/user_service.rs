use crate::models::User;
use crate::queries::users::{
  CreateUserData, CreateUserQuery, DeleteUserByIdQuery, GetUserByIdQuery, GetUserByNameQuery,
  UpdateUserData, UpdateUserPasswordData, UpdateUserPasswordQuery, UpdateUserQuery,
};
use crate::services::{PasswordError, PasswordService};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Custom error type for user operations
#[derive(Debug)]
pub enum UserError {
  /// Username is already in use
  UsernameAlreadyExists(String),
  /// Password hashing failed
  PasswordHashingFailed(PasswordError),
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Input validation failed
  ValidationError(String),
  /// Authentication failed
  AuthenticationError(String),
  /// Authorization failed
  AuthorizationError(String),
  /// User not found
  UserNotFound(i64),
  /// Self-deletion attempted
  SelfDeletion,
  /// Session creation failed
  SessionError(String),
}

impl std::fmt::Display for UserError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserError::UsernameAlreadyExists(username) => {
        write!(f, "Username '{username}' is already in use")
      }
      UserError::PasswordHashingFailed(err) => write!(f, "Password hashing failed: {err}"),
      UserError::DatabaseError(err) => write!(f, "Database error: {err}"),
      UserError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      UserError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      UserError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
      UserError::UserNotFound(user_id) => write!(f, "User with ID {user_id} not found"),
      UserError::SelfDeletion => write!(f, "Cannot delete your own account"),
      UserError::SessionError(msg) => write!(f, "Session creation failed: {msg}"),
    }
  }
}

impl std::error::Error for UserError {}

impl From<PasswordError> for UserError {
  fn from(err: PasswordError) -> Self {
    UserError::PasswordHashingFailed(err)
  }
}

impl From<sqlx::Error> for UserError {
  fn from(err: sqlx::Error) -> Self {
    UserError::DatabaseError(err)
  }
}

/// Service for user management operations
pub struct UserService;

impl UserService {
  /// Create a new user with validation and default role assignment
  ///
  /// This method validates the input (username and password), checks for username
  /// uniqueness, hashes the password using PasswordService, and assigns the default
  /// role to the user.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `username` - The username for the new user (must be unique)
  /// * `password` - The plain text password (will be hashed)
  ///
  /// # Returns
  /// * `Ok(User)` - Successfully created user
  /// * `Err(UserError)` - Creation failed due to validation, uniqueness, or database error
  pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
  ) -> Result<User, UserError> {
    // Input validation
    Self::validate_username(username)?;
    Self::validate_password(password)?;

    // Check if username already exists
    if let Some(_existing_user) = GetUserByNameQuery::run(pool, username).await? {
      return Err(UserError::UsernameAlreadyExists(username.to_string()));
    }

    // Hash the password
    let password_hash = PasswordService::generate(password)?;

    // Generate UUID for the user
    let user_uuid = Uuid::new_v4().to_string();

    // Create user data
    let create_data = CreateUserData {
      uuid: user_uuid,
      name: username.to_string(),
      role_id: None, // Use default role
      password_hash,
      metadata_json: None,
    };

    // Create the user
    let user = CreateUserQuery::run(pool, create_data).await?;

    Ok(user)
  }

  /// Retrieve a user by name (case-insensitive)
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `name` - The name of the user to retrieve
  ///
  /// # Returns
  /// * `Ok(Some(User))` - User found
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn get_user_by_name(
    pool: &SqlitePool,
    name: &str,
  ) -> Result<Option<User>, sqlx::Error> {
    GetUserByNameQuery::run(pool, name).await
  }

  /// Retrieve a user by ID
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to retrieve
  ///
  /// # Returns
  /// * `Ok(Some(User))` - User found
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn get_user_by_id(
    pool: &SqlitePool,
    user_id: i64,
  ) -> Result<Option<User>, sqlx::Error> {
    GetUserByIdQuery::run(pool, user_id).await
  }

  /// Validate username according to business rules
  fn validate_username(username: &str) -> Result<(), UserError> {
    if username.trim().is_empty() {
      return Err(UserError::ValidationError(
        "Username cannot be empty".to_string(),
      ));
    }

    if username.len() < 3 {
      return Err(UserError::ValidationError(
        "Username must be at least 3 characters long".to_string(),
      ));
    }

    if username.len() > 20 {
      return Err(UserError::ValidationError(
        "Username cannot be longer than 20 characters".to_string(),
      ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !username
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
      return Err(UserError::ValidationError(
        "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
      ));
    }

    Ok(())
  }

  /// Update a user's password with validation and verification
  ///
  /// This method validates the new password, verifies the current password,
  /// and updates the user's password in the database.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to update
  /// * `current_password` - The user's current password for verification
  /// * `new_password` - The new password to set
  /// * `new_password_confirmation` - Confirmation of the new password
  ///
  /// # Returns
  /// * `Ok(User)` - Successfully updated user
  /// * `Err(UserError)` - Update failed due to validation, verification, or database error
  pub async fn update_password(
    pool: &SqlitePool,
    user_id: i64,
    current_password: &str,
    new_password: &str,
    new_password_confirmation: &str,
  ) -> Result<User, UserError> {
    // Validate new password
    Self::validate_password(new_password)?;

    // Validate password confirmation matches
    if new_password != new_password_confirmation {
      return Err(UserError::ValidationError(
        "Passwords do not match".to_string(),
      ));
    }

    // Get current user to verify current password
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    // Verify current password
    let is_current_password_valid = PasswordService::verify(current_password, &user.password_hash)
      .map_err(UserError::PasswordHashingFailed)?;

    if !is_current_password_valid {
      return Err(UserError::ValidationError(
        "Current password is incorrect".to_string(),
      ));
    }

    // Hash the new password
    let new_password_hash = PasswordService::generate(new_password)?;

    // Update the password in database
    let update_data = UpdateUserPasswordData {
      password_hash: new_password_hash,
    };

    let updated_user = UpdateUserPasswordQuery::run(pool, user_id, update_data)
      .await
      .map_err(UserError::DatabaseError)?;

    Ok(updated_user)
  }

  /// Delete a user by ID
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to delete
  ///
  /// # Returns
  /// * `Ok(bool)` - True if user was deleted, false if user was not found
  /// * `Err(UserError)` - Database error occurred
  pub async fn delete_user(pool: &SqlitePool, user_id: i64) -> Result<bool, UserError> {
    DeleteUserByIdQuery::run(pool, user_id)
      .await
      .map_err(UserError::DatabaseError)
  }

  /// Validate password according to business rules
  fn validate_password(password: &str) -> Result<(), UserError> {
    if password.is_empty() {
      return Err(UserError::ValidationError(
        "Password cannot be empty".to_string(),
      ));
    }

    if password.len() < 6 {
      return Err(UserError::ValidationError(
        "Password must be at least 6 characters long".to_string(),
      ));
    }

    if password.len() > 128 {
      return Err(UserError::ValidationError(
        "Password cannot be longer than 128 characters".to_string(),
      ));
    }

    Ok(())
  }

  /// Update a user's details with validation and partial update support
  ///
  /// This method validates input, checks for username uniqueness if name is changing,
  /// hashes password if provided, and updates only specified fields.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of user to update
  /// * `update_data` - Partial user data to update (with Option<Option<T>> for nullable fields)
  /// * `password` - Optional new password (plain text, will be hashed)
  ///
  /// # Returns
  /// * `Ok(User)` - Successfully updated user
  /// * `Err(UserError)` - Validation, uniqueness, or database error
  pub async fn update_user(
    pool: &SqlitePool,
    user_id: i64,
    mut update_data: UpdateUserData,
    password: Option<String>,
  ) -> Result<User, UserError> {
    // Get current user for validation and comparison
    let current_user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    // Validate username if provided
    if let Some(ref name) = update_data.name {
      Self::validate_username(name)?;

      // Check username uniqueness if name is changing
      if name != &current_user.name {
        if let Some(_existing_user) = GetUserByNameQuery::run(pool, name).await? {
          return Err(UserError::UsernameAlreadyExists(name.to_string()));
        }
      }
    }

    // Hash password if provided
    if let Some(password) = password {
      Self::validate_password(&password)?;
      let password_hash = PasswordService::generate(&password)?;
      update_data.password_hash = Some(password_hash);
    }

    // Update user in database
    UpdateUserQuery::run(pool, update_data)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::users::{CreateUserData, CreateUserQuery, GetUserByIdQuery, UpdateUserData};
  use crate::test_utils::{create_test_database, create_test_database_with_pool_size};

  #[tokio::test]
  async fn test_create_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    assert_eq!(user.name, "testuser");
    assert!(user.id > 0);
    assert!(!user.uuid.is_empty());
    assert!(!user.password_hash.is_empty());
    assert!(user.password_hash.starts_with("$argon2"));
    assert!(user.created_ts > 0);
    assert_eq!(user.created_ts, user.updated_ts);
  }

  #[tokio::test]
  async fn test_create_user_username_already_exists() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create first user
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Try to create second user with same username
    let result = UserService::create_user(&pool, "testuser", "password456").await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => assert_eq!(username, "testuser"),
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_create_user_case_insensitive_username_check() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create first user with lowercase
    UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Try to create second user with different case
    let result = UserService::create_user(&pool, "TestUser", "password456").await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => assert_eq!(username, "TestUser"),
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_validate_username_empty() {
    let result = UserService::validate_username("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_username_whitespace_only() {
    let result = UserService::validate_username("   ");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_username_too_short() {
    let result = UserService::validate_username("ab");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 3 characters"));
  }

  #[tokio::test]
  async fn test_validate_username_too_long() {
    let result = UserService::validate_username("a".repeat(21).as_str());
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 20 characters"));
  }

  #[tokio::test]
  async fn test_validate_username_invalid_characters() {
    let result = UserService::validate_username("test@user");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("can only contain"));
  }

  #[tokio::test]
  async fn test_validate_username_valid_characters() {
    assert!(UserService::validate_username("test_user-123").is_ok());
    assert!(UserService::validate_username("TestUser").is_ok());
    assert!(UserService::validate_username("user123").is_ok());
    assert!(UserService::validate_username("test-user").is_ok());
    assert!(UserService::validate_username("test_user").is_ok());
  }

  #[tokio::test]
  async fn test_validate_password_empty() {
    let result = UserService::validate_password("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_password_too_short() {
    let result = UserService::validate_password("12345");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 6 characters"));
  }

  #[tokio::test]
  async fn test_validate_password_too_long() {
    let result = UserService::validate_password(&"a".repeat(129));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 128 characters"));
  }

  #[tokio::test]
  async fn test_validate_password_valid() {
    assert!(UserService::validate_password("123456").is_ok());
    assert!(UserService::validate_password("password123").is_ok());
    assert!(UserService::validate_password(&"a".repeat(128)).is_ok());
  }

  #[tokio::test]
  async fn test_get_user_by_id_query_works_correctly() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create a user first
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    let retrieved_user = GetUserByIdQuery::run(&pool, user.id).await.unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, user.name);
  }

  #[tokio::test]
  async fn test_get_user_by_id_query_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let user = GetUserByIdQuery::run(&pool, 999).await.unwrap();

    assert!(user.is_none());
  }

  #[tokio::test]
  async fn test_get_user_by_name_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create a user first
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Retrieve by name
    let retrieved_user = UserService::get_user_by_name(&pool, "testuser")
      .await
      .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, user.name);
  }

  #[tokio::test]
  async fn test_get_user_by_name_case_insensitive() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create a user first
    let user = UserService::create_user(&pool, "TestUser", "password123")
      .await
      .unwrap();

    // Retrieve by different case
    let retrieved_user = UserService::get_user_by_name(&pool, "testuser")
      .await
      .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.name, "TestUser"); // Original case preserved
  }

  #[tokio::test]
  async fn test_get_user_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let user = UserService::get_user_by_name(&pool, "nonexistent")
      .await
      .unwrap();

    assert!(user.is_none());
  }

  #[tokio::test]
  async fn test_update_password_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create default role and user
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user = UserService::create_user(&pool, "testuser", "oldpassword123")
      .await
      .unwrap();

    // Test: Update password
    let updated_user = UserService::update_password(
      &pool,
      user.id,
      "oldpassword123",
      "newpassword456",
      "newpassword456",
    )
    .await
    .unwrap();

    // Verify: Password hash changed and update was successful
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_ne!(updated_user.password_hash, user.password_hash);
    // Note: timestamp might be the same in fast test environments, so we just verify the update succeeded

    // Verify: New password works for authentication
    let is_new_password_valid =
      PasswordService::verify("newpassword456", &updated_user.password_hash).unwrap();
    assert!(is_new_password_valid);

    // Verify: Old password no longer works
    let is_old_password_valid =
      PasswordService::verify("oldpassword123", &updated_user.password_hash).unwrap();
    assert!(!is_old_password_valid);
  }

  #[tokio::test]
  async fn test_update_password_invalid_current_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create default role and user
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user = UserService::create_user(&pool, "testuser", "correctpassword")
      .await
      .unwrap();

    // Test: Try to update with wrong current password
    let result = UserService::update_password(
      &pool,
      user.id,
      "wrongpassword",
      "newpassword456",
      "newpassword456",
    )
    .await;

    // Verify: Should return error
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Current password is incorrect"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_password_password_confirmation_mismatch() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create default role and user
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user = UserService::create_user(&pool, "testuser", "currentpassword")
      .await
      .unwrap();

    // Test: Try to update with mismatched password confirmation
    let result = UserService::update_password(
      &pool,
      user.id,
      "currentpassword",
      "newpassword456",
      "differentpassword",
    )
    .await;

    // Verify: Should return error
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Passwords do not match"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_password_invalid_new_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create default role and user
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user = UserService::create_user(&pool, "testuser", "currentpassword")
      .await
      .unwrap();

    // Test: Try to update with invalid new password (too short)
    let result =
      UserService::update_password(&pool, user.id, "currentpassword", "123", "123").await;

    // Verify: Should return error
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("at least 6 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_password_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Test: Try to update password for non-existent user
    let result = UserService::update_password(
      &pool,
      999,
      "anypassword",
      "newpassword456",
      "newpassword456",
    )
    .await;

    // Verify: Should return error
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[tokio::test]
  async fn test_delete_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create default role and user
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user = UserService::create_user(&pool, "testuser", "password123")
      .await
      .unwrap();

    // Verify user exists before deletion
    let user_before = GetUserByIdQuery::run(&pool, user.id).await.unwrap();
    assert!(user_before.is_some());

    // Test: Delete user
    let deleted = UserService::delete_user(&pool, user.id).await.unwrap();
    assert!(deleted);

    // Verify user is deleted
    let user_after = GetUserByIdQuery::run(&pool, user.id).await.unwrap();
    assert!(user_after.is_none());
  }

  #[tokio::test]
  async fn test_delete_user_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Test: Try to delete non-existent user
    let deleted = UserService::delete_user(&pool, 999).await.unwrap();
    assert!(!deleted);
  }

  #[tokio::test]
  async fn test_delete_user_database_error() {
    let (pool, _temp_file) = create_test_database().await;

    // Close the pool to simulate database error
    pool.close().await;

    // Test: Try to delete user with closed database
    let result = UserService::delete_user(&pool, 1).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::DatabaseError(_) => {
        // Expected error type
      }
      _ => panic!("Expected DatabaseError"),
    }
  }

  // Tests for update_user method

  #[tokio::test]
  async fn test_update_user_success_name_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only name
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("newname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "newname");
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_success_role_only() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create a user and admin role
    setup_default_role(&pool).await;
    let admin_role_id = create_admin_role(&pool).await;

    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only role
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: Some(admin_role_id),
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_success_password_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only password
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None, // Will be set by service
      metadata_json: None,
    };

    let result = UserService::update_user(
      &pool,
      user.id,
      update_data,
      Some("newpassword123".to_string()),
    )
    .await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_ne!(updated_user.password_hash, user.password_hash); // Password should be different
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_success_metadata_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only metadata
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(Some(r#"{"key": "value"}"#.to_string())),
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"key": "value"}"#.to_string())
    );
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_success_metadata_to_null() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create a user with metadata
    setup_default_role(&pool).await;
    let admin_role_id = create_admin_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: Some(admin_role_id),
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: Some(r#"{"old": "data"}"#.to_string()),
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Set metadata to NULL
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(None), // Explicitly set to NULL
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, None); // Should be NULL
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_success_multiple_fields() {
    let (pool, _temp_file) = create_test_database_with_pool_size(1).await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let admin_role_id = create_admin_role(&pool).await;

    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update multiple fields
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("newname".to_string()),
      role_id: Some(admin_role_id),
      password_hash: None, // Will be set by service
      metadata_json: Some(Some(r#"{"updated": true}"#.to_string())),
    };

    let result = UserService::update_user(
      &pool,
      user.id,
      update_data,
      Some("newpassword123".to_string()),
    )
    .await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "newname");
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_ne!(updated_user.password_hash, user.password_hash); // Password should be different
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"updated": true}"#.to_string())
    );
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_no_updates() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: No updates provided
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    // Should be identical to original user (except possibly updated_ts)
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
  }

  #[tokio::test]
  async fn test_update_user_user_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Test: Try to update non-existent user
    let update_data = UpdateUserData {
      id: 999,
      name: Some("newname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, 999, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound error"),
    }
  }

  #[tokio::test]
  async fn test_update_user_username_validation_empty() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with empty username
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username cannot be empty"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_username_validation_too_short() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with too short username
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("ab".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username must be at least 3 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_username_validation_too_long() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with too long username
    let long_name = "a".repeat(21);
    let update_data = UpdateUserData {
      id: user.id,
      name: Some(long_name.clone()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username cannot be longer than 20 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_username_validation_invalid_chars() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with invalid characters
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("test@user".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(
          msg.contains("Username can only contain letters, numbers, underscores, and hyphens")
        );
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_username_already_exists() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create two users
    setup_default_role(&pool).await;

    let create_data1 = CreateUserData {
      uuid: "test-uuid-1".to_string(),
      name: "testuser1".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user1 = CreateUserQuery::run(&pool, create_data1).await.unwrap();

    let create_data2 = CreateUserData {
      uuid: "test-uuid-2".to_string(),
      name: "testuser2".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let _user2 = CreateUserQuery::run(&pool, create_data2).await.unwrap();

    // Test: Try to update user1 with user2's username
    let update_data = UpdateUserData {
      id: user1.id,
      name: Some("testuser2".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user1.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => {
        assert_eq!(username, "testuser2");
      }
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_update_user_password_validation_too_short() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with too short password
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result =
      UserService::update_user(&pool, user.id, update_data, Some("123".to_string())).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password must be at least 6 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_password_validation_too_long() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Try to update with too long password
    let long_password = "a".repeat(129);
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, Some(long_password)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password cannot be longer than 128 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_update_user_same_username_no_conflict() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update user with same username (should not conflict)
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("testuser".to_string()), // Same name
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UserService::update_user(&pool, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "testuser");
  }

  // Helper functions for tests
  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  async fn create_admin_role(pool: &SqlitePool) -> i64 {
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)",
    )
    .execute(pool)
    .await
    .unwrap();

    // Get the admin role ID
    let role_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
      .fetch_one(pool)
      .await
      .unwrap();
    role_id
  }
}
