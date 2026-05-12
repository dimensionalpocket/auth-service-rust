use crate::database::Databases;
use crate::graphql::types::UserRole;
use crate::middleware::session::SessionContext;
use crate::orchestrators::user::GetUsersOrchestrator;
use crate::types::{DpsAuthApiConfig, UserError};
use async_graphql::{Context, Object, Result};
use tracing::instrument;

/// GraphQL output type for user listing
#[derive(async_graphql::SimpleObject)]
pub struct UserListing {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  /// The user's role information
  pub role: UserRole,
  /// Timestamp when the user was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the user was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UsersResolver;

#[Object]
impl UsersResolver {
  /// Returns all users with their role information (admin only).
  ///
  /// This query:
  /// - Requires user authentication
  /// - Checks if the user has "can_list_users" permission
  /// - Returns all users with their role names
  /// - Orders users by name alphabetically
  ///
  /// # Returns
  /// * `Vec<UserListing>` - List of all users with role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_list_users" permission
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(self, ctx))]
  #[graphql(name = "users")]
  async fn users(&self, ctx: &Context<'_>) -> Result<Vec<UserListing>> {
    let databases = ctx.data::<Databases>()?;
    let config = ctx.data::<DpsAuthApiConfig>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match GetUsersOrchestrator::run(databases, session_context.clone(), config).await {
      Ok(users) => Ok(
        users
          .into_iter()
          .map(|user| UserListing {
            id: user.user.id,
            uuid: user.user.uuid,
            name: user.user.name,
            role: UserRole::from(user.role),
            created_ts: user.user.created_ts,
            updated_ts: user.user.updated_ts,
          })
          .collect(),
      ),
      Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
        "User with ID {user_id} not found"
      ))),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to list users: {}", err);
        Err(async_graphql::Error::new("Failed to retrieve users"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_query_schema, create_test_role_with_databases,
    create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_users_success() {
    // Create roles
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_list_users"]).await;
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    // Create users
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let _regular_user = create_test_user_with_databases(&databases, "user1", user_role_id).await;
    let _another_user = create_test_user_with_databases(&databases, "user2", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = UsersResolver;
    let schema = create_test_query_schema(
      query,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let result = schema
      .execute("{ users { id uuid name role { id name permissions } createdTs updatedTs } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let users = data["users"].as_array().unwrap();
    assert_eq!(users.len(), 3);

    // Verify field structure
    for user in users {
      assert!(user["id"].is_number());
      assert!(user["uuid"].is_string());
      assert!(user["name"].is_string());
      assert!(user["role"]["id"].is_string());
      assert!(user["role"]["name"].is_string());
      assert!(user["role"]["permissions"].is_array());
      assert!(user["createdTs"].is_number());
      assert!(user["updatedTs"].is_number());
    }

    // Verify ordering by name
    let names: Vec<String> = users
      .iter()
      .map(|u| u["name"].as_str().unwrap().to_string())
      .collect();
    assert_eq!(names, vec!["admin", "user1", "user2"]);
  }

  #[dps_auth_db_test]
  async fn test_users_unauthenticated() {
    // Create session context without user ID (not authenticated)
    let session_context = SessionContext::new(None);

    let query = UsersResolver;
    let schema = create_test_query_schema(
      query,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let result = schema.execute("{ users { id name } }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("No valid session"));
  }

  #[dps_auth_db_test]
  async fn test_users_forbidden() {
    // Create role without can_list_users permission
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    // Create regular user
    let regular_user = create_test_user_with_databases(&databases, "user1", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = UsersResolver;
    let schema = create_test_query_schema(
      query,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let result = schema.execute("{ users { id name } }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_db_test]
  async fn test_users_empty_database() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_list_users"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = UsersResolver;
    let schema = create_test_query_schema(
      query,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let result = schema.execute("{ users { id name role { name } } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let users = data["users"].as_array().unwrap();
    assert_eq!(users.len(), 1); // Only admin user should be returned
    assert_eq!(users[0]["name"].as_str().unwrap(), "admin");
    assert_eq!(users[0]["role"]["name"].as_str().unwrap(), "admin");
  }

  #[dps_auth_db_test]
  async fn test_users_nonexistent_user() {
    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = UsersResolver;
    let schema = create_test_query_schema(
      query,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let result = schema.execute("{ users { id name } }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("User with ID 999 not found"));
  }
}
