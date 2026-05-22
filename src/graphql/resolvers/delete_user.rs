use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::orchestrators::user::DeleteUserOrchestrator;
use crate::types::UserError;
use async_graphql::{Context, Object, Result};
use dps_config::DpsConfig;
use std::sync::Arc;
use tracing::instrument;

/// User deletion mutation resolver
#[derive(Default, Debug)]
pub struct DeleteUserResolver;

#[Object]
impl DeleteUserResolver {
  /// Deletes an existing user.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_delete_user" permission
  /// - Prevents users from deleting themselves
  /// - Deletes the user from the database
  /// - Returns true if deletion was successful
  ///
  /// # Arguments
  /// * `id` - ID of user to delete
  ///
  /// # Returns
  /// * `bool` - True if user was successfully deleted
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_delete_user" permission
  /// * Returns "User with ID {id} not found" if target user doesn't exist
  /// * Returns "Cannot delete your own account" if trying to delete self
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(user_id = %id))]
  #[graphql(name = "deleteUser")]
  async fn delete_user(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
    let databases = ctx.data::<Databases>()?;
    let config = ctx.data::<Arc<DpsConfig>>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match DeleteUserOrchestrator::run(databases, session_context.clone(), config, id).await {
      Ok(_) => Ok(true),
      Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
        "User with ID {user_id} not found"
      ))),
      Err(UserError::SelfDeletion) => {
        Err(async_graphql::Error::new("Cannot delete your own account"))
      }
      Err(err) => {
        tracing::error!("Failed to delete user: {}", err);
        Err(async_graphql::Error::new("Failed to delete user"))
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
    create_test_dps_config, create_test_mutation_schema, create_test_role_with_databases,
    create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;
  use sqlx::Row;

  #[dps_auth_db_test]
  async fn test_delete_user_success() {
    // Create roles
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    // Create users
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = format!(
      r#"
            mutation {{
                deleteUser(id: {})
            }}
            "#,
      target_user.id
    );

    let result = schema.execute(query).await;

    if !result.errors.is_empty() {
      println!("GraphQL errors: {:?}", result.errors);
    }

    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["deleteUser"], true);

    // Verify user is actually deleted from database
    let deleted_user = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(target_user.id)
      .fetch_one(&main_pool)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(deleted_user, 0);
  }

  #[dps_auth_db_test]
  async fn test_delete_user_forbidden() {
    // Create role without can_delete_user permission
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    // Create regular user
    let regular_user = create_test_user_with_databases(&databases, "user1", user_role_id).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = format!(
      r#"
            mutation {{
                deleteUser(id: {})
            }}
            "#,
      target_user.id
    );

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_db_test]
  async fn test_delete_user_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = r#"
            mutation {
                deleteUser(id: 1)
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("No valid session"));
  }

  #[dps_auth_db_test]
  async fn test_delete_user_self_deletion_prevented() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = format!(
      r#"
            mutation {{
                deleteUser(id: {})
            }}
            "#,
      admin_user.id // Same as session user ID
    );

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("Cannot delete your own account"));

    // Verify user still exists
    let user_still_exists = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = ?")
      .bind(admin_user.id)
      .fetch_one(&main_pool)
      .await
      .unwrap()
      .get::<i64, _>("count");
    assert_eq!(user_still_exists, 1);
  }

  #[dps_auth_db_test]
  async fn test_delete_user_not_found() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = r#"
            mutation {
                deleteUser(id: 999)
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("User with ID 999 not found"));
  }

  #[dps_auth_db_test]
  async fn test_delete_user_nonexistent_session_user() {
    // Create target user
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(), // Non-existent user ID
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = DeleteUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_dps_config()),
    );

    let query = format!(
      r#"
            mutation {{
                deleteUser(id: {})
            }}
            "#,
      target_user.id
    );

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("User with ID 999 not found"));
  }
}
