use crate::models::Role;

/// Result type for authentication operations containing both user and session information
#[derive(Debug, Clone)]
pub struct AuthResult {
  pub user_id: i64,
  pub username: String,
  pub role: Role,
  pub session_token: String,
}

/// Result type for user registration operations containing user information
#[derive(Debug, Clone)]
pub struct RegisterResult {
  pub user_id: i64,
  pub username: String,
  pub uuid: String,
  pub role: Role,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub session_token: String,
}

/// Result type for getting current authenticated user information
#[derive(Debug, Clone)]
pub struct AuthMeResult {
  pub user_id: i64,
  pub username: String,
  pub uuid: String,
  pub role: Role,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub session_iat: i64,
  pub session_exp: i64,
}
