use crate::models::User;
use crate::queries::roles::GetDefaultRoleQuery;
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct CreateUserData {
  pub uuid: String,
  pub name: String,
  pub role_id: Option<i64>,
  pub password_hash: String,
  pub metadata_json: Option<String>,
}

pub struct CreateUserQuery;

impl CreateUserQuery {
  pub async fn run(pool: &SqlitePool, data: CreateUserData) -> Result<User, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Determine the role_id to use
    let role_id = match data.role_id {
      Some(id) => id,
      None => {
        // Get the default role
        let default_role = GetDefaultRoleQuery::run(pool).await?;
        match default_role {
          Some(role) => role.id,
          None => {
            // Return a custom error if no default role exists
            return Err(sqlx::Error::RowNotFound);
          }
        }
      }
    };

    let result = sqlx::query(
      r#"
      INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json)
      VALUES (?, ?, ?, ?, ?, ?, ?)
      "#,
    )
    .bind(&data.uuid)
    .bind(now)
    .bind(now)
    .bind(&data.name)
    .bind(role_id)
    .bind(&data.password_hash)
    .bind(&data.metadata_json)
    .execute(pool)
    .await?;

    let user_id = result.last_insert_rowid();

    // Return the created user
    sqlx::query_as::<_, User>(
      "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role_model};
  use uuid::Uuid;

  #[tokio::test]
  async fn test_create_user_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role = create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;
    let role_id = role.id;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
      uuid: user_uuid.clone(),
      name: "Test User".to_string(),
      role_id: Some(role_id),
      password_hash: "hashed_password".to_string(),
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };

    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test User");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.metadata_json, Some(r#"{"test": true}"#.to_string()));
    assert!(user.created_ts > 0);
    assert_eq!(user.created_ts, user.updated_ts);
  }

  #[tokio::test]
  async fn test_create_user_duplicate_uuid_fails() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role first
    let role = create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;
    let role_id = role.id;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data1 = CreateUserData {
      uuid: user_uuid.clone(),
      name: "Test User 1".to_string(),
      role_id: Some(role_id),
      password_hash: "hashed_password1".to_string(),
      metadata_json: None,
    };

    // First user should succeed
    CreateUserQuery::run(&pool, create_data1).await.unwrap();

    // Second user with same UUID should fail
    let create_data2 = CreateUserData {
      uuid: user_uuid,
      name: "Test User 2".to_string(),
      role_id: Some(role_id),
      password_hash: "hashed_password2".to_string(),
      metadata_json: None,
    };

    let result = CreateUserQuery::run(&pool, create_data2).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_create_user_invalid_role_id_fails() {
    let (pool, _temp_file) = create_test_database().await;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
      uuid: user_uuid,
      name: "Test User".to_string(),
      role_id: Some(999), // Non-existent role
      password_hash: "hashed_password".to_string(),
      metadata_json: None,
    };

    let result = CreateUserQuery::run(&pool, create_data).await;
    assert!(result.is_err()); // Should fail due to foreign key constraint
  }

  #[tokio::test]
  async fn test_create_user_with_default_role() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with one default
    create_test_role_model(&pool, "admin", &["is_admin"], false).await;
    create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
      uuid: user_uuid.clone(),
      name: "Test User".to_string(),
      role_id: None, // Use default role
      password_hash: "hashed_password".to_string(),
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };

    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test User");
    // Should have the default role (user role)
    let default_role = GetDefaultRoleQuery::run(&pool).await.unwrap().unwrap();
    assert_eq!(user.role_id, default_role.id);
  }

  #[tokio::test]
  async fn test_create_user_with_explicit_role() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with one default
    let admin_role = create_test_role_model(&pool, "admin", &["is_admin"], false).await;
    let admin_role_id = admin_role.id;
    create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
      uuid: user_uuid.clone(),
      name: "Test Admin".to_string(),
      role_id: Some(admin_role_id), // Explicitly specify admin role
      password_hash: "hashed_password".to_string(),
      metadata_json: None,
    };

    let user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.name, "Test Admin");
    assert_eq!(user.role_id, admin_role_id); // Should have admin role, not default
  }

  #[tokio::test]
  async fn test_create_user_no_default_role_fails() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with no default
    create_test_role_model(&pool, "admin", &["is_admin"], false).await;
    create_test_role_model(&pool, "moderator", &["can_moderate"], false).await;

    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
      uuid: user_uuid,
      name: "Test User".to_string(),
      role_id: None, // Try to use default role, but none exists
      password_hash: "hashed_password".to_string(),
      metadata_json: None,
    };

    let result = CreateUserQuery::run(&pool, create_data).await;
    assert!(result.is_err()); // Should fail because no default role exists
  }
}
