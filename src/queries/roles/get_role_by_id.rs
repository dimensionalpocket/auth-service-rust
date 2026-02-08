use crate::models::Role;
use sqlx::{Row, SqliteConnection};

pub struct GetRoleByIdQuery;

impl GetRoleByIdQuery {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, sqlx::Error> {
    let row = sqlx::query(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?",
    )
    .bind(role_id)
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
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  #[dps_auth_db_test]
  async fn test_get_role_by_id_found() {
    let mut conn = pool.acquire().await.unwrap();

    // Insert test role
    let role_id = sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE)")
      .execute(&mut *conn)
      .await
      .unwrap()
      .last_insert_rowid();
    let role = GetRoleByIdQuery::run(&mut conn, role_id).await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.id, role_id);
    assert_eq!(role.name, "admin");
    assert_eq!(role.created_ts, 1234567890);
    assert!(!role.is_default);
  }

  #[dps_auth_db_test]
  async fn test_get_role_by_id_not_found() {
    let mut conn = pool.acquire().await.unwrap();

    let role = GetRoleByIdQuery::run(&mut conn, 999).await.unwrap();

    assert!(role.is_none());
  }
}
