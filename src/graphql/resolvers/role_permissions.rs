use crate::middleware::session::SessionContext;
use crate::models::role::ROLE_PERMISSIONS;
use crate::queries::users::GetUserByIdQuery;
use crate::services::role_service::RoleService;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

// TODO: Migrate this resolver to use orchestrator pattern like other resolvers.
// This resolver was implemented before orchestrators became the standard pattern.

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

    // Get current user
    let session_payload = session_context
      .payload
      .as_ref()
      .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

    let user = {
      let mut conn = pool
        .acquire()
        .await
        .map_err(|e| async_graphql::Error::new(format!("Failed to acquire connection: {e}")))?;
      GetUserByIdQuery::run(&mut conn, session_payload.sub)
        .await
        .map_err(|e| async_graphql::Error::new(format!("Failed to get current user: {e}")))?
        .ok_or_else(|| async_graphql::Error::new("User not found"))?
    };

    // Check permissions using scoped connection
    let (allowed, can_manage_admin) = {
      let mut conn = pool
        .acquire()
        .await
        .map_err(|e| async_graphql::Error::new(format!("Failed to acquire connection: {e}")))?;

      let allowed = RoleService::check_user_permission(&mut conn, &user, "can_manage_roles")
        .await
        .map_err(|e| async_graphql::Error::new(format!("Permission check failed: {e}")))?;

      let can_manage_admin =
        RoleService::check_user_permission(&mut conn, &user, "can_manage_admin_role_permission")
          .await
          .map_err(|e| async_graphql::Error::new(format!("Permission check failed: {e}")))?;

      (allowed, can_manage_admin)
    };

    if !allowed {
      return Err(async_graphql::Error::new(
        "Forbidden: Insufficient permissions",
      ));
    }

    // Filter permissions based on user's admin management rights
    let permissions: Vec<String> = ROLE_PERMISSIONS
      .iter()
      .filter(|&&perm| can_manage_admin || perm != "is_admin")
      .map(|s| s.to_string())
      .collect();

    Ok(permissions)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_query_schema, create_test_role, create_test_user,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_role_permissions_user_without_manage_roles_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create regular user role
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;
    let regular_user = create_test_user(&pool, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("Forbidden: Insufficient permissions"));
  }

  #[tokio::test]
  async fn test_role_permissions_user_with_manage_roles_but_not_admin_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role manager role
    let role_manager_id = create_test_role(&pool, "role_manager", &["can_manage_roles"]).await;
    let role_manager_user = create_test_user(&pool, "role_manager", role_manager_id).await;

    // Create session context for role manager
    let session_payload = DpsAuthSessionPayload {
      sub: role_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

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

  #[tokio::test]
  async fn test_role_permissions_user_with_admin_management_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin manager role
    let admin_manager_id = create_test_role(
      &pool,
      "admin_manager",
      &["can_manage_roles", "can_manage_admin_role_permission"],
    )
    .await;
    let admin_manager_user = create_test_user(&pool, "admin_manager", admin_manager_id).await;

    // Create session context for admin manager
    let session_payload = DpsAuthSessionPayload {
      sub: admin_manager_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

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

  #[tokio::test]
  async fn test_role_permissions_admin_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin"]).await;
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;

    // Create session context for admin
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

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

  #[tokio::test]
  async fn test_role_permissions_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let query = RolePermissionsResolver;
    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema.execute("{ rolePermissions }").await;

    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }
}
