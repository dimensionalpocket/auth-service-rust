use crate::models::{role::Role, user::User, user::UserWithRole};
use sqlx::{Row, SqliteConnection};

/// Query to retrieve a single user with their role information by user ID
pub struct GetUserByIdWithRoleQuery;

impl GetUserByIdWithRoleQuery {
  /// Execute query to get a user with their role information
  ///
  /// # Arguments
  /// * `conn` - Database connection
  /// * `user_id` - The ID of user to retrieve
  ///
  /// # Returns
  /// * `Ok(Some(UserWithRole))` - User found with role information
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn run(
    conn: &mut SqliteConnection,
    user_id: i64,
  ) -> Result<Option<UserWithRole>, sqlx::Error> {
    let row = sqlx::query(
      r#"
      SELECT 
        u.id, u.uuid, u.created_ts, u.updated_ts, u.name, u.role_id, u.password_hash, u.metadata_json,
        r.id as role_id,
        r.name as role_name,
        r.created_ts as role_created_ts,
        r.updated_ts as role_updated_ts,
        r.is_default as role_is_default,
        r.permissions_json as role_permissions_json
      FROM users u
      JOIN roles r ON u.role_id = r.id
      WHERE u.id = ?
      "#
    )
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(row) = row {
      let user = User {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        created_ts: row.try_get("created_ts")?,
        updated_ts: row.try_get("updated_ts")?,
        name: row.try_get("name")?,
        role_id: row.try_get("role_id")?,
        password_hash: row.try_get("password_hash")?,
        metadata_json: row.try_get("metadata_json")?,
      };

      let permissions_json: Option<String> = row.try_get("role_permissions_json")?;
      let permissions = Role::deserialize_permissions(&permissions_json);

      let role = Role {
        id: row.try_get("role_id")?,
        name: row.try_get("role_name")?,
        created_ts: row.try_get("role_created_ts")?,
        updated_ts: row.try_get("role_updated_ts")?,
        is_default: row.try_get("role_is_default")?,
        permissions,
      };

      Ok(Some(UserWithRole { user, role }))
    } else {
      Ok(None)
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};

  #[dps_auth_db_test]
  async fn test_get_user_by_id_with_role_success() {
    // Create role and user
    let role_id = create_test_role_with_pool(&pool, "admin", &["can_view_user_details"]).await;
    let user = create_test_user_with_pool(&pool, "testuser", role_id).await;

    // Query user with role
    let mut conn = pool.acquire().await.unwrap();
    let result = GetUserByIdWithRoleQuery::run(&mut conn, user.id)
      .await
      .unwrap();

    assert!(result.is_some());
    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.id, user.id);
    assert_eq!(user_with_role.user.name, "testuser");
    assert_eq!(user_with_role.role.name, "admin");
    assert_eq!(user_with_role.user.role_id, role_id);
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_id_with_role_not_found() {
    // Query non-existent user
    let mut conn = pool.acquire().await.unwrap();
    let result = GetUserByIdWithRoleQuery::run(&mut conn, 999).await.unwrap();

    assert!(result.is_none());
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_id_with_role_different_roles() {
    // Create different roles
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &[]).await;
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    // Create users with different roles
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;
    let regular_user = create_test_user_with_pool(&pool, "regular", user_role_id).await;

    // Query admin user
    let mut conn = pool.acquire().await.unwrap();
    let admin_result = GetUserByIdWithRoleQuery::run(&mut conn, admin_user.id)
      .await
      .unwrap();
    assert!(admin_result.is_some());
    assert_eq!(admin_result.unwrap().role.name, "admin");

    // Query regular user
    let user_result = GetUserByIdWithRoleQuery::run(&mut conn, regular_user.id)
      .await
      .unwrap();
    assert!(user_result.is_some());
    assert_eq!(user_result.unwrap().role.name, "user");
  }

  #[dps_auth_db_test]
  async fn test_get_user_by_id_with_role_joined_correctly() {
    // Create role
    let role_id = create_test_role_with_pool(&pool, "test_role", &[]).await;

    // Create user
    let user = create_test_user_with_pool(&pool, "test_user", role_id).await;

    // Query user and verify all fields are populated correctly
    let mut conn = pool.acquire().await.unwrap();
    let result = GetUserByIdWithRoleQuery::run(&mut conn, user.id)
      .await
      .unwrap();
    assert!(result.is_some());

    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.id, user.id);
    assert_eq!(user_with_role.user.uuid, user.uuid);
    assert_eq!(user_with_role.user.name, user.name);
    assert_eq!(user_with_role.user.role_id, role_id);
    assert_eq!(user_with_role.role.name, "test_role");
    assert!(!user_with_role.user.password_hash.is_empty());
    assert_eq!(user_with_role.user.created_ts, user.created_ts);
    assert_eq!(user_with_role.user.updated_ts, user.updated_ts);
  }
}
