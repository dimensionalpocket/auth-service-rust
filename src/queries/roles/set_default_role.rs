use crate::models::Role;
use sqlx::{Connection, Row, SqliteConnection};

pub struct SetDefaultRoleQuery;

impl SetDefaultRoleQuery {
  pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, sqlx::Error> {
    // Use a transaction for atomic operation
    let mut tx = conn.begin().await?;

    // First verify the role exists
    let role_exists = sqlx::query("SELECT COUNT(*) FROM roles WHERE id = ?")
      .bind(role_id)
      .fetch_one(&mut *tx)
      .await?
      .get::<i64, _>(0);

    if role_exists == 0 {
      tx.rollback().await?;
      return Ok(None);
    }

    // Set all currently default roles to non-default
    sqlx::query("UPDATE roles SET is_default = FALSE WHERE is_default = TRUE")
      .execute(&mut *tx)
      .await?;

    // Set target role as default and update timestamp
    let now = chrono::Utc::now().timestamp();

    sqlx::query("UPDATE roles SET is_default = TRUE, updated_ts = ? WHERE id = ?")
      .bind(now)
      .bind(role_id)
      .execute(&mut *tx)
      .await?;

    tx.commit().await?;

    // Return updated role
    let row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_one(&mut *conn)
        .await?;

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
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::test_utils::*;

  #[dps_auth_db_test]
  async fn test_set_default_role_success() {
    // Create test roles
    let role1_id = create_test_role_with_pool(&pool, "role1", &[]).await;
    let role2_id = create_test_role_with_pool(&pool, "role2", &[]).await;

    // Set role1 as default
    let mut conn = pool.acquire().await.unwrap();
    let result = SetDefaultRoleQuery::run(&mut conn, role1_id).await.unwrap();

    assert!(result.is_some());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, role1_id);
    assert!(updated_role.is_default);

    // Verify role2 is not default
    let role2_row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role2_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let permissions_json: Option<String> = role2_row.try_get("permissions_json").unwrap();
    let permissions = match permissions_json {
      Some(json) => serde_json::from_str(&json).unwrap_or_else(|_| vec![]),
      None => vec![],
    };
    let role2_updated = Role {
      id: role2_row.try_get("id").unwrap(),
      name: role2_row.try_get("name").unwrap(),
      created_ts: role2_row.try_get("created_ts").unwrap(),
      updated_ts: role2_row.try_get("updated_ts").unwrap(),
      is_default: role2_row.try_get("is_default").unwrap(),
      permissions,
    };
    assert!(!role2_updated.is_default);
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_atomic_behavior() {
    // Create test roles
    let role1_id = create_test_role_with_pool(&pool, "role1", &[]).await;
    let role2_id = create_test_role_with_pool(&pool, "role2", &[]).await;

    // Set role1 as default first
    let mut conn = pool.acquire().await.unwrap();
    SetDefaultRoleQuery::run(&mut conn, role1_id).await.unwrap();

    // Then set role2 as default
    let result = SetDefaultRoleQuery::run(&mut conn, role2_id).await.unwrap();

    assert!(result.is_some());
    let updated_role2 = result.unwrap();
    assert_eq!(updated_role2.id, role2_id);
    assert!(updated_role2.is_default);

    // Verify role1 is no longer default
    let role1_row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role1_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let permissions_json: Option<String> = role1_row.try_get("permissions_json").unwrap();
    let permissions = match permissions_json {
      Some(json) => serde_json::from_str(&json).unwrap_or_else(|_| vec![]),
      None => vec![],
    };
    let role1_updated = Role {
      id: role1_row.try_get("id").unwrap(),
      name: role1_row.try_get("name").unwrap(),
      created_ts: role1_row.try_get("created_ts").unwrap(),
      updated_ts: role1_row.try_get("updated_ts").unwrap(),
      is_default: role1_row.try_get("is_default").unwrap(),
      permissions,
    };
    assert!(!role1_updated.is_default);

    // Verify only one default role exists
    let default_count = sqlx::query("SELECT COUNT(*) FROM roles WHERE is_default = TRUE")
      .fetch_one(&mut *conn)
      .await
      .unwrap()
      .get::<i64, _>(0);
    assert_eq!(default_count, 1);
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_not_found() {
    // Try to set non-existent role as default
    let mut conn = pool.acquire().await.unwrap();
    let result = SetDefaultRoleQuery::run(&mut conn, 999).await.unwrap();

    assert!(result.is_none());
  }

  #[dps_auth_db_test]
  async fn test_set_default_role_timestamp_update() {
    // Create test role
    let role_id = create_test_role_with_pool(&pool, "role1", &[]).await;

    // Get original role to check timestamp
    let original_role_row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let permissions_json: Option<String> = original_role_row.try_get("permissions_json").unwrap();
    let permissions = match permissions_json {
      Some(json) => serde_json::from_str(&json).unwrap_or_else(|_| vec![]),
      None => vec![],
    };
    let original_role = Role {
      id: original_role_row.try_get("id").unwrap(),
      name: original_role_row.try_get("name").unwrap(),
      created_ts: original_role_row.try_get("created_ts").unwrap(),
      updated_ts: original_role_row.try_get("updated_ts").unwrap(),
      is_default: original_role_row.try_get("is_default").unwrap(),
      permissions,
    };
    let original_updated_ts = original_role.updated_ts;

    // Wait a bit to ensure timestamp difference (1+ seconds for timestamp in seconds)
    tokio::time::sleep(tokio::time::Duration::from_millis(1050)).await;

    // Set role as default
    let mut conn = pool.acquire().await.unwrap();
    let result = SetDefaultRoleQuery::run(&mut conn, role_id).await.unwrap();

    assert!(result.is_some());
    let updated_role = result.unwrap();
    assert!(updated_role.updated_ts > original_updated_ts);
  }
}
