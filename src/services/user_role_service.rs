use crate::models::user::User;
use crate::models::user_role::UserRole;
use crate::queries::user_roles::{
  GetAllRolesQuery, GetRoleByIdQuery, GetRoleByNameQuery, UpdateRoleData, UpdateRoleQuery,
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

pub struct UserRoleService;

impl UserRoleService {
  /// Get all roles
  pub async fn get_all_roles(pool: &SqlitePool) -> Result<Vec<UserRole>, RoleError> {
    GetAllRolesQuery::run(pool)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Get a role by ID
  pub async fn get_role_by_id(
    pool: &SqlitePool,
    role_id: i64,
  ) -> Result<Option<UserRole>, RoleError> {
    GetRoleByIdQuery::run(pool, role_id)
      .await
      .map_err(RoleError::DatabaseError)
  }

  /// Get a role by name
  pub async fn get_role_by_name(
    pool: &SqlitePool,
    name: &str,
  ) -> Result<Option<UserRole>, RoleError> {
    GetRoleByNameQuery::run(pool, name)
      .await
      .map_err(RoleError::DatabaseError)
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
  /// * `Ok(UserRole)` - Updated role
  /// * `Err(RoleError)` - Database error, role not found, or validation error
  pub async fn update_role(
    pool: &SqlitePool,
    role_id: i64,
    update_data: UpdateRoleData,
  ) -> Result<UserRole, RoleError> {
    // Validate permissions if provided
    if let Some(ref permissions) = update_data.permissions {
      for permission in permissions {
        if !crate::models::user_role::is_valid_role_permission(permission) {
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
    if !crate::models::user_role::is_valid_role_permission(permission) {
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
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_role_by_id_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role with permissions
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let role_id = result.last_insert_rowid();
    let role = UserRoleService::get_role_by_id(&pool, role_id)
      .await
      .unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert!(role.has_permission("is_admin"));
  }

  #[tokio::test]
  async fn test_get_role_by_name_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role with permissions
    sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let role = UserRoleService::get_role_by_name(&pool, "user")
      .await
      .unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_get_all_roles_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with different permissions
    sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_manage_roles\"]'), ('user', 1234567891, 1234567891, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();

    let roles = UserRoleService::get_all_roles(&pool).await.unwrap();

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

    let roles = UserRoleService::get_all_roles(&pool).await.unwrap();

    assert_eq!(roles.len(), 0);
  }

  #[tokio::test]
  async fn test_check_user_permission_admin_has_all_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
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
      UserRoleService::check_user_permission(&pool, &admin_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      UserRoleService::check_user_permission(&pool, &admin_user, "can_create_site")
        .await
        .unwrap()
    );
    assert!(
      UserRoleService::check_user_permission(&pool, &admin_user, "can_delete_user")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_regular_user_specific_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role
    let user_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')")
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
      UserRoleService::check_user_permission(&pool, &regular_user, "can_view_user_self")
        .await
        .unwrap()
    );

    // User should NOT have admin permissions
    assert!(
      !UserRoleService::check_user_permission(&pool, &regular_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      !UserRoleService::check_user_permission(&pool, &regular_user, "is_admin")
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
      !UserRoleService::check_user_permission(&pool, &user_no_role, "can_list_users")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_invalid_permission() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
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
      !UserRoleService::check_user_permission(&pool, &admin_user, "invalid_permission")
        .await
        .unwrap()
    );
    assert!(
      !UserRoleService::check_user_permission(&pool, &admin_user, "")
        .await
        .unwrap()
    );
    assert!(!UserRoleService::check_user_permission(
      &pool,
      &admin_user,
      "nonexistent_can_permission"
    )
    .await
    .unwrap());
  }

  #[tokio::test]
  async fn test_check_user_permission_valid_new_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role with new permissions
    let role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('role_manager', 1234567890, 1234567890, FALSE, '[\"can_edit_user_role\", \"can_manage_roles\"]')")
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
    assert!(UserRoleService::check_user_permission(
      &pool,
      &role_manager_user,
      "can_edit_user_role"
    )
    .await
    .unwrap());
    assert!(
      UserRoleService::check_user_permission(&pool, &role_manager_user, "can_manage_roles")
        .await
        .unwrap()
    );

    // User should NOT have admin management permission
    assert!(!UserRoleService::check_user_permission(
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
    let admin_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\"]')")
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
      UserRoleService::check_user_permission(&pool, &admin_user, "can_edit_user_role")
        .await
        .unwrap()
    );
    assert!(
      UserRoleService::check_user_permission(&pool, &admin_user, "can_manage_roles")
        .await
        .unwrap()
    );
    assert!(UserRoleService::check_user_permission(
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('test-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
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

    let updated_role = UserRoleService::update_role(&pool, role_id, update_data)
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('partial-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\", \"can_list_users\"]')")
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

    let updated_role = UserRoleService::update_role(&pool, role_id, update_data)
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

    let result = UserRoleService::update_role(&pool, 999, update_data).await;
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('test-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
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

    let result = UserRoleService::update_role(&pool, role_id, update_data).await;
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('empty-permissions-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
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

    let updated_role = UserRoleService::update_role(&pool, role_id, update_data)
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('all-permissions-role', 1234567890, 1234567890, FALSE, '[\"can_view_user_self\"]')")
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

    let updated_role = UserRoleService::update_role(&pool, role_id, update_data)
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
}
