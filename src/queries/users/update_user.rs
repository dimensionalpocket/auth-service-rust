use crate::models::User;
use crate::queries::users::GetUserByIdQuery;
use sqlx::SqlitePool;

/// Data structure for updating user details with partial update support
#[derive(Debug)]
pub struct UpdateUserData {
  pub id: i64,
  pub name: Option<String>,
  pub role_id: Option<i64>, // role_id is NOT NULL in database, so only Option<i64> for partial updates
  pub password_hash: Option<String>,
  pub metadata_json: Option<Option<String>>, // Allows explicit NULL setting for metadata
}

/// Database query for updating user details with partial update support
pub struct UpdateUserQuery;

impl UpdateUserQuery {
  /// Update a user's details in database with partial update support
  ///
  /// This method updates only the fields provided in update_data.
  /// It uses dynamic SQL building to handle partial updates properly.
  /// It automatically updates the updated_ts timestamp.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `update_data` - The user data to update (partial fields only)
  ///
  /// # Returns
  /// * `Ok(User)` - The updated user with new timestamp
  /// * `Err(sqlx::Error)` - Database error if the update fails
  ///
  /// # Errors
  /// * Returns error if user_id doesn't exist
  /// * Returns error if database operation fails
  pub async fn run(pool: &SqlitePool, update_data: UpdateUserData) -> Result<User, sqlx::Error> {
    let current_timestamp = chrono::Utc::now().timestamp();

    // Start with base query
    let mut query_str = "UPDATE users SET updated_ts = ?".to_string();
    let mut has_updates = false;

    // Add each field if present
    if update_data.name.is_some() {
      query_str.push_str(", name = ?");
      has_updates = true;
    }

    if update_data.role_id.is_some() {
      query_str.push_str(", role_id = ?");
      has_updates = true;
    }

    if update_data.password_hash.is_some() {
      query_str.push_str(", password_hash = ?");
      has_updates = true;
    }

    if update_data.metadata_json.is_some() {
      query_str.push_str(", metadata_json = ?");
      has_updates = true;
    }

    // No updates requested
    if !has_updates {
      return GetUserByIdQuery::run(pool, update_data.id)
        .await?
        .ok_or(sqlx::Error::RowNotFound);
    }

    // Complete query string
    query_str.push_str(" WHERE id = ? RETURNING *");

    // Create and execute query with all bindings
    let mut query = sqlx::query_as::<_, User>(&query_str).bind(current_timestamp);

    if let Some(name) = &update_data.name {
      query = query.bind(name);
    }

    if let Some(role_id) = &update_data.role_id {
      query = query.bind(role_id);
    }

    if let Some(password_hash) = &update_data.password_hash {
      query = query.bind(password_hash);
    }

    // metadata_json is Option<Option<String>> - we need to flatten to Option<&str>
    if let Some(metadata_json) = &update_data.metadata_json {
      query = query.bind(metadata_json.as_deref());
    }

    query = query.bind(update_data.id);

    query.fetch_one(pool).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::PasswordService;

  async fn setup_default_role(pool: &SqlitePool) {
    sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('user', 1234567890, 1234567890, TRUE)",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  async fn setup_admin_role(pool: &SqlitePool) -> i64 {
    let result = sqlx::query(
      "INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)",
    )
    .execute(pool)
    .await
    .unwrap();
    result.last_insert_rowid()
  }

  #[tokio::test]
  async fn test_update_user_name_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only name
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("updateduser".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Name changed, other fields unchanged
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, "updateduser");
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_role_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user and admin role
    setup_default_role(&pool).await;
    let admin_role_id = setup_admin_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only role
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: Some(admin_role_id),
      password_hash: None,
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Role changed, other fields unchanged
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_password_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only password
    let new_password_hash = PasswordService::generate("newpassword456").unwrap();
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: Some(new_password_hash.clone()),
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Password hash changed, other fields unchanged
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_ne!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.password_hash, new_password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_multiple_fields() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user and admin role
    setup_default_role(&pool).await;
    let admin_role_id = setup_admin_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update multiple fields
    let new_password_hash = PasswordService::generate("newpassword456").unwrap();
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("updateduser".to_string()),
      role_id: Some(admin_role_id),
      password_hash: Some(new_password_hash.clone()),
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: All specified fields changed
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, "updateduser");
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(updated_user.password_hash, new_password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_role_to_different_role() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with default role and admin role
    setup_default_role(&pool).await;
    let admin_role_id = setup_admin_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None, // Will use default role (id=1)
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Change role to admin
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: Some(admin_role_id), // Change to admin role
      password_hash: None,
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Role changed to admin
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_no_fields() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update with no fields
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: User unchanged (except possibly timestamp)
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    // Note: updated_ts might be the same in fast test environments
  }

  #[tokio::test]
  async fn test_update_user_nonexistent_user() {
    let (pool, _temp_file) = create_test_database().await;

    // Test: Try to update non-existent user
    let update_data = UpdateUserData {
      id: 999,
      name: Some("nonexistent".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserQuery::run(&pool, update_data).await;

    // Verify: Should return error
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_update_user_partial_field_preservation() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with all fields
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: Some(r#"{"key": "value"}"#.to_string()),
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only name
    let update_data = UpdateUserData {
      id: user.id,
      name: Some("newname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Only name changed, metadata preserved
    assert_eq!(updated_user.name, "newname");
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_metadata_only() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with metadata
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: Some(r#"{"key": "value"}"#.to_string()),
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Update only metadata
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(Some(r#"{"updated": true}"#.to_string())),
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Only metadata changed, other fields preserved
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"updated": true}"#.to_string())
    );
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[tokio::test]
  async fn test_update_user_metadata_to_null() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create a user with metadata
    setup_default_role(&pool).await;
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: PasswordService::generate("password123").unwrap(),
      metadata_json: Some(r#"{"key": "value"}"#.to_string()),
    };
    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Test: Set metadata to NULL
    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(None), // Explicitly set to NULL
    };

    let updated_user = UpdateUserQuery::run(&pool, update_data).await.unwrap();

    // Verify: Metadata is now NULL
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.metadata_json, None); // Should be NULL
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }
}
