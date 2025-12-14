/*
 * Shared test utilities for creating test data.
 *
 * This module provides centralized functions for creating test users, roles, and other entities.
 * All helpers use existing Query objects to ensure consistency with application behavior.
 *
 * Do NOT create local create_test_* functions in test modules - use these shared utilities instead.
 */

#[cfg(any(test, feature = "test-utils"))]
use crate::database::Database;
use crate::models::{Role, User};
use crate::queries::roles::{CreateRoleData, CreateRoleQuery, GetDefaultRoleQuery};
use crate::queries::users::{CreateUserData, CreateUserQuery};
use crate::services::password_service::PasswordService;
use sqlx::SqlitePool;
use tempfile::NamedTempFile;
use uuid::Uuid;

// GraphQL test utilities
use crate::middleware::session::SessionContext;
use crate::DpsAuthApiConfig;
use async_graphql::{Object, ObjectType, Schema};

// Re-export GraphQL test utilities for consistent usage
pub use async_graphql::{EmptyMutation, EmptySubscription};

// Centralized TestEmptyQuery to replace all duplicates
#[derive(Default)]
pub struct TestEmptyQuery;

#[Object]
impl TestEmptyQuery {
  async fn dummy(&self) -> &str {
    "test"
  }
}

pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
  create_test_database_with_config(true).await
}

pub async fn create_test_database_with_pool_size(pool_size: u32) -> (SqlitePool, NamedTempFile) {
  create_test_database_with_config_and_pool_size(true, pool_size).await
}

pub async fn create_test_database_with_config(
  configure_sqlite: bool,
) -> (SqlitePool, NamedTempFile) {
  create_test_database_with_config_and_pool_size(configure_sqlite, 1).await
}

pub async fn create_test_database_with_config_and_pool_size(
  configure_sqlite: bool,
  pool_size: u32,
) -> (SqlitePool, NamedTempFile) {
  let temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let sqlite_file_path = temp_file.path().display().to_string();
  let database = Database::new_with_pool_size(&sqlite_file_path, Some(pool_size))
    .await
    .expect("Failed to create test database");
  let pool = database.pool;

  // Optionally configure SQLite settings
  if configure_sqlite {
    Database::configure_sqlite(&pool)
      .await
      .expect("Failed to configure SQLite");
  }

  // Run migrations
  let database = Database { pool: pool.clone() };
  database.migrate().await.expect("Failed to run migrations");

  (pool, temp_file)
}

/// Create a test user with default password
pub async fn create_test_user(pool: &SqlitePool, username: &str, role_id: i64) -> User {
  create_test_user_with_password(pool, username, role_id, "password123").await
}

/// Create a test user with custom password
pub async fn create_test_user_with_password(
  pool: &SqlitePool,
  username: &str,
  role_id: i64,
  password: &str,
) -> User {
  create_test_user_full(pool, username, Some(role_id), password, None).await
}

/// Create a test user with all parameters
pub async fn create_test_user_full(
  pool: &SqlitePool,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  // If no role_id specified, ensure a default role exists
  let final_role_id = if role_id.is_none() {
    match GetDefaultRoleQuery::run(pool).await.unwrap() {
      Some(default_role) => Some(default_role.id),
      None => {
        // Create a default role if none exists
        Some(
          create_test_role_model(pool, "user", &["can_view_user_self"], true)
            .await
            .id,
        )
      }
    }
  } else {
    role_id
  };

  let password_hash = PasswordService::generate(password).unwrap();
  let metadata_json_str = metadata_json.map(|v| v.to_string());
  let create_data = CreateUserData {
    uuid: Uuid::new_v4().to_string(),
    name: username.to_string(),
    role_id: final_role_id,
    password_hash,
    metadata_json: metadata_json_str,
  };
  CreateUserQuery::run(pool, create_data).await.unwrap()
}

/// Create a test role and return ID
pub async fn create_test_role(pool: &SqlitePool, name: &str, permissions: &[&str]) -> i64 {
  let role = create_test_role_model(pool, name, permissions, false).await;
  role.id
}

