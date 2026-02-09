use crate::middleware::session::SessionContext;
use crate::orchestrators::role::GetRolePermissionsOrchestrator;
use crate::types::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(Default, Debug)]
pub struct RolePermissionsResolver;

#[Object]
impl RolePermissionsResolver {
  /// Get all available role permissions
  /// Requires can_manage_roles permission
  #[graphql(name = "rolePermissions")]
  #[instrument(skip(self, ctx), fields())]
  async fn role_permissions(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = ctx.data::<SessionContext>()?;

    match GetRolePermissionsOrchestrator::run(pool, session_context.clone()).await {
      Ok(permissions) => Ok(permissions),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to get role permissions: {}", err);
        Err(async_graphql::Error::new(
          "Failed to retrieve role permissions",
        ))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::models::role::ROLE_PERMISSIONS;
  use crate::test_utils::{
    create_test_query_schema, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_role_permissions_user_without_manage_roles_permission() {
    // Create regular user role
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, databases.clone(), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("Forbidden: Insufficient permissions"));
  }

  #[dps_auth_db_test]
  async fn test_role_permissions_user_with_manage_roles_but_not_admin_permission() {
    // Create role manager role
    let role_manager_id =
      create_test_role_with_databases(&databases, "role_manager", &["can_manage_roles"]).await;
    let role_manager_user =
      create_test_user_with_databases(&databases, "role_manager", role_manager_id).await;

    // Create session context for role manager
    let session_payload = DpsAuthSessionPayload {
      sub: role_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, databases.clone(), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let permissions = data["rolePermissions"].as_array().unwrap();

    // Should contain all permissions except is_admin
    let permission_strings: Vec<String> = permissions
      .iter()
      .map(|p| p.as_str().unwrap().to_string())
      .collect();

    assert!(!permission_strings.contains(&"is_admin".to_string()));
    assert!(permission_strings.contains(&"can_manage_roles".to_string()));
    assert!(permission_strings.contains(&"can_view_user_self".to_string()));
    assert_eq!(permission_strings.len(), ROLE_PERMISSIONS.len() - 1); // All except is_admin
  }

  #[dps_auth_db_test]
  async fn test_role_permissions_user_with_admin_management_permission() {
    // Create admin manager role
    let admin_manager_id = create_test_role_with_databases(
      &databases,
      "admin_manager",
      &["can_manage_roles", "can_manage_admin_role_permission"],
    )
    .await;
    let admin_manager_user =
      create_test_user_with_databases(&databases, "admin_manager", admin_manager_id).await;

    // Create session context for admin manager
    let session_payload = DpsAuthSessionPayload {
      sub: admin_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, databases.clone(), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let permissions = data["rolePermissions"].as_array().unwrap();

    // Should contain all permissions including is_admin
    let permission_strings: Vec<String> = permissions
      .iter()
      .map(|p| p.as_str().unwrap().to_string())
      .collect();

    assert!(permission_strings.contains(&"is_admin".to_string()));
    assert!(permission_strings.contains(&"can_manage_roles".to_string()));
    assert!(permission_strings.contains(&"can_manage_admin_role_permission".to_string()));
    assert_eq!(permission_strings.len(), ROLE_PERMISSIONS.len()); // All permissions
  }

  #[dps_auth_db_test]
  async fn test_role_permissions_admin_user() {
    // Create admin role
    let admin_role_id = create_test_role_with_databases(&databases, "admin", &["is_admin"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, databases.clone(), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let permissions = data["rolePermissions"].as_array().unwrap();

    // Admin should see all permissions (admin bypasses can_manage_roles check)
    let permission_strings: Vec<String> = permissions
      .iter()
      .map(|p| p.as_str().unwrap().to_string())
      .collect();

    assert!(permission_strings.contains(&"is_admin".to_string()));
    assert!(permission_strings.contains(&"can_manage_roles".to_string()));
    assert_eq!(permission_strings.len(), ROLE_PERMISSIONS.len()); // All permissions
  }

  #[dps_auth_db_test]
  async fn test_role_permissions_unauthenticated() {
    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, databases.clone(), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }
}
