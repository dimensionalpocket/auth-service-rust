use crate::models::user::UserWithRole;
use sqlx::SqlitePool;

/// Query to retrieve a single user with their role information by user ID
pub struct GetUserByIdWithRoleQuery;

impl GetUserByIdWithRoleQuery {
  /// Execute the query to get a user with their role information
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `user_id` - The ID of the user to retrieve
  ///
  /// # Returns
  /// * `Ok(Some(UserWithRole))` - User found with role information
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn run(pool: &SqlitePool, user_id: i64) -> Result<Option<UserWithRole>, sqlx::Error> {
    let result = sqlx::query_as::<_, UserWithRole>(
      r#"
      SELECT 
        u.id, u.uuid, u.created_ts, u.updated_ts, u.name, u.role_id, u.password_hash, u.metadata_json,
        r.name as role_name
      FROM users u
      JOIN roles r ON u.role_id = r.id
      WHERE u.id = ?
      "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role, create_test_user};

  #[tokio::test]
  async fn test_get_user_by_id_with_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let role_id = create_test_role(&pool, "admin", &["can_view_user_details"]).await;
    let user = create_test_user(&pool, "testuser", role_id).await;

    // Query user with role
    let result = GetUserByIdWithRoleQuery::run(&pool, user.id).await.unwrap();

    assert!(result.is_some());
    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.id, user.id);
    assert_eq!(user_with_role.user.name, "testuser");
    assert_eq!(user_with_role.role_name, "admin");
    assert_eq!(user_with_role.user.role_id, role_id);
  }

  #[tokio::test]
  async fn test_get_user_by_id_with_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Query non-existent user
    let result = GetUserByIdWithRoleQuery::run(&pool, 999).await.unwrap();

    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_get_user_by_id_with_role_different_roles() {
    let (pool, _temp_file) = create_test_database().await;

    // Create different roles
    let admin_role_id = create_test_role(&pool, "admin", &[]).await;
    let user_role_id = create_test_role(&pool, "user", &[]).await;

    // Create users with different roles
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let regular_user = create_test_user(&pool, "regular", user_role_id).await;

    // Query admin user
    let admin_result = GetUserByIdWithRoleQuery::run(&pool, admin_user.id)
      .await
      .unwrap();
    assert!(admin_result.is_some());
    assert_eq!(admin_result.unwrap().role_name, "admin");

    // Query regular user
    let user_result = GetUserByIdWithRoleQuery::run(&pool, regular_user.id)
      .await
      .unwrap();
    assert!(user_result.is_some());
    assert_eq!(user_result.unwrap().role_name, "user");
  }

  #[tokio::test]
  async fn test_get_user_by_id_with_role_joined_correctly() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role
    let role_id = create_test_role(&pool, "test_role", &[]).await;

    // Create user
    let user = create_test_user(&pool, "test_user", role_id).await;

    // Query user and verify all fields are populated correctly
    let result = GetUserByIdWithRoleQuery::run(&pool, user.id).await.unwrap();
    assert!(result.is_some());

    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.id, user.id);
    assert_eq!(user_with_role.user.uuid, user.uuid);
    assert_eq!(user_with_role.user.name, user.name);
    assert_eq!(user_with_role.user.role_id, role_id);
    assert_eq!(user_with_role.role_name, "test_role");
    assert!(!user_with_role.user.password_hash.is_empty());
    assert_eq!(user_with_role.user.created_ts, user.created_ts);
    assert_eq!(user_with_role.user.updated_ts, user.updated_ts);
  }
}
