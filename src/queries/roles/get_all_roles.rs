use crate::models::Role;
use sqlx::{Row, SqliteConnection};

pub struct GetAllRolesQuery;

impl GetAllRolesQuery {
  pub async fn run(conn: &mut SqliteConnection) -> Result<Vec<Role>, sqlx::Error> {
    let rows = sqlx::query(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles ORDER BY name",
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut roles = Vec::new();
    for row in rows {
      let permissions_json: Option<String> = row.try_get("permissions_json")?;
      let permissions = Role::deserialize_permissions(&permissions_json);

      roles.push(Role {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        created_ts: row.try_get("created_ts")?,
        updated_ts: row.try_get("updated_ts")?,
        is_default: row.try_get("is_default")?,
        permissions,
      });
    }

    Ok(roles)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role_model};

  #[tokio::test]
  async fn test_get_all_roles_returns_default_roles() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Insert test roles
    create_test_role_model(&mut conn, "admin", &["is_admin"], false).await;
    create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;

    let roles = GetAllRolesQuery::run(&mut conn).await.unwrap();

    assert_eq!(roles.len(), 2);
    assert_eq!(roles[0].name, "admin"); // Ordered by name
    assert!(!roles[0].is_default);
    assert_eq!(roles[1].name, "user");
    assert!(roles[1].is_default);
  }

  #[tokio::test]
  async fn test_get_all_roles_empty_table() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    let roles = GetAllRolesQuery::run(&mut conn).await.unwrap();

    assert_eq!(roles.len(), 0);
  }
}
