use crate::models::{role::Role, user::User, user::UserWithRole};
use sqlx::{Row, SqliteConnection};

pub struct GetAllUsersWithRolesQuery;

impl GetAllUsersWithRolesQuery {
  pub async fn run(conn: &mut SqliteConnection) -> Result<Vec<UserWithRole>, sqlx::Error> {
    let rows = sqlx::query(
      r#"
      SELECT 
        u.id, u.uuid, u.created_ts, u.updated_ts, u.name, u.role_id, u.password_hash, u.metadata_json,
        r.id as role_id,
        r.name as role_name,
        r.created_ts as role_created_ts,
        r.updated_ts as role_updated_ts,
        r.is_default as role_is_default,
        r.permissions_json as role_permissions_json
      FROM users u
      JOIN roles r ON u.role_id = r.id
      ORDER BY u.name
      "#
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut users_with_roles = Vec::new();
    for row in rows {
      let user = User {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        created_ts: row.try_get("created_ts")?,
        updated_ts: row.try_get("updated_ts")?,
        name: row.try_get("name")?,
        role_id: row.try_get("role_id")?,
        password_hash: row.try_get("password_hash")?,
        metadata_json: row.try_get("metadata_json")?,
      };

      let permissions_json: Option<String> = row.try_get("role_permissions_json")?;
      let permissions = Role::deserialize_permissions(&permissions_json);

      let role = Role {
        id: row.try_get("role_id")?,
        name: row.try_get("role_name")?,
        created_ts: row.try_get("role_created_ts")?,
        updated_ts: row.try_get("role_updated_ts")?,
        is_default: row.try_get("role_is_default")?,
        permissions,
      };

      users_with_roles.push(UserWithRole { user, role });
    }

    Ok(users_with_roles)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  #[tokio::test]
  async fn test_get_all_users_with_roles_empty() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    let result = GetAllUsersWithRolesQuery::run(&mut conn).await.unwrap();
    assert_eq!(result.len(), 0);
  }

  #[tokio::test]
  async fn test_get_all_users_with_roles_with_data() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles
    let admin_role = create_test_role_model_with_pool(&pool, "admin", &[], false).await;
    let user_role =
      create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;

    // Only acquire connection after pool usage
    // as this will empty the pool (size is 1 in tests)
    let mut conn = pool.acquire().await.unwrap();

    // Insert test users
    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind("user1-uuid")
      .bind("Alice")
      .bind(admin_role.id)
      .bind("hash1")
      .bind(None::<String>)
      .bind(1234567890)
      .bind(1234567890)
      .execute(&mut *conn)
      .await
      .unwrap();

    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind("user2-uuid")
      .bind("Bob")
      .bind(user_role.id)
      .bind("hash2")
      .bind(None::<String>)
      .bind(1234567890)
      .bind(1234567890)
      .execute(&mut *conn)
      .await
      .unwrap();

    let result = GetAllUsersWithRolesQuery::run(&mut conn).await.unwrap();
    assert_eq!(result.len(), 2);

    // Verify ordering by name
    assert_eq!(result[0].user.name, "Alice");
    assert_eq!(result[0].role.name, "admin");
    assert_eq!(result[1].user.name, "Bob");
    assert_eq!(result[1].role.name, "user");
  }

  #[tokio::test]
  async fn test_get_all_users_with_roles_joins_correctly() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    let test_role = create_test_role_model_with_pool(&pool, "test_role", &[], false).await;

    let mut conn = pool.acquire().await.unwrap();

    // Insert test user
    sqlx::query("INSERT INTO users (uuid, name, role_id, password_hash, metadata_json, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind("test-uuid")
      .bind("TestUser")
      .bind(test_role.id)
      .bind("test_hash")
      .bind("{\"test\": true}")
      .bind(1234567890)
      .bind(1234567890)
      .execute(&mut *conn)
      .await
      .unwrap();

    let result = GetAllUsersWithRolesQuery::run(&mut conn).await.unwrap();
    assert_eq!(result.len(), 1);

    let user_with_role = &result[0];
    assert_eq!(user_with_role.user.name, "TestUser");
    assert_eq!(user_with_role.user.uuid, "test-uuid");
    assert_eq!(user_with_role.user.role_id, 1);
    assert_eq!(user_with_role.role.name, "test_role");
    assert_eq!(user_with_role.user.password_hash, "test_hash");
    assert_eq!(
      user_with_role.user.metadata_json,
      Some("{\"test\": true}".to_string())
    );
  }
}
