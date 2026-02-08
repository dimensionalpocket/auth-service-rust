use crate::middleware::session::SessionContext;
use crate::orchestrators::role::SetDefaultRoleOrchestrator;
use crate::types::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for set default role response
#[derive(async_graphql::SimpleObject)]
pub struct SetDefaultRoleResponse {
  /// The role's database ID
  pub id: i64,
  /// The role's name
  pub name: String,
  /// Whether this is the default role for new users
  #[graphql(name = "isDefault")]
  pub is_default: bool,
  /// The role's permissions as a string array
  pub permissions: Vec<String>,
  /// Timestamp when the role was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the role was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Set default role mutation resolver
#[derive(Default, Debug)]
pub struct SetDefaultRoleResolver;

#[Object]
impl SetDefaultRoleResolver {
  /// Sets a role as the default role for new users.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_manage_roles" permission
  /// - Validates that the role exists
  /// - Atomically sets the role as default while unsetting any existing default
  /// - Automatically updates the updated_ts timestamp
  /// - Returns the updated role information
  ///
  /// # Arguments
  /// * `role_id` - ID of the role to set as default
  ///
  /// # Returns
  /// * `SetDefaultRoleResponse` - The updated role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns GraphQL error if role is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(role_id = %role_id))]
  #[graphql(name = "setDefaultRole")]
  async fn set_default_role(
    &self,
    ctx: &Context<'_>,
    role_id: i64,
  ) -> Result<SetDefaultRoleResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match SetDefaultRoleOrchestrator::run(pool, session_context.clone(), role_id).await {
      Ok(role) => {
        let permissions = role.permissions;
        Ok(SetDefaultRoleResponse {
          id: role.id,
          name: role.name,
          is_default: role.is_default,
          permissions,
          created_ts: role.created_ts,
          updated_ts: role.updated_ts,
        })
      }
      Err(RoleError::RoleNotFound(id)) => Err(async_graphql::Error::new(format!(
        "Role with ID {id} not found"
      ))),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to set default role: {}", err);
        Err(async_graphql::Error::new("Failed to set default role"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::roles::GetRoleByIdQuery;
  use crate::test_utils::{
    create_test_mutation_schema, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_set_default_role_success() {
    // Create admin role with can_manage_roles permission
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a role first
    let role_id = create_test_role_with_pool(
      &pool,
      "test-role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user.id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = SetDefaultRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        setDefaultRole(roleId: $ROLE_ID) {
          id
          name
          isDefault
          permissions
          createdTs
          updatedTs
        }
      }
    "#
    .replace("$ROLE_ID", &role_id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["setDefaultRole"];

    assert_eq!(role_data["id"].as_i64().unwrap(), role_id);
    assert_eq!(role_data["name"].as_str().unwrap(), "test-role");
    assert!(role_data["isDefault"].as_bool().unwrap());
    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 2);
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_view_user_self"));
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_list_users"));
    assert!(role_data["createdTs"].as_i64().unwrap() > 0);
    assert!(role_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_set_default_role_atomic_behavior() {
    // Create admin role with can_manage_roles permission
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create two roles
    let role1_id = create_test_role_with_pool(&pool, "role1", &["can_view_user_self"]).await;
    let role2_id = create_test_role_with_pool(&pool, "role2", &["can_list_users"]).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user.id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = SetDefaultRoleResolver;
    let schema =
      create_test_mutation_schema(mutation, Some(pool.clone()), Some(session_context), None);

    // Set first role as default
    let query1 = r#"
      mutation {
        setDefaultRole(roleId: $ROLE_ID) {
          id
          isDefault
        }
      }
    "#
    .replace("$ROLE_ID", &role1_id.to_string());

    let result1 = schema.execute(query1).await;
    assert!(result1.errors.is_empty());

    // Set second role as default
    let query2 = r#"
      mutation {
        setDefaultRole(roleId: $ROLE_ID) {
          id
          isDefault
        }
      }
    "#
    .replace("$ROLE_ID", &role2_id.to_string());

    let result2 = schema.execute(query2).await;
    assert!(result2.errors.is_empty());

    // Verify only role2 is default now using the existing query utilities
    let mut conn = pool.acquire().await.unwrap();
    let role1_check = GetRoleByIdQuery::run(&mut conn, role1_id)
      .await
      .unwrap()
      .unwrap();
    let role2_check = GetRoleByIdQuery::run(&mut conn, role2_id)
      .await
      .unwrap()
      .unwrap();

    assert!(!role1_check.is_default);
    assert!(role2_check.is_default);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_set_default_role_forbidden() {
    // Create user role without can_manage_roles permission
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a role first
    let role_id = create_test_role_with_pool(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user.id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = SetDefaultRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        setDefaultRole(roleId: $ROLE_ID) {
          id
          name
        }
      }
    "#
    .replace("$ROLE_ID", &role_id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_set_default_role_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = SetDefaultRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        setDefaultRole(roleId: 1) {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_set_default_role_not_found() {
    // Create admin role with can_manage_roles permission
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user.id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = SetDefaultRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        setDefaultRole(roleId: 999) {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
  }
}
