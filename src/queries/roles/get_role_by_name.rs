use crate::models::Role;
use sqlx::{Row, SqliteConnection};

pub struct GetRoleByNameQuery;

impl GetRoleByNameQuery {
  pub async fn run(conn: &mut SqliteConnection, name: &str) -> Result<Option<Role>, sqlx::Error> {
    let row = sqlx::query(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(row) = row {
      let permissions_json: Option<String> = row.try_get("permissions_json")?;
      let permissions = Role::deserialize_permissions(&permissions_json);

      Ok(Some(Role {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        created_ts: row.try_get("created_ts")?,
        updated_ts: row.try_get("updated_ts")?,
        is_default: row.try_get("is_default")?,
        permissions,
      }))
    } else {
      Ok(None)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role_model};

  #[tokio::test]
  async fn test_get_role_by_name_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)")
      .execute(&pool)
      .await
      .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let role = GetRoleByNameQuery::run(&mut conn, "admin").await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "admin");
    assert_eq!(role.created_ts, 1234567890);
    assert!(!role.is_default);
  }

  #[tokio::test]
  async fn test_get_role_by_name_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let role = GetRoleByNameQuery::run(&mut conn, "nonexistent")
      .await
      .unwrap();

    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_role_by_name_case_sensitive() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    create_test_role_model(&pool, "admin", &["is_admin"], false).await;

    // Should not find with different case
    let mut conn = pool.acquire().await.unwrap();
    let role = GetRoleByNameQuery::run(&mut conn, "ADMIN").await.unwrap();
    assert!(role.is_none());
  }
}
