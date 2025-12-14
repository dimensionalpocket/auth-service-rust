use crate::models::{role::Role, user::User, user::UserWithRole};
use sqlx::{Row, SqlitePool};

/// Query to retrieve a single user with their role information by username
pub struct GetUserByNameWithRoleQuery;

impl GetUserByNameWithRoleQuery {
  /// Execute the query to get a user with their role information by username
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `name` - The username of the user to retrieve
  ///
  /// # Returns
  /// * `Ok(Some(UserWithRole))` - User found with role information
  /// * `Ok(None)` - User not found
  /// * `Err(sqlx::Error)` - Database error occurred
  pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<UserWithRole>, sqlx::Error> {
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
      WHERE u.name = ? COLLATE NOCASE
      "#
    )
    .bind(name)
    .fetch_optional(pool)
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
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role, create_test_user};

  #[tokio::test]
  async fn test_get_user_by_name_with_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let role_id = create_test_role(&pool, "admin", &["can_view_user_details"]).await;
    let user = create_test_user(&pool, "testuser", role_id).await;

    // Query user with role
    let result = GetUserByNameWithRoleQuery::run(&pool, "testuser")
      .await
      .unwrap();

    assert!(result.is_some());
    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.id, user.id);
    assert_eq!(user_with_role.user.name, "testuser");
    assert_eq!(user_with_role.role.name, "admin");
    assert_eq!(user_with_role.user.role_id, role_id);
  }

  #[tokio::test]
  async fn test_get_user_by_name_with_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Query non-existent user
    let result = GetUserByNameWithRoleQuery::run(&pool, "nonexistent")
      .await
      .unwrap();

    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_get_user_by_name_with_role_case_insensitive() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role and user
    let role_id = create_test_role(&pool, "user", &[]).await;
    create_test_user(&pool, "TestUser", role_id).await;

    // Query with different cases
    let result_lower = GetUserByNameWithRoleQuery::run(&pool, "testuser")
      .await
      .unwrap();
    let result_upper = GetUserByNameWithRoleQuery::run(&pool, "TESTUSER")
      .await
      .unwrap();
    let result_mixed = GetUserByNameWithRoleQuery::run(&pool, "tEsTuSeR")
      .await
      .unwrap();

    assert!(result_lower.is_some());
    assert!(result_upper.is_some());
    assert!(result_mixed.is_some());

    assert_eq!(result_lower.unwrap().role.name, "user");
    assert_eq!(result_upper.unwrap().role.name, "user");
    assert_eq!(result_mixed.unwrap().role.name, "user");
  }

  #[tokio::test]
  async fn test_get_user_by_name_with_role_permissions() {
    let (pool, _temp_file) = create_test_database().await;

    // Create role with permissions and user
    let role_id = create_test_role(&pool, "admin", &["is_admin", "can_view_user_details"]).await;
    let _user = create_test_user(&pool, "adminuser", role_id).await;

    // Query user with role
    let result = GetUserByNameWithRoleQuery::run(&pool, "adminuser")
      .await
      .unwrap();

    assert!(result.is_some());
    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.name, "adminuser");
    assert_eq!(user_with_role.role.name, "admin");
    assert_eq!(user_with_role.role.permissions.len(), 2);
    assert!(user_with_role
      .role
      .permissions
      .contains(&"is_admin".to_string()));
    assert!(user_with_role
      .role
      .permissions
      .contains(&"can_view_user_details".to_string()));
  }
}
