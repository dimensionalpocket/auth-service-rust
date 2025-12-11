use crate::models::user::UserWithRole;
use sqlx::SqlitePool;

pub struct GetAllUsersWithRolesQuery;

impl GetAllUsersWithRolesQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Vec<UserWithRole>, sqlx::Error> {
    sqlx::query_as::<_, UserWithRole>(
      r#"
      SELECT 
        u.id, u.uuid, u.created_ts, u.updated_ts, u.name, u.role_id, u.password_hash, u.metadata_json,
        r.name as role_name
      FROM users u
      JOIN roles r ON u.role_id = r.id
      ORDER BY u.name
      "#
    )
    .fetch_all(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_all_users_with_roles_empty() {
    let (pool, _temp_file) = create_test_database().await;

    let result = GetAllUsersWithRolesQuery::run(&pool).await.unwrap();
    assert_eq!(result.len(), 0);
  }

  #[tokio::test]
  async fn test_get_all_users_with_roles_with_data() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[]')")
      .execute(&pool)
      .await
      .unwrap();

    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[]')")
      .execute(&pool)
      .await
      .unwrap();

    // Insert test users
    sqlx::query("INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user1-uuid', 1234567890, 1234567890, 'Alice', 1, 'hash1', NULL)")
      .execute(&pool)
      .await
      .unwrap();

    sqlx::query("INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user2-uuid', 1234567890, 1234567890, 'Bob', 2, 'hash2', NULL)")
      .execute(&pool)
      .await
      .unwrap();

    let result = GetAllUsersWithRolesQuery::run(&pool).await.unwrap();
    assert_eq!(result.len(), 2);

    // Verify ordering by name
    assert_eq!(result[0].user.name, "Alice");
    assert_eq!(result[0].role_name, "admin");
    assert_eq!(result[1].user.name, "Bob");
    assert_eq!(result[1].role_name, "user");
  }

  #[tokio::test]
  async fn test_get_all_users_with_roles_joins_correctly() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test role
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('test_role', 1234567890, 1234567890, FALSE, '[]')")
      .execute(&pool)
      .await
      .unwrap();

    // Insert test user
    sqlx::query("INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('test-uuid', 1234567890, 1234567890, 'TestUser', 1, 'test_hash', '{\"test\": true}')")
      .execute(&pool)
      .await
      .unwrap();

    let result = GetAllUsersWithRolesQuery::run(&pool).await.unwrap();
    assert_eq!(result.len(), 1);

    let user_with_role = &result[0];
    assert_eq!(user_with_role.user.name, "TestUser");
    assert_eq!(user_with_role.user.uuid, "test-uuid");
    assert_eq!(user_with_role.user.role_id, 1);
    assert_eq!(user_with_role.role_name, "test_role");
    assert_eq!(user_with_role.user.password_hash, "test_hash");
    assert_eq!(
      user_with_role.user.metadata_json,
      Some("{\"test\": true}".to_string())
    );
  }
}
