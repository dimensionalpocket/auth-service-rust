use crate::models::Role;
use sqlx::{Row, SqliteConnection};

#[derive(Debug)]
pub struct UpdateRoleData {
  pub id: i64,
  pub name: Option<String>,
  pub permissions: Option<Vec<String>>,
}

pub struct UpdateRoleQuery;

impl UpdateRoleQuery {
  pub async fn run(
    conn: &mut SqliteConnection,
    data: UpdateRoleData,
  ) -> Result<Option<Role>, sqlx::Error> {
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

    let result = query.execute(&mut *conn).await?;

    if result.rows_affected() == 0 {
      return Ok(None);
    }

    // Return updated role
    let row = sqlx::query(
            "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles WHERE id = ?"
        )
        .bind(data.id)
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
  use super::*;
  use crate::models::ROLE_PERMISSIONS;
  use crate::test_utils::{create_test_database, create_test_role_model};

  #[tokio::test]
  async fn test_update_role_success() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role_model(
      &pool,
      "test-role",
      &["can_view_user_self", "can_list_users"],
      false,
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

    let mut conn = pool.acquire().await.unwrap();
    let updated_role = UpdateRoleQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, "updated-role");
    assert_eq!(updated_role.permissions.len(), 2);
    assert!(updated_role
      .permissions
      .contains(&"can_edit_user".to_string()));
    assert!(updated_role
      .permissions
      .contains(&"can_delete_user".to_string()));
    assert!(updated_role.updated_ts > role.updated_ts);
    assert_eq!(updated_role.created_ts, role.created_ts);
  }

  #[tokio::test]
  async fn test_update_role_partial_update() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role_model(
      &pool,
      "partial-role",
      &["can_view_user_self", "can_list_users"],
      false,
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

    let mut conn = pool.acquire().await.unwrap();
    let updated_role = UpdateRoleQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, "partial-updated");
    assert_eq!(updated_role.permissions.len(), 2); // unchanged
    assert!(updated_role
      .permissions
      .contains(&"can_view_user_self".to_string()));
    assert!(updated_role
      .permissions
      .contains(&"can_list_users".to_string()));
    assert!(updated_role.updated_ts > role.updated_ts);
  }

  #[tokio::test]
  async fn test_update_role_set_permissions_to_empty() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role_model(
      &pool,
      "empty-permissions-role",
      &["can_view_user_self"],
      false,
    )
    .await;

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update permissions to empty array
    let update_data = UpdateRoleData {
      id: role.id,
      name: None,
      permissions: Some(vec![]), // Set to empty array
    };

    let mut conn = pool.acquire().await.unwrap();
    let updated_role = UpdateRoleQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, role.name); // unchanged
    assert_eq!(updated_role.permissions.len(), 0); // should be empty now
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

    let mut conn = pool.acquire().await.unwrap();
    let result = UpdateRoleQuery::run(&mut conn, update_data).await.unwrap();
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_update_role_no_changes() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role =
      create_test_role_model(&pool, "no-changes-role", &["can_view_user_self"], false).await;

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update with no actual changes (empty payload)
    let update_data = UpdateRoleData {
      id: role.id,
      name: None,
      permissions: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    let updated_role = UpdateRoleQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.id, role.id);
    assert_eq!(updated_role.name, role.name);
    assert_eq!(updated_role.permissions, role.permissions);
    assert!(updated_role.updated_ts > role.updated_ts); // updated_ts should still change
  }

  #[tokio::test]
  async fn test_update_role_with_all_valid_permissions() {
    let (pool, _tmp) = create_test_database().await;

    // Create a role first
    let role = create_test_role_model(
      &pool,
      "all-permissions-role",
      &["can_view_user_self"],
      false,
    )
    .await;

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

    let mut conn = pool.acquire().await.unwrap();
    let updated_role = UpdateRoleQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_role.name, "all-permissions-updated");
    assert_eq!(updated_role.permissions.len(), all_permissions.len());

    // Verify all permissions are present
    for permission in &all_permissions {
      assert!(
        updated_role.permissions.contains(permission),
        "Missing permission: {permission}"
      );
    }
  }
}
