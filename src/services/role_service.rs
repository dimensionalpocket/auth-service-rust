use crate::models::role::Role;
use crate::models::user::User;
use crate::queries::roles::{
  CreateRoleData, CreateRoleQuery, DeleteRoleQuery, GetAllRolesQuery, GetRoleByIdQuery,
  GetRoleByNameQuery, SetDefaultRoleQuery, UpdateRoleData, UpdateRoleQuery,
};
use sqlx::SqlitePool;
use tracing::warn;

/// Custom error type for user role operations
#[derive(Debug)]
pub enum RoleError {
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Authentication failed (used by orchestrators)
  AuthenticationError(String),
  /// Authorization failed (used by orchestrators)
  AuthorizationError(String),
  /// Role not found
  RoleNotFound(i64),
  /// Role name already exists
  RoleNameAlreadyExists(String),
  /// Role is in use and cannot be deleted
  RoleInUse(i64),
  /// Input validation failed
  ValidationError(String),
  /// Invalid permission string
  InvalidPermission(String),
}

impl std::fmt::Display for RoleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RoleError::DatabaseError(err) => write!(f, "Database error: {err}"),
      RoleError::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
      RoleError::AuthorizationError(msg) => write!(f, "Authorization error: {msg}"),
      RoleError::RoleNotFound(id) => write!(f, "Role not found: {id}"),
      RoleError::RoleNameAlreadyExists(name) => write!(f, "Role name already exists: {name}"),
      RoleError::RoleInUse(id) => write!(f, "Role is in use and cannot be deleted: {id}"),
      RoleError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      RoleError::InvalidPermission(permission) => write!(f, "Invalid permission: {permission}"),
    }
  }
}

impl std::error::Error for RoleError {}

impl From<sqlx::Error> for RoleError {
  fn from(err: sqlx::Error) -> Self {
    RoleError::DatabaseError(err)
  }
}

pub struct RoleService;

