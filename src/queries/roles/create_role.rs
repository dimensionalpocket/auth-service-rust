use crate::models::Role;
use serde_json;
use sqlx::SqliteConnection;

#[derive(Debug)]
pub struct CreateRoleData {
  pub name: String,
  pub permissions: Vec<String>,
  pub is_default: bool,
}

pub struct CreateRoleQuery;

impl CreateRoleQuery {
  pub async fn run(conn: &mut SqliteConnection, data: CreateRoleData) -> Result<Role, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    let permissions_json = if data.permissions.is_empty() {
      "[]".to_string()
    } else {
      serde_json::to_string(&data.permissions).map_err(|e| {
        sqlx::Error::Io(std::io::Error::new(
          std::io::ErrorKind::InvalidData,
          format!("Failed to serialize permissions: {e}"),
        ))
      })?
    };

    let result = sqlx::query(
      r#"
      INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json)
      VALUES (?, ?, ?, ?, ?)
      "#,
    )
    .bind(&data.name)
    .bind(now)
    .bind(now)
    .bind(data.is_default)
    .bind(permissions_json)
    .execute(&mut *conn)
    .await?;

    let role_id = result.last_insert_rowid();

    Ok(Role {
      id: role_id,
      name: data.name,
      created_ts: now,
      updated_ts: now,
      is_default: data.is_default,
      permissions: data.permissions,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_create_role_success() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateRoleData {
      name: "test_role".to_string(),
      permissions: vec![
        "can_view_user_self".to_string(),
        "can_list_users".to_string(),
      ],
      is_default: false,
    };

    let mut conn = pool.acquire().await.unwrap();
    let role = CreateRoleQuery::run(&mut conn, data).await.unwrap();

    assert_eq!(role.name, "test_role");
    assert!(!role.is_default);
    assert!(role.created_ts > 0);
    assert_eq!(role.created_ts, role.updated_ts);

    assert_eq!(role.permissions.len(), 2);
    assert!(role.permissions.contains(&"can_view_user_self".to_string()));
    assert!(role.permissions.contains(&"can_list_users".to_string()));
  }

  #[tokio::test]
  async fn test_create_role_empty_permissions() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateRoleData {
      name: "empty_permissions_role".to_string(),
      permissions: vec![],
      is_default: true,
    };

    let mut conn = pool.acquire().await.unwrap();
    let role = CreateRoleQuery::run(&mut conn, data).await.unwrap();

    assert_eq!(role.name, "empty_permissions_role");
    assert!(role.is_default);
    assert_eq!(role.permissions.len(), 0);
  }

  #[tokio::test]
  async fn test_create_role_duplicate_name_fails() {
    let (pool, _tmp) = create_test_database().await;

    let data1 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: false,
    };

    {
      let mut conn = pool.acquire().await.unwrap();
      CreateRoleQuery::run(&mut conn, data1).await.unwrap();
    }

    let data2 = CreateRoleData {
      name: "duplicate".to_string(),
      permissions: vec!["can_list_users".to_string()],
      is_default: false,
    };

    let mut conn = pool.acquire().await.unwrap();
    let result = CreateRoleQuery::run(&mut conn, data2).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_create_role_default_role() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateRoleData {
      name: "default_test_role".to_string(),
      permissions: vec!["can_view_user_self".to_string()],
      is_default: true,
    };

    let mut conn = pool.acquire().await.unwrap();
    let role = CreateRoleQuery::run(&mut conn, data).await.unwrap();

    assert_eq!(role.name, "default_test_role");
    assert!(role.is_default);
    assert_eq!(role.permissions.len(), 1);
    assert!(role.permissions.contains(&"can_view_user_self".to_string()));
  }

  #[tokio::test]
  async fn test_create_role_many_permissions() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateRoleData {
      name: "many_permissions_role".to_string(),
      permissions: vec![
        "is_admin".to_string(),
        "can_view_user_self".to_string(),
        "can_update_user_self".to_string(),
        "can_create_email_self".to_string(),
        "can_delete_email_self".to_string(),
        "can_list_users".to_string(),
        "can_view_user_details".to_string(),
        "can_delete_user".to_string(),
        "can_edit_user".to_string(),
        "can_create_site".to_string(),
        "can_delete_site".to_string(),
        "can_update_site".to_string(),
        "can_view_site_details".to_string(),
        "can_edit_user_role".to_string(),
        "can_manage_roles".to_string(),
        "can_manage_admin_role_permission".to_string(),
      ],
      is_default: false,
    };

    let mut conn = pool.acquire().await.unwrap();
    let role = CreateRoleQuery::run(&mut conn, data).await.unwrap();

    assert_eq!(role.name, "many_permissions_role");
    assert!(!role.is_default);

    assert_eq!(role.permissions.len(), 16);
    assert!(role.permissions.contains(&"is_admin".to_string()));
    assert!(role.permissions.contains(&"can_manage_roles".to_string()));
  }
}
