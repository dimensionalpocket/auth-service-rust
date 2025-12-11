use crate::models::Role;
use sqlx::{Row, SqlitePool};

pub struct SetDefaultRoleQuery;

impl SetDefaultRoleQuery {
  pub async fn run(pool: &SqlitePool, role_id: i64) -> Result<Option<Role>, sqlx::Error> {
    // Use a transaction for atomic operation
    let mut tx = pool.begin().await?;

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
    sqlx::query_as::<_, Role>(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_optional(pool)
        .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::*;

  #[tokio::test]
  async fn test_set_default_role_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create test roles
    let role1_id = create_test_role(&pool, "role1", &[]).await;
    let role2_id = create_test_role(&pool, "role2", &[]).await;

    // Set role1 as default
    let result = SetDefaultRoleQuery::run(&pool, role1_id).await.unwrap();

    assert!(result.is_some());
    let updated_role = result.unwrap();
    assert_eq!(updated_role.id, role1_id);
    assert!(updated_role.is_default);

    // Verify role2 is not default
    let role2_updated = sqlx::query_as::<_, Role>(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role2_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!role2_updated.is_default);
  }

  #[tokio::test]
  async fn test_set_default_role_atomic_behavior() {
    let (pool, _temp_file) = create_test_database().await;

    // Create test roles
    let role1_id = create_test_role(&pool, "role1", &[]).await;
    let role2_id = create_test_role(&pool, "role2", &[]).await;

    // Set role1 as default first
    SetDefaultRoleQuery::run(&pool, role1_id).await.unwrap();

    // Then set role2 as default
    let result = SetDefaultRoleQuery::run(&pool, role2_id).await.unwrap();

    assert!(result.is_some());
    let updated_role2 = result.unwrap();
    assert_eq!(updated_role2.id, role2_id);
    assert!(updated_role2.is_default);

    // Verify role1 is no longer default
    let role1_updated = sqlx::query_as::<_, Role>(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role1_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!role1_updated.is_default);

    // Verify only one default role exists
    let default_count = sqlx::query("SELECT COUNT(*) FROM roles WHERE is_default = TRUE")
      .fetch_one(&pool)
      .await
      .unwrap()
      .get::<i64, _>(0);
    assert_eq!(default_count, 1);
  }

  #[tokio::test]
  async fn test_set_default_role_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Try to set non-existent role as default
    let result = SetDefaultRoleQuery::run(&pool, 999).await.unwrap();

    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_set_default_role_timestamp_update() {
    let (pool, _temp_file) = create_test_database().await;

    // Create test role
    let role_id = create_test_role(&pool, "role1", &[]).await;

    // Get original role to check timestamp
    let original_role = sqlx::query_as::<_, Role>(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let original_updated_ts = original_role.updated_ts;

    // Wait a bit to ensure timestamp difference (1+ seconds for timestamp in seconds)
    tokio::time::sleep(tokio::time::Duration::from_millis(1050)).await;

    // Set role as default
    let result = SetDefaultRoleQuery::run(&pool, role_id).await.unwrap();

    assert!(result.is_some());
    let updated_role = result.unwrap();
    assert!(updated_role.updated_ts > original_updated_ts);
  }
}
