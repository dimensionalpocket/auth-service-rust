use crate::middleware::session::SessionContext;
use crate::orchestrators::role_orchestrator::RoleOrchestrator;
use crate::queries::roles::CreateRoleData;
use crate::services::role_service::RoleError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL input type for creating a role
#[derive(async_graphql::InputObject)]
pub struct AddRoleData {
  /// The name of role to create
  pub name: String,
  /// The permissions to assign to the role
  pub permissions: Vec<String>,
}

/// GraphQL output type for role addition response
#[derive(async_graphql::SimpleObject)]
pub struct AddRoleResponse {
  /// The created role's database ID
  pub id: i64,
  /// The role's name
  pub name: String,
  /// Whether this role is a default role
  #[graphql(name = "isDefault")]
  pub is_default: bool,
  /// The role's permissions
  pub permissions: Vec<String>,
  /// Timestamp when role was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when role was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Role addition mutation resolver
#[derive(Default, Debug)]
pub struct AddRoleResolver;

#[Object]
impl AddRoleResolver {
  /// Adds a new role with provided parameters.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if user has "can_manage_roles" permission
  /// - Validates role name format and uniqueness
  /// - Validates all permissions against the whitelist
  /// - Creates role in database
  /// - Returns created role information
  ///
  /// # Arguments
  /// * `name` - Unique name for role
  /// * `permissions` - List of permissions to assign to role
  ///
  /// # Returns
  /// * `AddRoleResponse` - The created role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_manage_roles" permission
  /// * Returns GraphQL error if role name validation fails
  /// * Returns GraphQL error if role name already exists
  /// * Returns GraphQL error if any permission is invalid
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(name = %name))]
  #[graphql(name = "addRole")]
  async fn add_role(
    &self,
    ctx: &Context<'_>,
    name: String,
    permissions: Vec<String>,
  ) -> Result<AddRoleResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    let create_data = CreateRoleData {
      name,
      permissions,
      is_default: false,
    };

    match RoleOrchestrator::create_role_with_permission_check(
      pool,
      session_context.clone(),
      create_data,
    )
    .await
    {
      Ok(role) => Ok(AddRoleResponse {
        id: role.id,
        name: role.name.clone(),
        is_default: role.is_default,
        permissions: role.permissions(),
        created_ts: role.created_ts,
        updated_ts: role.updated_ts,
      }),
      Err(RoleError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(RoleError::RoleNameAlreadyExists(name)) => Err(async_graphql::Error::new(format!(
        "Role name '{name}' is already in use"
      ))),
      Err(RoleError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(RoleError::InvalidPermission(permission)) => Err(async_graphql::Error::new(format!(
        "Invalid permission: {permission}"
      ))),
      Err(err) => {
        tracing::error!("Failed to create role: {}", err);
        Err(async_graphql::Error::new("Failed to create role"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_database, create_test_mutation_schema};
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_add_role_success() {
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

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "test-role", 
          permissions: ["can_view_user_self", "can_list_users"]
        ) {
          id
          name
          isDefault
          permissions
          createdTs
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["addRole"];

    assert!(role_data["id"].as_i64().unwrap() > 0);
    assert_eq!(role_data["name"].as_str().unwrap(), "test-role");
    assert!(!role_data["isDefault"].as_bool().unwrap());

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

  #[tokio::test]
  async fn test_add_role_empty_permissions() {
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

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "empty-permissions-role", 
          permissions: []
        ) {
          id
          name
          isDefault
          permissions
          createdTs
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let role_data = &data["addRole"];

    assert_eq!(
      role_data["name"].as_str().unwrap(),
      "empty-permissions-role"
    );
    assert!(!role_data["isDefault"].as_bool().unwrap()); // Always false for now

    let permissions = role_data["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 0);
  }

  #[tokio::test]
  async fn test_add_role_forbidden() {
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

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "forbidden-role", 
          permissions: ["can_view_user_self"]
        ) {
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
  async fn test_add_role_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "unauth-role", 
          permissions: ["can_view_user_self"]
        ) {
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
  async fn test_add_role_duplicate_name() {
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

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "duplicate", 
          permissions: ["can_view_user_self"]
        ) {
          id
          name
        }
      }
    "#;

    // First creation should succeed
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    // Second creation with same name should fail
    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("already in use"));
  }

  #[tokio::test]
  async fn test_add_role_invalid_permission() {
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

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "invalid-permission-role", 
          permissions: ["invalid_permission"]
        ) {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Invalid permission"));
  }

  #[tokio::test]
  async fn test_add_role_empty_name() {
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

    let mutation = AddRoleResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addRole(
          name: "", 
          permissions: ["can_view_user_self"]
        ) {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Validation error"));
  }
}
