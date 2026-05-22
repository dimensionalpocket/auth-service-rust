use crate::types::PasswordError;
use crate::types::RoleError;

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

impl std::error::Error for UserError {}
