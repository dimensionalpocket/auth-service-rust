/// Custom error type for role operations
#[derive(Debug)]
pub enum RoleError {
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Authentication failed (used by orchestrators)
  AuthenticationError(String),
  /// Authorization failed (used by orchestrators)
  AuthorizationError(String),
  /// Role not found
  RoleNotFound(i64),
  /// Role name already exists
  RoleNameAlreadyExists(String),
  /// Role is in use and cannot be deleted
  RoleInUse(i64),
  /// Input validation failed
  ValidationError(String),
  /// Invalid permission string
  InvalidPermission(String),
}

impl std::fmt::Display for RoleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RoleError::DatabaseError(err) => write!(f, "Database error: {err}"),
      RoleError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      RoleError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
      RoleError::RoleNotFound(id) => write!(f, "Role not found: {id}"),
      RoleError::RoleNameAlreadyExists(name) => write!(f, "Role name already exists: {name}"),
      RoleError::RoleInUse(id) => write!(f, "Role is in use and cannot be deleted: {id}"),
      RoleError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      RoleError::InvalidPermission(permission) => write!(f, "Invalid permission: {permission}"),
    }
  }
}

impl std::error::Error for RoleError {}

impl From<sqlx::Error> for RoleError {
  fn from(err: sqlx::Error) -> Self {
    RoleError::DatabaseError(err)
  }
}
