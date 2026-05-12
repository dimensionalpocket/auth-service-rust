use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::orchestrators::role::GetRoleOrchestrator;
use crate::types::{DpsAuthApiConfig, RoleError};
use async_graphql::{Context, Object, Result};
use tracing::instrument;

/// GraphQL output type for role details
#[derive(async_graphql::SimpleObject)]
pub struct RoleResponse {
  /// The role's database ID
  pub id: i64,
  /// The role's name
  pub name: String,
  /// The role's permissions as a string array
  pub permissions: Vec<String>,
  /// Whether this is the default role for new users
  pub is_default: bool,
  /// Timestamp when role was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when role was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Role query resolver - returns role data by ID
#[derive(Default, Debug)]
pub struct RoleResolver;

#[Object]
impl RoleResolver {
  /// Returns role information by ID.
  ///
  /// This query:
  /// - Requires user authentication
  /// - Checks if the user has "can_manage_roles" permission
  /// - Returns all role fields including permissions
  /// - Returns error if role is not found
  ///
  /// # Arguments
  /// * `id` - The database ID of the role to retrieve
  ///
  /// # Returns
  /// * `RoleResponse` - Role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns "Role not found" if role with given ID doesn't exist
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(role_id = %id))]
  #[graphql(name = "role")]
  async fn role(&self, ctx: &Context<'_>, id: i64) -> Result<RoleResponse> {
    let databases = ctx.data::<Databases>()?;
    let config = ctx.data::<DpsAuthApiConfig>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match GetRoleOrchestrator::run(databases, session_context.clone(), config, id).await {
      Ok(role) => {
        let permissions = role.permissions;
        Ok(RoleResponse {
          id: role.id,
          name: role.name,
          permissions,
          is_default: role.is_default,
          created_ts: role.created_ts,
          updated_ts: role.updated_ts,
        })
      }
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::RoleNotFound(role_id)) => Err(async_graphql::Error::new(format!(
        "Role with ID {role_id} not found"
      ))),
      Err(err) => {
        tracing::error!("Failed to get role details: {}", err);
        Err(async_graphql::Error::new("Failed to retrieve role details"))
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
    create_test_config, create_test_query_schema, create_test_role_model_with_databases,
    create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_role_success() {
    // Create admin role with can_manage_roles permission
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create test role
    let test_role = create_test_role_model_with_databases(
      &databases,
      "user",
      &["can_view_user_self", "can_edit_profile"],
      true,
    )
    .await;
    let test_role_id = test_role.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = RoleResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = format!(
      r#"
            query {{
                role(id: {test_role_id}) {{
                    id
                    name
                    permissions
                    isDefault
                    createdTs
                    updatedTs
                }}
            }}
            "#
    );

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["role"];

    assert_eq!(role_data["id"].as_i64().unwrap(), test_role_id);
    assert_eq!(role_data["name"].as_str().unwrap(), "user");
    assert!(role_data["isDefault"].as_bool().unwrap());
    assert!(role_data["createdTs"].as_i64().unwrap() > 0);
    assert!(role_data["updatedTs"].as_i64().unwrap() > 0);

    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 2);
    let permission_strings: Vec<String> = permissions
      .iter()
      .map(|p| p.as_str().unwrap().to_string())
      .collect();
    assert!(permission_strings.contains(&"can_view_user_self".to_string()));
    assert!(permission_strings.contains(&"can_edit_profile".to_string()));
  }

  #[dps_auth_db_test]
  async fn test_get_role_forbidden() {
    // Create user role without can_manage_roles permission
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user_with_databases(&databases, "user", user_role_id).await;
    let user_id = user.id;

    // Create test role to try to retrieve
    create_test_role_with_databases(&databases, "editor", &["can_edit_content"]).await;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = RoleResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
            query {
                role(id: 2) {
                    id
                    name
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_db_test]
  async fn test_get_role_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let query_resolver = RoleResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
            query {
                role(id: 1) {
                    id
                    name
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("No valid session"));
  }

  #[dps_auth_db_test]
  async fn test_get_role_not_found() {
    // Create admin role with can_manage_roles permission
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_manage_roles"]).await;

    // Create admin user
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = RoleResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
            query {
                role(id: 999) {
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
