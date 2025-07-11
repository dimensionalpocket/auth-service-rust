use crate::models::user::User;
use crate::models::user_role::UserRole;
use crate::queries::user_roles::{GetRoleByIdQuery, GetRoleByNameQuery};
use sqlx::SqlitePool;

pub struct UserRoleService;

impl UserRoleService {
  /// Get a role by ID
  pub async fn get_role_by_id(pool: &SqlitePool, role_id: i64) -> Result<Option<UserRole>, sqlx::Error> {
    GetRoleByIdQuery::run(pool, role_id).await
  }

  /// Get a role by name
  pub async fn get_role_by_name(pool: &SqlitePool, name: &str) -> Result<Option<UserRole>, sqlx::Error> {
    GetRoleByNameQuery::run(pool, name).await
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
  /// * `Ok(false)` - User does not have the permission
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn check_user_permission(pool: &SqlitePool, user: &User, permission: &str) -> Result<bool, sqlx::Error> {
    let role = GetRoleByIdQuery::run(pool, user.role_id).await?;
    
    match role {
      Some(role) => {
        // Check if user is admin first - admins have all permissions
        if role.has_permission("is_admin") {
          Ok(true)
        } else {
          // For non-admin users, check the specific permission
          Ok(role.has_permission(permission))
        }
      },
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
    let result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\"]')")
      .execute(&pool)
      .await
      .unwrap();
    
    let role_id = result.last_insert_rowid();
    let role = UserRoleService::get_role_by_id(&pool, role_id).await.unwrap();
    
    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert!(role.has_permission("is_admin"));
  }

  #[tokio::test]
  async fn test_get_role_by_name_delegates_to_query() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Insert test role with permissions
    sqlx::query("INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('user', 1234567890, TRUE, '[\"can_view_user_self\"]')")
      .execute(&pool)
      .await
      .unwrap();
    
    let role = UserRoleService::get_role_by_name(&pool, "user").await.unwrap();
    
    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.has_permission("can_view_user_self"));
  }

  #[tokio::test]
  async fn test_check_user_permission_admin_has_all_permissions() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Create admin role
    let admin_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\"]')")
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
    
    // Admin should have any permission
    assert!(UserRoleService::check_user_permission(&pool, &admin_user, "any_permission").await.unwrap());
    assert!(UserRoleService::check_user_permission(&pool, &admin_user, "can_create_user").await.unwrap());
    assert!(UserRoleService::check_user_permission(&pool, &admin_user, "can_delete_user").await.unwrap());
  }

  #[tokio::test]
  async fn test_check_user_permission_regular_user_specific_permissions() {
    let (pool, _temp_file) = create_test_database().await;
    
    // Create user role
    let user_role_result = sqlx::query("INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('user', 1234567890, TRUE, '[\"can_view_user_self\", \"can_update_user_self\"]')")
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
    assert!(UserRoleService::check_user_permission(&pool, &regular_user, "can_view_user_self").await.unwrap());
    assert!(UserRoleService::check_user_permission(&pool, &regular_user, "can_update_user_self").await.unwrap());
    
    // User should NOT have admin permissions
    assert!(!UserRoleService::check_user_permission(&pool, &regular_user, "can_create_user").await.unwrap());
    assert!(!UserRoleService::check_user_permission(&pool, &regular_user, "is_admin").await.unwrap());
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
    assert!(!UserRoleService::check_user_permission(&pool, &user_no_role, "any_permission").await.unwrap());
  }
}