use crate::models::Role;
use sqlx::{Row, SqliteConnection};

pub struct DeleteRoleQuery;

impl DeleteRoleQuery {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Role, sqlx::Error> {
    // First fetch of role to return its data
    let row = sqlx::query(
      "SELECT id, created_ts, updated_ts, name, permissions_json, is_default FROM roles WHERE id = ?"
    )
    .bind(role_id)
    .fetch_one(&mut *conn)
    .await?;

    let permissions_json: Option<String> = row.try_get("permissions_json")?;
    let permissions = Role::deserialize_permissions(&permissions_json);

    let role = Role {
      id: row.try_get("id")?,
      created_ts: row.try_get("created_ts")?,
      updated_ts: row.try_get("updated_ts")?,
      name: row.try_get("name")?,
      is_default: row.try_get("is_default")?,
      permissions,
    };

    // Then delete the role
    let result = sqlx::query("DELETE FROM roles WHERE id = ?")
      .bind(role_id)
      .execute(&mut *conn)
      .await?;

    if result.rows_affected() == 0 {
      return Err(sqlx::Error::RowNotFound);
    }

    Ok(role)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::queries::roles::{CreateRoleData, CreateRoleQuery};
  use crate::queries::users::{CreateUserData, CreateUserQuery};

  use sqlx::Row;

  #[dps_auth_db_test]
  async fn test_delete_role_success() {
    // Create a role first
    let create_data = CreateRoleData {
      name: "test-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = {
      let mut conn = main_pool.acquire().await.unwrap();
      CreateRoleQuery::run(&mut conn, create_data).await.unwrap()
    };

    // Delete role and get returned data
    let mut conn = main_pool.acquire().await.unwrap();
    let deleted_role = DeleteRoleQuery::run(&mut conn, role.id).await.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_role.id, role.id);
    assert_eq!(deleted_role.name, role.name);
    assert_eq!(deleted_role.created_ts, role.created_ts);
    assert_eq!(deleted_role.updated_ts, role.updated_ts);
    assert_eq!(deleted_role.is_default, role.is_default);

    // Verify role is deleted from database
    let result = sqlx::query("SELECT COUNT(*) FROM roles WHERE id = ?")
      .bind(role.id)
      .fetch_one(&mut *conn)
      .await
      .unwrap();
    let count: i64 = result.get(0);
    assert_eq!(count, 0);
  }

  #[dps_auth_db_test]
  async fn test_delete_role_not_found() {
    // Try to delete non-existent role
    let mut conn = main_pool.acquire().await.unwrap();
    let result = DeleteRoleQuery::run(&mut conn, 999).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));
  }

  #[dps_auth_db_test]
  async fn test_delete_role_in_use() {
    // Create a role
    let role_data = CreateRoleData {
      name: "test-role".to_string(),
      permissions: vec!["can_manage_roles".to_string()],
      is_default: false,
    };
    let role = {
      let mut conn = main_pool.acquire().await.unwrap();
      CreateRoleQuery::run(&mut conn, role_data).await.unwrap()
    };

    // Create a user with this role (this will create a foreign key constraint)
    let user_data = CreateUserData {
      uuid: uuid::Uuid::new_v4().to_string(),
      name: "test-user".to_string(),
      password_hash: "hashed_password".to_string(),
      role_id: Some(role.id),
      metadata_json: None,
    };
    {
      let mut conn = main_pool.acquire().await.unwrap();
      CreateUserQuery::run(&mut conn, user_data).await.unwrap();
    }

    // Try to delete the role while it's in use - should fail due to foreign key constraint
    let mut conn = main_pool.acquire().await.unwrap();
    let result = DeleteRoleQuery::run(&mut conn, role.id).await;
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
