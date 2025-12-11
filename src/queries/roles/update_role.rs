use crate::models::Role;
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct UpdateRoleData {
  pub id: i64,
  pub name: Option<String>,
  pub permissions: Option<Vec<String>>,
}

pub struct UpdateRoleQuery;

impl UpdateRoleQuery {
  pub async fn run(pool: &SqlitePool, data: UpdateRoleData) -> Result<Option<Role>, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Build the UPDATE query dynamically based on provided fields
    let mut set_clauses = vec!["updated_ts = ?".to_string()];

    // Add each field if provided
    if data.name.is_some() {
      set_clauses.push("name = ?".to_string());
    }

    if data.permissions.is_some() {
      set_clauses.push("permissions_json = ?".to_string());
    }

    // Construct the SQL
    let sql = format!("UPDATE roles SET {} WHERE id = ?", set_clauses.join(", "));

    // Execute the query with proper binding
    let mut query = sqlx::query(&sql).bind(now);

    // Bind each value in order
    if let Some(name) = data.name {
      query = query.bind(name);
    }

    if let Some(permissions) = data.permissions {
      let permissions_json = serde_json::to_string(&permissions).map_err(|e| {
        sqlx::Error::Io(std::io::Error::new(
          std::io::ErrorKind::InvalidData,
          format!("Failed to serialize permissions: {e}"),
        ))
      })?;
      query = query.bind(permissions_json);
    }

    query = query.bind(data.id);

    let result = query.execute(pool).await?;

    if result.rows_affected() == 0 {
      return Ok(None);
    }

    // Return updated role
    sqlx::query_as::<_, Role>(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(data.id)
        .fetch_one(pool)
        .await
        .map(Some)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::models::ROLE_PERMISSIONS;

  async fn create_test_role(pool: &SqlitePool, name: &str, permissions: Vec<&str>) -> Role {
    let now = chrono::Utc::now().timestamp();
    let permissions_json = serde_json::to_string(&permissions).unwrap();

    let result = sqlx::query(
      r#"
      INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json)
      VALUES (?, ?, ?, ?, ?)
      "#,
    )
    .bind(name)
    .bind(now)
    .bind(now)
    .bind(false)
    .bind(permissions_json)
    .execute(pool)
    .await
    .unwrap();

    let role_id = result.last_insert_rowid();

    sqlx::query_as::<_, Role>(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .unwrap()
  }

  #[tokio::test]
  async fn test_update_role_success() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role(
      &pool,
      "test-role",
      vec!["can_view_user_self", "can_list_users"],
    )
    .await;

    // Update the role
    let update_data = UpdateRoleData {
      id: role.id,
      name: Some("updated-role".to_string()),
      permissions: Some(vec![
        "can_edit_user".to_string(),
        "can_delete_user".to_string(),
      ]),
    };

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let updated_role = UpdateRoleQuery::run(&pool, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, "updated-role");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_edit_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
    assert!(updated_role.updated_ts > role.updated_ts);
    assert_eq!(updated_role.created_ts, role.created_ts);
  }

  #[tokio::test]
  async fn test_update_role_partial_update() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role(
      &pool,
      "partial-role",
      vec!["can_view_user_self", "can_list_users"],
    )
    .await;

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update only the name
    let update_data = UpdateRoleData {
      id: role.id,
      name: Some("partial-updated".to_string()),
      permissions: None,
    };

    let updated_role = UpdateRoleQuery::run(&pool, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, "partial-updated");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 2); // unchanged
    assert!(permissions.contains(&"can_view_user_self".to_string()));
    assert!(permissions.contains(&"can_list_users".to_string()));
    assert!(updated_role.updated_ts > role.updated_ts);
  }

  #[tokio::test]
  async fn test_update_role_set_permissions_to_empty() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role(&pool, "empty-permissions-role", vec!["can_view_user_self"]).await;

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update permissions to empty array
    let update_data = UpdateRoleData {
      id: role.id,
      name: None,
      permissions: Some(vec![]), // Set to empty array
    };

    let updated_role = UpdateRoleQuery::run(&pool, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, role.name); // unchanged
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), 0); // should be empty now
    assert!(updated_role.updated_ts > role.updated_ts);
  }

  #[tokio::test]
  async fn test_update_role_not_found() {
    let (pool, _tmp) = create_test_database().await;

    let update_data = UpdateRoleData {
      id: 999,
      name: Some("nonexistent".to_string()),
      permissions: None,
    };

    let result = UpdateRoleQuery::run(&pool, update_data).await.unwrap();
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_update_role_no_changes() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role(&pool, "no-changes-role", vec!["can_view_user_self"]).await;

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update with no actual changes (empty payload)
    let update_data = UpdateRoleData {
      id: role.id,
      name: None,
      permissions: None,
    };

    let updated_role = UpdateRoleQuery::run(&pool, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, role.name);
    assert_eq!(updated_role.permissions(), role.permissions());
    assert!(updated_role.updated_ts > role.updated_ts); // updated_ts should still change
  }

  #[tokio::test]
  async fn test_update_role_with_all_valid_permissions() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role(&pool, "all-permissions-role", vec!["can_view_user_self"]).await;

    // Update with all valid permissions
    let all_permissions: Vec<String> = ROLE_PERMISSIONS
      .iter()
      .map(|&perm| perm.to_string())
      .collect();

    let update_data = UpdateRoleData {
      id: role.id,
      name: Some("all-permissions-updated".to_string()),
      permissions: Some(all_permissions.clone()),
    };

    let updated_role = UpdateRoleQuery::run(&pool, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.name, "all-permissions-updated");
    let permissions = updated_role.permissions();
    assert_eq!(permissions.len(), all_permissions.len());

    // Verify all permissions are present
    for permission in &all_permissions {
      assert!(
        permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }
}
