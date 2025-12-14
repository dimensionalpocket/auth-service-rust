use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use crate::services::role_service::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for role removal response
#[derive(async_graphql::SimpleObject)]
pub struct RemoveRoleResponse {
  /// Whether the role was successfully deleted
  pub success: bool,
  /// The deleted role's database ID
  pub id: i64,
  /// The deleted role's name
  pub name: String,
}

/// Role removal mutation resolver
#[derive(Default, Debug)]
pub struct RemoveRoleResolver;

#[Object]
impl RemoveRoleResolver {
  /// Removes a role by ID.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if user has "can_manage_roles" permission
  /// - Validates that no users are currently using the role
  /// - Deletes the role from database
  /// - Returns success status and deleted role information
  ///
  /// # Arguments
  /// * `id` - The database ID of the role to delete
  ///
  /// # Returns
  /// * `RemoveRoleResponse` - Success status and deleted role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns "Role not found" if role with given ID doesn't exist
  /// * Returns "Role is in use and cannot be deleted" if any users are using the role
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(id = %id))]
  #[graphql(name = "removeRole")]
  async fn remove_role(&self, ctx: &Context<'_>, id: i64) -> Result<RemoveRoleResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match RoleOrchestrator::delete_role_with_permission_check(pool, session_context.clone(), id)
      .await
    {
      Ok(role) => Ok(RemoveRoleResponse {
        success: true,
        id: role.id,
        name: role.name.clone(),
      }),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::RoleNotFound(role_id)) => Err(async_graphql::Error::new(format!(
        "Role not found: {role_id}"
      ))),
      Err(RoleError::RoleInUse(role_id)) => Err(async_graphql::Error::new(format!(
        "Role is in use and cannot be deleted: {role_id}"
      ))),
      Err(err) => {
        tracing::error!("Failed to delete role: {}", err);
        Err(async_graphql::Error::new("Failed to delete role"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_mutation_schema, create_test_role, create_test_user,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_remove_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_manage_roles permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create a role to delete
    let _role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 2) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let response_data = &data["removeRole"];

    assert!(response_data["success"].as_bool().unwrap());
    assert_eq!(response_data["id"].as_i64().unwrap(), 2);
    assert_eq!(response_data["name"].as_str().unwrap(), "test-role");
  }

  #[tokio::test]
  async fn test_remove_role_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without can_manage_roles permission
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user(&pool, "user", user_role_id).await;
    let user_id = user.id;

    // Create a role to try to delete
    let _role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 2) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[tokio::test]
  async fn test_remove_role_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 1) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[tokio::test]
  async fn test_remove_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_manage_roles permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 999) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Role not found"));
  }

  #[tokio::test]
  async fn test_remove_role_in_use() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_manage_roles permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create a role to delete
    let role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create a user with the role to be deleted
    create_test_user(&pool, "test-user", role_id).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 2) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("Role is in use and cannot be deleted"));
  }

  #[tokio::test]
  async fn test_remove_role_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context for non-existent user
    let session_payload = ServiceSessionPayload {
      sub: 999,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeRole(id: 1) {
          success
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("User not found"));
  }
}
