use crate::models::Role;
use sqlx::{Row, SqliteConnection};

pub struct GetDefaultRoleQuery;

impl GetDefaultRoleQuery {
  pub async fn run(conn: &mut SqliteConnection) -> Result<Option<Role>, sqlx::Error> {
    let row = sqlx::query(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE is_default = TRUE LIMIT 1"
    )
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
  async fn test_get_default_role_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with one default
    create_test_role_model(&pool, "admin", &["is_admin"], false).await;
    create_test_role_model(&pool, "user", &["can_view_user_self"], true).await;

    let mut conn = pool.acquire().await.unwrap();
    let role = GetDefaultRoleQuery::run(&mut conn).await.unwrap();

    assert!(role.is_some());
    let role = role.unwrap();
    assert_eq!(role.name, "user");
    assert!(role.is_default);
  }

  #[tokio::test]
  async fn test_get_default_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles with no default
    create_test_role_model(&pool, "admin", &["is_admin"], false).await;
    create_test_role_model(&pool, "moderator", &["can_moderate"], false).await;

    let mut conn = pool.acquire().await.unwrap();
    let role = GetDefaultRoleQuery::run(&mut conn).await.unwrap();

    assert!(role.is_none());
  }

  #[tokio::test]
  async fn test_get_default_role_empty_table() {
    let (pool, _temp_file) = create_test_database().await;

    let mut conn = pool.acquire().await.unwrap();
    let role = GetDefaultRoleQuery::run(&mut conn).await.unwrap();

    assert!(role.is_none());
  }
}
