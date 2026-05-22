use std::fmt;

/// Custom error type for password operations
#[derive(Debug)]
pub enum PasswordError {
  /// Error occurred during password hashing
  HashingError(String),
  /// Error occurred during password verification
  VerificationError(String),
  /// Invalid hash format provided
  InvalidHash(String),
}

impl fmt::Display for PasswordError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PasswordError::HashingError(msg) => write!(f, "Password hashing error: {msg}"),
      PasswordError::VerificationError(msg) => write!(f, "Password verification error: {msg}"),
      PasswordError::InvalidHash(msg) => write!(f, "Invalid hash format: {msg}"),
    }
  }
}

impl std::error::Error for PasswordError {}