/// Create a test role and return full Role model
pub async fn create_test_role_model(
  pool: &SqlitePool,
  name: &str,
  permissions: &[&str],
  is_default: bool,
) -> Role {
  let create_data = CreateRoleData {
    name: name.to_string(),
    permissions: permissions.iter().map(|&p| p.to_string()).collect(),
    is_default,
  };
  CreateRoleQuery::run(pool, create_data).await.unwrap()
}

/// Create a test user via GraphQL mutation (for integration tests)
pub async fn create_test_user_via_mutation(
  app: &axum::Router,
  username: &str,
  password: &str,
) -> String {
  use axum::{
    body::Body,
    http::{Request, StatusCode},
  };
  use tower::ServiceExt;

  let query = format!(
    r#"{{
        "query": "mutation {{ authRegister(username: \"{username}\", password: \"{password}\", passwordConfirmation: \"{password}\") {{ uuid username }} }}"
      }}"#
  );

  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  let data: serde_json::Value = serde_json::from_str(&body_str).unwrap();

  // Verify user creation was successful
  assert!(data["errors"].is_null(), "User creation failed: {body_str}");
  assert!(!data["data"]["authRegister"]["uuid"].is_null());

  data["data"]["authRegister"]["uuid"]
    .as_str()
    .unwrap()
    .to_string()
}

// GraphQL test schema helper methods
/// For query tests - pass query directly
pub fn create_test_query_schema<Q>(
  query: Q,
  pool: Option<SqlitePool>,
  session: Option<SessionContext>,
  config: Option<DpsAuthApiConfig>,
) -> Schema<Q, EmptyMutation, EmptySubscription>
where
  Q: ObjectType + 'static,
{
  let mut schema_builder = Schema::build(query, EmptyMutation, EmptySubscription);

  if let Some(pool) = pool {
    schema_builder = schema_builder.data(pool);
  }

  if let Some(session) = session {
    schema_builder = schema_builder.data(session);
  }

  if let Some(config) = config {
    schema_builder = schema_builder.data(config);
  }

  schema_builder.finish()
}

