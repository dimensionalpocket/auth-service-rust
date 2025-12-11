use crate::models::Role;
use sqlx::SqlitePool;

pub struct GetAllRolesQuery;

impl GetAllRolesQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Vec<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
      "SELECT id, name, created_ts, updated_ts, is_default, permissions_json FROM roles ORDER BY name",
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
  async fn test_get_all_roles_returns_default_roles() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert test roles
    sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE), ('user', 1234567891, 1234567891, TRUE)")
      .execute(&pool)
      .await
      .unwrap();

    let roles = GetAllRolesQuery::run(&pool).await.unwrap();

    assert_eq!(roles.len(), 2);
    assert_eq!(roles[0].name, "admin"); // Ordered by name
    assert!(!roles[0].is_default);
    assert_eq!(roles[1].name, "user");
    assert!(roles[1].is_default);
  }

  #[tokio::test]
  async fn test_get_all_roles_empty_table() {
    let (pool, _temp_file) = create_test_database().await;

    let roles = GetAllRolesQuery::run(&pool).await.unwrap();

    assert_eq!(roles.len(), 0);
  }
}
