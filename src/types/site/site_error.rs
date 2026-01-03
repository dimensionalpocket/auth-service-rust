use crate::types::RoleError;
use std::fmt;

/// Custom error type for site operations
#[derive(Debug)]
pub enum SiteError {
  /// Slug is already in use
  SlugAlreadyExists(String),
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Input validation failed
  ValidationError(String),
  /// Site not found
  SiteNotFound(i64),
  /// Authentication failed
  AuthenticationError(String),
  /// Authorization failed
  AuthorizationError(String),
}

impl fmt::Display for SiteError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SiteError::SlugAlreadyExists(slug) => {
        write!(f, "Slug '{slug}' is already in use")
      }
      SiteError::DatabaseError(err) => write!(f, "Database error: {err}"),
      SiteError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      SiteError::SiteNotFound(id) => write!(f, "Site with ID {id} not found"),
      SiteError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SiteError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
    }
  }
}

impl std::error::Error for SiteError {}

impl From<sqlx::Error> for SiteError {
  fn from(err: sqlx::Error) -> Self {
    SiteError::DatabaseError(err)
  }
}

impl From<RoleError> for SiteError {
  fn from(err: RoleError) -> Self {
    match err {
      RoleError::DatabaseError(db_err) => SiteError::DatabaseError(db_err),
      RoleError::AuthenticationError(msg) => SiteError::AuthenticationError(msg),
      RoleError::AuthorizationError(msg) => SiteError::AuthorizationError(msg),
      // Other RoleError variants shouldn't occur in site operations,
      // but we'll handle them as database errors for safety
      _ => SiteError::DatabaseError(sqlx::Error::Protocol(format!("Role error: {err}"))),
    }
  }
}
