use dps_auth_session::DpsAuthSessionError;
use std::fmt;

/// Custom error type for session operations
#[derive(Debug)]
pub enum SessionError {
  /// Token encoding/decoding errors from the auth session service
  AuthSessionError(DpsAuthSessionError),
  /// Authentication failed - user input validation
  AuthenticationError(String),
  /// Database operation failed during authentication
  DatabaseError(String),
  /// Password verification failed
  PasswordVerificationError(String),
  /// Server configuration error (e.g., missing session secret)
  ConfigurationError(String),
}

impl fmt::Display for SessionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SessionError::AuthSessionError(err) => write!(f, "Auth session error: {err}"),
      SessionError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      SessionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      SessionError::PasswordVerificationError(msg) => {
        write!(f, "Password verification error: {msg}")
      }
      SessionError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
    }
  }
}

impl From<DpsAuthSessionError> for SessionError {
  fn from(err: DpsAuthSessionError) -> Self {
    SessionError::AuthSessionError(err)
  }
}

impl std::error::Error for SessionError {}
