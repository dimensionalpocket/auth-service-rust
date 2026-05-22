use crate::graphql::types::UserRole;
use async_graphql::SimpleObject;

/// GraphQL output type for user with role information
#[derive(SimpleObject)]
pub struct UserWithRoleResponse {
  /// The user's ID
  #[graphql(name = "id")]
  pub id: i64,
  /// The user's username
  #[graphql(name = "name")]
  pub name: String,
  /// The user's role information
  pub role: UserRole,
  /// The user's UUID (public identifier) - optional for register responses
  pub uuid: Option<String>,
  /// Timestamp when the user was created - optional for login responses
  #[graphql(name = "createdTs")]
  pub created_ts: Option<i64>,
  /// Timestamp when the user was last updated - optional for login responses
  #[graphql(name = "updatedTs")]
  pub updated_ts: Option<i64>,
}
