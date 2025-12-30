use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use async_graphql::{Context, Object, Result, SimpleObject};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for role listing
#[derive(SimpleObject, Debug)]
pub struct RoleListing {
  pub id: i64,
  pub name: String,
  pub permissions: Vec<String>,
  /// Whether this role is the default role for new users
  #[graphql(name = "isDefault")]
  pub is_default: bool,
  /// Timestamp when the role was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the role was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Roles query resolver for retrieving role information
#[derive(Default, Debug)]
pub struct RolesResolver;

#[Object]
impl RolesResolver {
  /// Returns all roles in the database.
  ///
  /// This query requires authentication and either `can_manage_roles` OR `can_edit_user_role` permission.
  /// Returns all roles with their permissions as string arrays.
  ///
  /// Example response:
  /// ```json
  /// {
  ///   "roles": [
  ///     {
  ///       "id": 1,
  ///       "name": "admin",
  ///       "permissions": ["is_admin", "can_manage_roles"],
  ///       "isDefault": false,
  ///       "createdTs": 1640995200,
  ///       "updatedTs": 1640995200
  ///     }
  ///   ]
  /// }
  /// ```
  #[instrument(skip(self, ctx))]
  #[graphql(name = "roles")]
  async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<RoleListing>> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = ctx.data::<SessionContext>()?;

    let roles =
      RoleOrchestrator::get_all_roles_with_permission_check(pool, session_context.clone())
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

    let role_listings: Vec<RoleListing> = roles
      .into_iter()
      .map(|role| {
        let permissions = role.permissions;
        RoleListing {
          id: role.id,
          name: role.name,
          permissions,
          is_default: role.is_default,
          created_ts: role.created_ts,
          updated_ts: role.updated_ts,
        }
      })
      .collect();

    Ok(role_listings)
  }
}

#[cfg(test)]
mod tests {
  use crate::graphql::resolvers::RolesResolver;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_query_schema, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_roles_admin_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create some roles to retrieve
    create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolesResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute("{ roles { id name permissions isDefault createdTs updatedTs } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let roles = data["roles"].as_array().unwrap();
    assert!(roles.len() >= 3); // admin, user, editor

    // Verify field structure
    for role in roles {
      assert!(role["id"].is_number());
      assert!(role["name"].is_string());
      assert!(role["permissions"].is_array());
      assert!(role["isDefault"].is_boolean());
      assert!(role["createdTs"].is_number());
      assert!(role["updatedTs"].is_number());
    }
  }

  #[tokio::test]
  async fn test_roles_role_editor_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role editor role and user
    let role_editor_id = create_test_role_with_pool(&pool, "role_editor", &["can_edit_user_role"]).await;
    let role_editor_user = create_test_user_with_pool(&pool, "role_editor", role_editor_id).await;

    // Create some roles to retrieve
    create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    create_test_role_with_pool(&pool, "editor", &["can_edit_content"]).await;

    // Create session context for role editor user
    let session_payload = DpsAuthSessionPayload {
      sub: role_editor_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolesResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute("{ roles { id name permissions isDefault createdTs updatedTs } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let roles = data["roles"].as_array().unwrap();
    assert!(roles.len() >= 3); // admin, user, editor

    // Verify field structure
    for role in roles {
      assert!(role["id"].is_number());
      assert!(role["name"].is_string());
      assert!(role["permissions"].is_array());
      assert!(role["isDefault"].is_boolean());
      assert!(role["createdTs"].is_number());
      assert!(role["updatedTs"].is_number());
    }
  }

  #[tokio::test]
  async fn test_roles_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let query = RolesResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute("{ roles { id name permissions isDefault createdTs updatedTs } }")
      .await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication error"));
  }

  #[tokio::test]
  async fn test_roles_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without required permissions
    let user_role_id = create_test_role_with_pool(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolesResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute("{ roles { id name permissions isDefault createdTs updatedTs } }")
      .await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authorization error"));
  }

  #[tokio::test]
  async fn test_roles_empty_database() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["is_admin", "can_manage_roles"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolesResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute("{ roles { id name permissions isDefault createdTs updatedTs } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let roles = data["roles"].as_array().unwrap();
    // Should return the admin role that was created
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0]["name"].as_str().unwrap(), "admin");
  }
}