impl RoleService {
  /// Get all roles
  pub async fn get_all_roles(pool: &SqlitePool) -> Result<Vec<Role>, RoleError> {
    GetAllRolesQuery::run(pool)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Get a role by ID
  pub async fn get_role_by_id(pool: &SqlitePool, role_id: i64) -> Result<Option<Role>, RoleError> {
    GetRoleByIdQuery::run(pool, role_id)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Get a role by name
  pub async fn get_role_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Role>, RoleError> {
    GetRoleByNameQuery::run(pool, name)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Create a new role
  ///
  /// This method creates a new role with the provided data.
  /// All permissions in the input array are validated against the whitelist.
  /// Role name format and uniqueness are validated.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `create_data` - Data for creating the new role
  ///
  /// # Returns
  /// * `Ok(Role)` - Created role
  /// * `Err(RoleError)` - Database error, validation error, or role name already exists
  pub async fn create_role(
    pool: &SqlitePool,
    create_data: CreateRoleData,
  ) -> Result<Role, RoleError> {
    // Validate role name format
    if create_data.name.trim().is_empty() {
      return Err(RoleError::ValidationError(
        "Role name cannot be empty".to_string(),
      ));
    }

    // Validate all permissions are valid
    for permission in &create_data.permissions {
      if !crate::models::role::is_valid_role_permission(permission) {
        return Err(RoleError::InvalidPermission(permission.clone()));
      }
    }

    // Check if role name already exists
    let existing_role = GetRoleByNameQuery::run(pool, &create_data.name)
      .await
      .map_err(RoleError::DatabaseError)?;

    if existing_role.is_some() {
      return Err(RoleError::RoleNameAlreadyExists(create_data.name));
    }

    let create_data_with_default_false = CreateRoleData {
      name: create_data.name,
      permissions: create_data.permissions,
      is_default: false,
    };
    CreateRoleQuery::run(pool, create_data_with_default_false)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Delete a role
  ///
  /// This method deletes a role from the database after checking that no users are currently using it.
  /// The role data is returned before deletion for audit purposes.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `role_id` - ID of the role to delete
  ///
  /// # Returns
  /// * `Ok(Role)` - The deleted role data
  /// * `Err(RoleError)` - Database error, role not found, or role in use
  pub async fn delete_role(pool: &SqlitePool, role_id: i64) -> Result<Role, RoleError> {
    use sqlx::Row;

    // First check if any users are using this role
    let user_count = sqlx::query("SELECT COUNT(*) FROM users WHERE role_id = ?")
      .bind(role_id)
      .fetch_one(pool)
      .await
      .map_err(RoleError::DatabaseError)?;

    let count: i64 = user_count.get(0);
    if count > 0 {
      return Err(RoleError::RoleInUse(role_id));
    }

    // Delete the role
    match DeleteRoleQuery::run(pool, role_id).await {
      Ok(role) => Ok(role),
      Err(sqlx::Error::RowNotFound) => Err(RoleError::RoleNotFound(role_id)),
      Err(err) => Err(RoleError::DatabaseError(err)),
    }
  }

  /// Update a role
  ///
  /// This method updates an existing role with the provided data.
  /// Only the fields provided in the update_data will be modified (PATCH semantics).
  /// All permissions in the input array are validated against the whitelist.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `role_id` - ID of the role to update
  /// * `update_data` - Data to update (partial update supported)
  ///
  /// # Returns
  /// * `Ok(Role)` - Updated role
  /// * `Err(RoleError)` - Database error, role not found, or validation error
  pub async fn update_role(
    pool: &SqlitePool,
    role_id: i64,
    update_data: UpdateRoleData,
  ) -> Result<Role, RoleError> {
    // Validate permissions if provided
    if let Some(ref permissions) = update_data.permissions {
      for permission in permissions {
        if !crate::models::role::is_valid_role_permission(permission) {
          return Err(RoleError::InvalidPermission(permission.clone()));
        }
      }
    }

    // Create update data with the correct ID
    let update_data_with_id = UpdateRoleData {
      id: role_id,
      name: update_data.name,
      permissions: update_data.permissions,
    };

    // Run the update query
    let updated_role = UpdateRoleQuery::run(pool, update_data_with_id)
      .await
      .map_err(RoleError::DatabaseError)?;

    match updated_role {
      Some(role) => Ok(role),
      None => Err(RoleError::RoleNotFound(role_id)),
    }
  }

  /// Set a role as the default role
  ///
  /// This method atomically sets the specified role as the default role while
  /// unsetting any existing default role. The operation is performed in a single
  /// transaction to ensure atomicity.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `role_id` - ID of the role to set as default
  ///
  /// # Returns
  /// * `Ok(Role)` - The updated role with is_default set to true
  /// * `Err(RoleError)` - Database error or role not found
  pub async fn set_default_role(pool: &SqlitePool, role_id: i64) -> Result<Role, RoleError> {
    match SetDefaultRoleQuery::run(pool, role_id).await {
      Ok(Some(role)) => Ok(role),
      Ok(None) => Err(RoleError::RoleNotFound(role_id)),
      Err(err) => Err(RoleError::DatabaseError(err)),
    }
  }

  /// Check if a user has a specific permission
  ///
  /// This method checks if the given user has the specified permission through their role.
  /// Special handling for admin users: if the user's role has the "is_admin" permission,
  /// this method will return true for ANY permission check, regardless of what specific
  /// permission is being requested.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user` - The user to check permissions for (required, not optional)
  /// * `permission` - The permission string to check for
  ///
  /// # Returns
  /// * `Ok(true)` - User has the permission (or is admin)
  /// * `Ok(false)` - User does not have the permission or permission is invalid
  /// * `Err(RoleError)` - Database error occurred
  pub async fn check_user_permission(
    pool: &SqlitePool,
    user: &User,
    permission: &str,
  ) -> Result<bool, RoleError> {
    // Validate permission exists
    if !crate::models::role::is_valid_role_permission(permission) {
      warn!("Invalid permission checked: {}", permission);
      return Ok(false);
    }

    let role = GetRoleByIdQuery::run(pool, user.role_id)
      .await
      .map_err(RoleError::DatabaseError)?;

    match role {
      Some(role) => {
        // Check if user is admin first - admins have all permissions
        if role.has_permission("is_admin") {
          Ok(true)
        } else {
          // For non-admin users, check the specific permission
          Ok(role.has_permission(permission))
        }
      }
      None => Ok(false),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::roles::GetDefaultRoleQuery;
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_role_by_id_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role with permissions
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let role_id = result.last_insert_rowid();
    let role = RoleService::get_role_by_id(&pool, role_id).await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert!(role.has_permission("is_admin"));
  }

  #[tokio::test]
  async fn test_get_role_by_name_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role with permissions
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let role = RoleService::get_role_by_name(&pool, "user").await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_get_all_roles_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with different permissions
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]'), ('user', 1234567891, 1234567891, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let roles = RoleService::get_all_roles(&pool).await.unwrap();

    assert_eq!(roles.len(), 2);

    // Roles should be ordered by name
    assert_eq!(roles[0].name, "admin");
    assert!(!roles[0].is_default);
    assert!(roles[0].has_permission("is_admin"));
    assert!(roles[0].has_permission("can_manage_roles"));

    assert_eq!(roles[1].name, "user");
    assert!(roles[1].is_default);
    assert!(roles[1].has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_get_all_roles_empty_table() {
    let (pool, _temp_file) = create_test_database().await;

    let roles = RoleService::get_all_roles(&pool).await.unwrap();

    assert_eq!(roles.len(), 0);
  }

  #[tokio::test]
  async fn test_check_user_permission_admin_has_all_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let admin_role_id = admin_role_result.last_insert_rowid();

    // Create admin user
    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // Admin should have any valid permission
    assert!(
      RoleService::check_user_permission(&pool, &admin_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      RoleService::check_user_permission(&pool, &admin_user, "can_create_site")
        .await
        .unwrap()
    );
    assert!(
      RoleService::check_user_permission(&pool, &admin_user, "can_delete_user")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_regular_user_specific_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role
    let user_role_result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let user_role_id = user_role_result.last_insert_rowid();

    // Create regular user
    let regular_user = User {
      id: 2,
      uuid: "user-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "user".to_string(),
      role_id: user_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // User should have specific permissions
    assert!(
      RoleService::check_user_permission(&pool, &regular_user, "can_view_user_self")
        .await
        .unwrap()
    );

    // User should NOT have admin permissions
    assert!(
      !RoleService::check_user_permission(&pool, &regular_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      !RoleService::check_user_permission(&pool, &regular_user, "is_admin")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_user_with_no_role() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user with non-existent role
    let user_no_role = User {
      id: 3,
      uuid: "no-role-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "no-role".to_string(),
      role_id: 999, // Non-existent role
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // User with no role should have no permissions
    assert!(
      !RoleService::check_user_permission(&pool, &user_no_role, "can_list_users")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_invalid_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let admin_role_id = admin_role_result.last_insert_rowid();

    // Create admin user
    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // Even admin users should get false for invalid permissions
    assert!(
      !RoleService::check_user_permission(&pool, &admin_user, "invalid_permission")
        .await
        .unwrap()
    );
    assert!(!RoleService::check_user_permission(&pool, &admin_user, "")
      .await
      .unwrap());
    assert!(
      !RoleService::check_user_permission(&pool, &admin_user, "nonexistent_can_permission")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_valid_new_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role with new permissions
    let role_result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('role_manager', 1234567890, 1234567890, FALSE, '[\"can_edit_user_role\", \"can_manage_roles\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = role_result.last_insert_rowid();

    // Create user with role management permissions
    let role_manager_user = User {
      id: 2,
      uuid: "role-manager-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "role_manager".to_string(),
      role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // User should have the new permissions
    assert!(
      RoleService::check_user_permission(&pool, &role_manager_user, "can_edit_user_role")
        .await
        .unwrap()
    );
    assert!(
      RoleService::check_user_permission(&pool, &role_manager_user, "can_manage_roles")
        .await
        .unwrap()
    );

    // User should NOT have admin management permission
    assert!(!RoleService::check_user_permission(
      &pool,
      &role_manager_user,
      "can_manage_admin_role_permission"
    )
    .await
    .unwrap());
  }

  #[tokio::test]
  async fn test_check_user_permission_admin_bypasses_validation() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let admin_role_id = admin_role_result.last_insert_rowid();

    // Create admin user
    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    // Admin should have all valid permissions
    assert!(
      RoleService::check_user_permission(&pool, &admin_user, "can_edit_user_role")
        .await
        .unwrap()
    );
    assert!(
      RoleService::check_user_permission(&pool, &admin_user, "can_manage_roles")
        .await
        .unwrap()
    );
    assert!(RoleService::check_user_permission(
      &pool,
      &admin_user,
      "can_manage_admin_role_permission"
    )
    .await
    .unwrap());
  }

  #[tokio::test]
  async fn test_update_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('test-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = result.last_insert_rowid();

    // Update the role
    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("updated-role".to_string()),
      permissions: Some(vec![
        "can_edit_user".to_string(),
        "can_delete_user".to_string(),
      ]),
    };

    let updated_role = RoleService::update_role(&pool, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "updated-role");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_edit_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_partial_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('partial-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\", \"can_list_users\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = result.last_insert_rowid();

    // Update only the name
    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("partial-updated".to_string()),
      permissions: None,
    };

    let updated_role = RoleService::update_role(&pool, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "partial-updated");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2); // unchanged
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[tokio::test]
  async fn test_update_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let update_data = UpdateRoleData {
      id: 999,
      name: Some("nonexistent".to_string()),
      permissions: None,
    };

    let result = RoleService::update_role(&pool, 999, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound error"),
    }
  }

  #[tokio::test]
  async fn test_update_role_invalid_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('test-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = result.last_insert_rowid();

    // Update with invalid permission
    let update_data = UpdateRoleData {
      id: role_id,
      name: None,
      permissions: Some(vec!["invalid_permission".to_string()]),
    };

    let result = RoleService::update_role(&pool, role_id, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[tokio::test]
  async fn test_update_role_empty_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('empty-permissions-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = result.last_insert_rowid();

    // Update permissions to empty array
    let update_data = UpdateRoleData {
      id: role_id,
      name: None,
      permissions: Some(vec![]), // Set to empty array
    };

    let updated_role = RoleService::update_role(&pool, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.id, role_id);
    assert_eq!(updated_role.name, "empty-permissions-role"); // unchanged
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 0); // should be empty now
  }

  #[tokio::test]
  async fn test_update_role_all_valid_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let result = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('all-permissions-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    let role_id = result.last_insert_rowid();

    // Update with all valid permissions
    let all_permissions: Vec<String> = crate::models::ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let update_data = UpdateRoleData {
      id: role_id,
      name: Some("all-permissions-updated".to_string()),
      permissions: Some(all_permissions.clone()),
    };

    let updated_role = RoleService::update_role(&pool, role_id, update_data)
      .await
      .unwrap();

    assert_eq!(updated_role.name, "all-permissions-updated");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), all_permissions.len());

    // Verify all permissions are present
    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }

  #[tokio::test]
  async fn test_create_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "can_list_users".to_string(),
      ],
      is_default: false,
    };

    let role = RoleService::create_role(&pool, create_data).await.unwrap();

    assert_eq!(role.name, "test_role");
    assert!(!role.is_default);
    assert!(role.created_ts > 0);
    assert_eq!(role.created_ts, role.updated_ts);

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
  }

  #[tokio::test]
  async fn test_create_role_empty_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "empty_permissions_role".to_string(),
      permissions: vec![],
      is_default: true,
    };

    let role = RoleService::create_role(&pool, create_data).await.unwrap();

    assert_eq!(role.name, "empty_permissions_role");
    assert!(!role.is_default); // Always false for now
    assert_eq!(role.permissions().len(), 0);
  }

  #[tokio::test]
  async fn test_create_role_duplicate_name_fails() {
    let (pool, _temp_file) = create_test_database().await;

    // Create first role
    let create_data1 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    RoleService::create_role(&pool, create_data1).await.unwrap();

    // Try to create second role with same name
    let create_data2 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };

    let result = RoleService::create_role(&pool, create_data2).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNameAlreadyExists(name) => assert_eq!(name, "duplicate"),
      _ => panic!("Expected RoleNameAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_create_role_empty_name_fails() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = RoleService::create_role(&pool, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => assert_eq!(msg, "Role name cannot be empty"),
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_whitespace_name_fails() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "   ".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    let result = RoleService::create_role(&pool, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::ValidationError(msg) => assert_eq!(msg, "Role name cannot be empty"),
      _ => panic!("Expected ValidationError"),
    }
  }

  #[tokio::test]
  async fn test_create_role_invalid_permission_fails() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "invalid_permission_role".to_string(),
      permissions: vec!["invalid_permission".to_string()],
      is_default: false,
    };

    let result = RoleService::create_role(&pool, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[tokio::test]
  async fn test_create_role_multiple_invalid_permissions_fails() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "multiple_invalid_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "invalid_permission1".to_string(),
        "invalid_permission2".to_string(),
      ],
      is_default: false,
    };

    let result = RoleService::create_role(&pool, create_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::InvalidPermission(permission) => assert_eq!(permission, "invalid_permission1"),
      _ => panic!("Expected InvalidPermission error"),
    }
  }

  #[tokio::test]
  async fn test_create_role_all_valid_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    let all_permissions: Vec<String> = crate::models::ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let create_data = CreateRoleData {
      name: "all_permissions_role".to_string(),
      permissions: all_permissions.clone(),
      is_default: false,
    };

    let role = RoleService::create_role(&pool, create_data).await.unwrap();

    assert_eq!(role.name, "all_permissions_role");
    assert!(!role.is_default);

    let permissions = role.permissions();
    assert_eq!(permissions.len(), all_permissions.len());

    // Verify all permissions are present
    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }

  #[tokio::test]
  async fn test_create_role_default_role() {
    let (pool, _temp_file) = create_test_database().await;

    let create_data = CreateRoleData {
      name: "default_test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: true,
    };

    let role = RoleService::create_role(&pool, create_data).await.unwrap();

    assert_eq!(role.name, "default_test_role");
    assert!(!role.is_default); // Always false for now
    assert_eq!(role.permissions().len(), 1);
    assert!(role
      .permissions()
      .contains(&"can_view_user_self".to_string()));
  }

  #[tokio::test]
  async fn test_delete_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role first
    let create_data = CreateRoleData {
      name: "test-delete-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = RoleService::create_role(&pool, create_data).await.unwrap();

    // Delete the role
    let deleted_role = RoleService::delete_role(&pool, role.id).await.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_role.id, role.id);
    assert_eq!(deleted_role.name, role.name);
    assert_eq!(deleted_role.created_ts, role.created_ts);
    assert_eq!(deleted_role.updated_ts, role.updated_ts);
    assert_eq!(deleted_role.is_default, role.is_default);

    // Verify role is deleted from database
    let result = RoleService::get_role_by_id(&pool, role.id).await.unwrap();
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_delete_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Try to delete non-existent role
    let result = RoleService::delete_role(&pool, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound error"),
    }
  }

