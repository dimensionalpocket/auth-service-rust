use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use crate::queries::roles::UpdateRoleData;
use crate::services::role_service::RoleError;
use async_graphql::{Context, InputObject, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL input type for role update data
#[derive(InputObject)]
pub struct UpdateRoleDataInput {
  /// Optional new name for the role
  pub name: Option<String>,
  /// Optional new permissions for the role
  pub permissions: Option<Vec<String>>,
}

/// GraphQL output type for role update response
#[derive(async_graphql::SimpleObject)]
pub struct UpdateRoleResponse {
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

/// Role update mutation resolver
#[derive(Default, Debug)]
pub struct UpdateRoleResolver;

#[Object]
impl UpdateRoleResolver {
  /// Updates an existing role with the provided parameters.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_manage_roles" permission
  /// - Validates all permissions in the input array are valid
  /// - Updates only the fields provided (PATCH semantics)
  /// - Automatically updates the updated_ts timestamp
  /// - Returns the updated role information
  ///
  /// # Arguments
  /// * `id` - Role ID to update
  /// * `name` - Optional new name for the role
  /// * `permissions` - Optional new permissions array for the role
  ///
  /// # Returns
  /// * `UpdateRoleResponse` - The updated role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns GraphQL error if role is not found
  /// * Returns GraphQL error if any permission in the input array is invalid
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(id = %id))]
  #[graphql(name = "updateRole")]
  async fn update_role(
    &self,
    ctx: &Context<'_>,
    id: i64,
    name: Option<String>,
    permissions: Option<Vec<String>>,
  ) -> Result<UpdateRoleResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context
    let session_context = SessionContext::from_context(ctx)?;

    // Convert parameters to UpdateRoleData
    let update_data = UpdateRoleData {
      id,
      name,
      permissions,
    };

    match RoleOrchestrator::update_role_with_permission_check(
      pool,
      session_context.clone(),
      id,
      update_data,
    )
    .await
    {
      Ok(role) => {
        let permissions = role.permissions();
        Ok(UpdateRoleResponse {
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
      Err(RoleError::InvalidPermission(permission)) => Err(async_graphql::Error::new(format!(
        "Invalid permission: {permission}"
      ))),
      Err(RoleError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to update role: {}", err);
        Err(async_graphql::Error::new("Failed to update role"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_database, create_test_role};
  use async_graphql::*;
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  // Minimal query struct for testing mutations in isolation
  #[derive(Default)]
  struct TestEmptyQuery;

  #[Object]
  impl TestEmptyQuery {
    async fn dummy(&self) -> &str {
      "test"
    }
  }

  #[tokio::test]
  async fn test_update_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a role first
    let role_id = create_test_role(
      &pool,
      "test-role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(
          id: $ROLE_ID,
          name: "updated-role", 
          permissions: ["can_edit_user", "can_delete_user"]
        ) {
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
    let role_data = &data["updateRole"];

    assert_eq!(role_data["id"].as_i64().unwrap(), role_id);
    assert_eq!(role_data["name"].as_str().unwrap(), "updated-role");
    assert!(!role_data["isDefault"].as_bool().unwrap());
    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 2);
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_edit_user"));
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_delete_user"));
    assert!(role_data["createdTs"].as_i64().unwrap() > 0);
    assert!(role_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[tokio::test]
  async fn test_update_role_partial_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a role first
    let role_id = create_test_role(
      &pool,
      "partial-role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(
          id: $ROLE_ID,
          name: "partial-updated"
        ) {
          id
          name
          permissions
        }
      }
    "#
    .replace("$ROLE_ID", &role_id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["updateRole"];

    assert_eq!(role_data["id"].as_i64().unwrap(), role_id);
    assert_eq!(role_data["name"].as_str().unwrap(), "partial-updated");
    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 2); // unchanged
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_view_user_self"));
    assert!(permissions
      .iter()
      .any(|p| p.as_str().unwrap() == "can_list_users"));
  }

  #[tokio::test]
  async fn test_update_role_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert user role without can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert regular user
    let user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user-uuid', 1234567890, 1234567890, 'user', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let user_id = user_result.last_insert_rowid();

    // Create a role first
    let role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(id: $ROLE_ID, name: "forbidden-role") {
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

  #[tokio::test]
  async fn test_update_role_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(id: 1, name: "unauth-role") {
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
  async fn test_update_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(id: 999, name: "nonexistent-role") {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
  }

  #[tokio::test]
  async fn test_update_role_invalid_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a role first
    let role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(
          id: $ROLE_ID,
          permissions: ["invalid_permission"]
        ) {
          id
          name
        }
      }
    "#
    .replace("$ROLE_ID", &role_id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Invalid permission"));
  }

  #[tokio::test]
  async fn test_update_role_empty_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_manage_roles permission
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a role first
    let role_id = create_test_role(&pool, "test-role", &["can_view_user_self"]).await;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateRoleResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateRole(
          id: $ROLE_ID,
          permissions: []
        ) {
          id
          name
          permissions
        }
      }
    "#
    .replace("$ROLE_ID", &role_id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["updateRole"];

    assert_eq!(role_data["id"].as_i64().unwrap(), role_id);
    assert_eq!(role_data["name"].as_str().unwrap(), "test-role"); // unchanged
    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 0); // should be empty now
  }
}
