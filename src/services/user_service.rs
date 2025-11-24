use crate::models::User;
use crate::queries::users::{
  CreateUserData, CreateUserQuery, GetUserByIdQuery, GetUserByNameQuery, UpdateUserPasswordData,
  UpdateUserPasswordQuery,
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
      .ok_or_else(|| UserError::ValidationError("User not found".to_string()))?;

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
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::queries::users::GetUserByIdQuery;

  #[tokio::test]
  async fn test_create_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert default role first
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      "INSERT INTO user_roles (name, created_ts, is_default) VALUES ('user', 1234567890, TRUE)",
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
      UserError::ValidationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }
}