  #[tokio::test]
  async fn test_delete_role_in_use() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a role
    let role_data = CreateRoleData {
      name: "test-in-use-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = RoleService::create_role(&pool, role_data).await.unwrap();

    // Create a user with this role
    let user_data = crate::queries::users::CreateUserData {
      uuid: uuid::Uuid::new_v4().to_string(),
      name: "test-user".to_string(),
      password_hash: "hashed_password".to_string(),
      role_id: Some(role.id),
      metadata_json: None,
    };
    crate::queries::users::CreateUserQuery::run(&pool, user_data)
      .await
      .unwrap();

    // Try to delete the role while it's in use
    let result = RoleService::delete_role(&pool, role.id).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleInUse(id) => assert_eq!(id, role.id),
      _ => panic!("Expected RoleInUse error"),
    }
  }

  #[tokio::test]
  async fn test_set_default_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create two roles
    let role1_data = CreateRoleData {
      name: "role1".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };
    let role1 = RoleService::create_role(&pool, role1_data).await.unwrap();

    let role2_data = CreateRoleData {
      name: "role2".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };
    let role2 = RoleService::create_role(&pool, role2_data).await.unwrap();

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role1 as default
    let updated_role1 = RoleService::set_default_role(&pool, role1.id)
      .await
      .unwrap();

    assert_eq!(updated_role1.id, role1.id);
    assert_eq!(updated_role1.name, "role1");
    assert!(updated_role1.is_default);
    assert!(updated_role1.updated_ts > role1.updated_ts);

    // Verify role1 is now default
    let default_role = GetDefaultRoleQuery::run(&pool).await.unwrap();
    assert!(default_role.is_some());
    assert_eq!(default_role.unwrap().id, role1.id);

    // Add delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Set role2 as default (should unset role1)
    let updated_role2 = RoleService::set_default_role(&pool, role2.id)
      .await
      .unwrap();

    assert_eq!(updated_role2.id, role2.id);
    assert_eq!(updated_role2.name, "role2");
    assert!(updated_role2.is_default);

    // Verify role2 is now default and role1 is not
    let default_role = GetDefaultRoleQuery::run(&pool).await.unwrap();
    assert!(default_role.is_some());
    assert_eq!(default_role.unwrap().id, role2.id);

    let current_role1 = RoleService::get_role_by_id(&pool, role1.id).await.unwrap();
    assert!(current_role1.is_some());
    assert!(!current_role1.unwrap().is_default);
  }

  #[tokio::test]
  async fn test_set_default_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let result = RoleService::set_default_role(&pool, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      RoleError::RoleNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected RoleNotFound error"),
    }
  }

  #[tokio::test]
  async fn test_set_default_role_atomic_behavior() {
    let (pool, _temp_file) = create_test_database().await;

    // Create multiple roles
    let _role1_data = CreateRoleData {
      name: "atomic-role1".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };
    let _role1 = RoleService::create_role(&pool, _role1_data).await.unwrap();

    let role2_data = CreateRoleData {
      name: "atomic-role2".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };
    let role2 = RoleService::create_role(&pool, role2_data).await.unwrap();

    let role3_data = CreateRoleData {
      name: "atomic-role3".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role3 = RoleService::create_role(&pool, role3_data).await.unwrap();

    // Set role2 as default
    RoleService::set_default_role(&pool, role2.id)
      .await
      .unwrap();

    // Verify only role2 is default
    let all_roles = RoleService::get_all_roles(&pool).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role2.id);

    // Set role3 as default
    RoleService::set_default_role(&pool, role3.id)
      .await
      .unwrap();

    // Verify only role3 is default now
    let all_roles = RoleService::get_all_roles(&pool).await.unwrap();
    let default_roles: Vec<_> = all_roles.iter().filter(|r| r.is_default).collect();
    assert_eq!(default_roles.len(), 1);
    assert_eq!(default_roles[0].id, role3.id);

    // Verify role2 is no longer default
    let current_role2 = RoleService::get_role_by_id(&pool, role2.id).await.unwrap();
    assert!(current_role2.is_some());
    assert!(!current_role2.unwrap().is_default);
  }
}
