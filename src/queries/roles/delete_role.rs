use crate::models::Role;
use sqlx::SqlitePool;

pub struct DeleteRoleQuery;

impl DeleteRoleQuery {
  pub async fn run(pool: &SqlitePool, role_id: i64) -> Result<Role, sqlx::Error> {
    // First fetch the role to return its data
    let role = sqlx::query_as::<_, Role>(
      "SELECT id, created_ts, updated_ts, name, permissions_json, is_default FROM roles WHERE id = ?"
    )
    .bind(role_id)
    .fetch_one(pool)
    .await?;

    // Then delete the role
    let result = sqlx::query("DELETE FROM roles WHERE id = ?")
      .bind(role_id)
      .execute(pool)
      .await?;

    if result.rows_affected() == 0 {
      return Err(sqlx::Error::RowNotFound);
    }

    Ok(role)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::roles::{CreateRoleData, CreateRoleQuery};
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::test_utils::create_test_database;
  use sqlx::Row;

  #[tokio::test]
  async fn test_delete_role_success() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let create_data = CreateRoleData {
      name: "test-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = CreateRoleQuery::run(&pool, create_data).await.unwrap();

    // Delete the role and get returned data
    let deleted_role = DeleteRoleQuery::run(&pool, role.id).await.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_role.id, role.id);
    assert_eq!(deleted_role.name, role.name);
    assert_eq!(deleted_role.created_ts, role.created_ts);
    assert_eq!(deleted_role.updated_ts, role.updated_ts);
    assert_eq!(deleted_role.is_default, role.is_default);

    // Verify role is deleted from database
    let result = sqlx::query("SELECT COUNT(*) FROM roles WHERE id = ?")
      .bind(role.id)
      .fetch_one(&pool)
      .await
      .unwrap();
    let count: i64 = result.get(0);
    assert_eq!(count, 0);
  }

  #[tokio::test]
  async fn test_delete_role_not_found() {
    let (pool, _tmp) = create_test_database().await;

    // Try to delete non-existent role
    let result = DeleteRoleQuery::run(&pool, 999).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));
  }

  #[tokio::test]
  async fn test_delete_role_in_use() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role
    let role_data = CreateRoleData {
      name: "test-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = CreateRoleQuery::run(&pool, role_data).await.unwrap();

    // Create a user with this role (this will create a foreign key constraint)
    let user_data = CreateUserData {
      uuid: uuid::Uuid::new_v4().to_string(),
      name: "test-user".to_string(),
      password_hash: "hashed_password".to_string(),
      role_id: Some(role.id),
      metadata_json: None,
    };
    CreateUserQuery::run(&pool, user_data).await.unwrap();

    // Try to delete the role while it's in use - should fail due to foreign key constraint
    let result = DeleteRoleQuery::run(&pool, role.id).await;
    assert!(result.is_err());
    // The query should fail due to foreign key constraint
    match result.unwrap_err() {
      sqlx::Error::Database(db_err) => {
        // Foreign key constraint violation
        assert!(db_err.message().contains("FOREIGN KEY constraint failed"));
      }
      _ => panic!("Expected database error"),
    }
  }
}