/// For mutation tests - pass mutation directly
pub fn create_test_mutation_schema<M>(
  mutation: M,
  pool: Option<SqlitePool>,
  session: Option<SessionContext>,
  config: Option<DpsAuthApiConfig>,
) -> Schema<TestEmptyQuery, M, EmptySubscription>
where
  M: ObjectType + 'static,
{
  let mut schema_builder = Schema::build(TestEmptyQuery, mutation, EmptySubscription);

  if let Some(pool) = pool {
    schema_builder = schema_builder.data(pool);
  }

  if let Some(session) = session {
    schema_builder = schema_builder.data(session);
  }

  if let Some(config) = config {
    schema_builder = schema_builder.data(config);
  }

  schema_builder.finish()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::ROLE_PERMISSIONS;

  #[tokio::test]
  async fn test_create_test_user_no_role() {
    let (pool, _tmp) = create_test_database().await;

    // Create user without role (will create and use default role automatically)
    let user = create_test_user_full(&pool, "testuser", None, "password123", None).await;

    assert_eq!(user.name, "testuser");
    // Should have the default role ID (created automatically)
    assert!(user.role_id > 0);

    // Verify that default role is created automatically
    let default_role = GetDefaultRoleQuery::run(&pool).await.unwrap().unwrap();
    assert_eq!(user.role_id, default_role.id);
    assert_eq!(default_role.name, "user");
    assert!(default_role.is_default);
  }

  #[tokio::test]
  async fn test_create_test_user_custom_password() {
    let (pool, _tmp) = create_test_database().await;

    let role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;

    // Create user with custom password
    let user = create_test_user_with_password(&pool, "testuser", role_id, "custompass").await;

    assert_eq!(user.name, "testuser");
    assert_eq!(user.role_id, role_id);
  }

  #[tokio::test]
  async fn test_create_test_user_full() {
    let (pool, _tmp) = create_test_database().await;

    let role_id = create_test_role(&pool, "test_role", &["can_view_user_self"]).await;
    let metadata = serde_json::json!({"key": "value"});

    // Create user with all parameters
    let user = create_test_user_full(
      &pool,
      "testuser",
      Some(role_id),
      "password123",
      Some(metadata.clone()),
    )
    .await;

    assert_eq!(user.name, "testuser");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.metadata_json, Some(metadata.to_string()));
  }

  #[tokio::test]
  async fn test_create_test_role_id() {
    let (pool, _tmp) = create_test_database().await;

    // Create role and get ID
    let role_id = create_test_role(
      &pool,
      "test_role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    assert!(role_id > 0);
  }

  #[tokio::test]
  async fn test_create_test_role_model() {
    let (pool, _tmp) = create_test_database().await;

    // Create role and get full model
    let role = create_test_role_model(
      &pool,
      "test_role",
      &["can_view_user_self", "can_list_users"],
      true,
    )
    .await;

    assert_eq!(role.name, "test_role");
    assert!(role.is_default);
    assert_eq!(role.permissions.len(), 2);
    assert!(role.permissions.contains(&"can_view_user_self".to_string()));
    assert!(role.permissions.contains(&"can_list_users".to_string()));
  }

  #[tokio::test]
  async fn test_create_test_role_empty_permissions() {
    let (pool, _tmp) = create_test_database().await;

    // Create role with no permissions
    let role = create_test_role_model(&pool, "empty_role", &[], false).await;

    assert_eq!(role.name, "empty_role");
    assert!(!role.is_default);
    assert_eq!(role.permissions.len(), 0);
  }

  #[tokio::test]
  async fn test_create_test_role_all_permissions() {
    let (pool, _tmp) = create_test_database().await;

    // Create role with all available permissions
    let role = create_test_role_model(&pool, "admin_role", ROLE_PERMISSIONS, false).await;

    assert_eq!(role.name, "admin_role");
    assert_eq!(role.permissions.len(), ROLE_PERMISSIONS.len());
  }

  // Tests for GraphQL schema helper methods
  #[tokio::test]
  async fn test_create_test_query_schema_no_context() {
    use super::*;

    let schema = create_test_query_schema(TestEmptyQuery, None, None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_query_schema_with_pool() {
    use super::*;

    let (pool, _tmp) = create_test_database().await;
    let schema = create_test_query_schema(TestEmptyQuery, Some(pool), None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_query_schema_with_session() {
    use super::*;

    let session = SessionContext::new(None);
    let schema = create_test_query_schema(TestEmptyQuery, None, Some(session), None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_query_schema_with_config() {
    use super::*;

    let config = DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      session_secret: vec![1, 2, 3, 4], // dummy secret
      cookie_domain: "localhost".to_string(),
      api_path: "/graphql".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };
    let schema = create_test_query_schema(TestEmptyQuery, None, None, Some(config));

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_query_schema_all_context() {
    use super::*;

    let (pool, _tmp) = create_test_database().await;
    let session = SessionContext::new(None);
    let config = DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      session_secret: vec![1, 2, 3, 4], // dummy secret
      cookie_domain: "localhost".to_string(),
      api_path: "/graphql".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let schema = create_test_query_schema(TestEmptyQuery, Some(pool), Some(session), Some(config));

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_mutation_schema_no_context() {
    use super::*;

    let schema = create_test_mutation_schema(EmptyMutation, None, None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_mutation_schema_with_pool() {
    use super::*;

    let (pool, _tmp) = create_test_database().await;
    let schema = create_test_mutation_schema(EmptyMutation, Some(pool), None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_mutation_schema_with_session() {
    use super::*;

    let session = SessionContext::new(None);
    let schema = create_test_mutation_schema(EmptyMutation, None, Some(session), None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_mutation_schema_with_config() {
    use super::*;

    let config = DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      session_secret: vec![1, 2, 3, 4], // dummy secret
      cookie_domain: "localhost".to_string(),
      api_path: "/graphql".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };
    let schema = create_test_mutation_schema(EmptyMutation, None, None, Some(config));

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[tokio::test]
  async fn test_create_test_mutation_schema_all_context() {
    use super::*;

    let (pool, _tmp) = create_test_database().await;
    let session = SessionContext::new(None);
    let config = DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      session_secret: vec![1, 2, 3, 4], // dummy secret
      cookie_domain: "localhost".to_string(),
      api_path: "/graphql".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      session_ttl_seconds: 3600,
    };

    let schema =
      create_test_mutation_schema(EmptyMutation, Some(pool), Some(session), Some(config));

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }
}
