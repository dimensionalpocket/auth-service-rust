/*
 * Shared test utilities for creating test data.
 *
 * This module provides centralized functions for creating test users, roles, and other entities.
 * All helpers use existing Query objects to ensure consistency with application behavior.
 *
 * Do NOT create local create_test_* functions in test modules - use these shared utilities instead.
 */

#[cfg(any(test, feature = "test-utils"))]
use crate::database::{Databases, MainDatabase, SessionDatabase};
use crate::models::{Role, User};
use crate::queries::roles::{CreateRoleData, CreateRoleQuery, GetDefaultRoleQuery};
use crate::queries::users::{CreateUserData, CreateUserQuery};
use crate::services::GeneratePasswordHashService;
use sqlx::{SqliteConnection, SqlitePool};
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

pub async fn create_test_database() -> (Databases, NamedTempFile, NamedTempFile) {
  create_test_database_with_config(true).await
}

pub async fn create_test_database_with_pool_size(
  pool_size: u32,
) -> (Databases, NamedTempFile, NamedTempFile) {
  create_test_database_with_config_and_pool_size(true, pool_size).await
}

pub async fn create_test_database_with_config(
  configure_sqlite: bool,
) -> (Databases, NamedTempFile, NamedTempFile) {
  create_test_database_with_config_and_pool_size(configure_sqlite, 1).await
}

pub async fn create_test_database_with_config_and_pool_size(
  configure_sqlite: bool,
  pool_size: u32,
) -> (Databases, NamedTempFile, NamedTempFile) {
  let main_temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let main_sqlite_file_path = main_temp_file.path().display().to_string();

  let session_temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let session_sqlite_file_path = session_temp_file.path().display().to_string();

  let database = MainDatabase::new_with_pool_size(&main_sqlite_file_path, Some(pool_size))
    .await
    .expect("Failed to create test database");
  let main_pool = database.pool;

  // Optionally configure SQLite settings
  if configure_sqlite {
    MainDatabase::configure_sqlite(&main_pool)
      .await
      .expect("Failed to configure SQLite");
  }

  // Run migrations
  let database = MainDatabase {
    pool: main_pool.clone(),
  };
  database.migrate().await.expect("Failed to run migrations");

  let session_database = SessionDatabase::new_with_pool_size(&session_sqlite_file_path, Some(1))
    .await
    .expect("Failed to create session test database");
  let session_pool = session_database.pool;

  let databases = Databases::new(main_pool, session_pool);

  (databases, main_temp_file, session_temp_file)
}

/// Create a test user with default password
pub async fn create_test_user_with_pool(pool: &SqlitePool, username: &str, role_id: i64) -> User {
  let mut conn = pool.acquire().await.unwrap();
  create_test_user(&mut conn, username, role_id).await
}

/// Create a test user with default password (takes connection)
pub async fn create_test_user(conn: &mut SqliteConnection, username: &str, role_id: i64) -> User {
  create_test_user_with_password(conn, username, role_id, "password123").await
}

/// Create a test user with custom password
pub async fn create_test_user_with_pool_and_password(
  pool: &SqlitePool,
  username: &str,
  role_id: i64,
  password: &str,
) -> User {
  let mut conn = pool.acquire().await.unwrap();
  create_test_user_with_password(&mut conn, username, role_id, password).await
}

/// Create a test user with custom password (takes connection)
pub async fn create_test_user_with_password(
  conn: &mut SqliteConnection,
  username: &str,
  role_id: i64,
  password: &str,
) -> User {
  create_test_user_full(conn, username, Some(role_id), password, None).await
}

/// Create a test user with all parameters
pub async fn create_test_user_full_with_pool(
  pool: &SqlitePool,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  let mut conn = pool.acquire().await.unwrap();
  create_test_user_full(&mut conn, username, role_id, password, metadata_json).await
}

/// Create a test user with all parameters (takes connection)
pub async fn create_test_user_full(
  conn: &mut SqliteConnection,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  // If no role_id specified, ensure a default role exists
  let final_role_id = if role_id.is_none() {
    // Check if default role exists
    let default_role_id = GetDefaultRoleQuery::run(conn).await.unwrap().map(|r| r.id);

    match default_role_id {
      Some(id) => Some(id),
      None => {
        // Create a default role if none exists
        Some(
          create_test_role_model(conn, "user", &["can_view_user_self"], true)
            .await
            .id,
        )
      }
    }
  } else {
    role_id
  };

  let password_hash = GeneratePasswordHashService::run(password).unwrap();
  let metadata_json_str = metadata_json.map(|v| v.to_string());
  let create_data = CreateUserData {
    uuid: Uuid::new_v4().to_string(),
    name: username.to_string(),
    role_id: final_role_id,
    password_hash,
    metadata_json: metadata_json_str,
  };
  CreateUserQuery::run(conn, create_data).await.unwrap()
}

/// Create a test user with all parameters including specific UUID
pub async fn create_test_user_with_pool_and_uuid(
  pool: &SqlitePool,
  uuid: &str,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  let mut conn = pool.acquire().await.unwrap();
  create_test_user_with_uuid(&mut conn, uuid, username, role_id, password, metadata_json).await
}

/// Create a test user with all parameters including specific UUID (takes connection)
pub async fn create_test_user_with_uuid(
  conn: &mut SqliteConnection,
  uuid: &str,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  // If no role_id specified, ensure a default role exists
  let final_role_id = if role_id.is_none() {
    // Check if default role exists
    let default_role_id = GetDefaultRoleQuery::run(conn).await.unwrap().map(|r| r.id);

    match default_role_id {
      Some(id) => Some(id),
      None => {
        // Create a default role if none exists
        Some(
          create_test_role_model(conn, "user", &["can_view_user_self"], true)
            .await
            .id,
        )
      }
    }
  } else {
    role_id
  };

  let password_hash = GeneratePasswordHashService::run(password).unwrap();
  let metadata_json_str = metadata_json.map(|v| v.to_string());
  let create_data = CreateUserData {
    uuid: uuid.to_string(),
    name: username.to_string(),
    role_id: final_role_id,
    password_hash,
    metadata_json: metadata_json_str,
  };
  CreateUserQuery::run(conn, create_data).await.unwrap()
}

/// Create a test role and return ID
pub async fn create_test_role_with_pool(
  pool: &SqlitePool,
  name: &str,
  permissions: &[&str],
) -> i64 {
  let role = create_test_role_model_with_pool(pool, name, permissions, false).await;
  role.id
}

/// Create a test role and return ID (takes connection)
pub async fn create_test_role(
  conn: &mut SqliteConnection,
  name: &str,
  permissions: &[&str],
) -> i64 {
  let role = create_test_role_model(conn, name, permissions, false).await;
  role.id
}

/// Create a test role and return full Role model
pub async fn create_test_role_model_with_pool(
  pool: &SqlitePool,
  name: &str,
  permissions: &[&str],
  is_default: bool,
) -> Role {
  let mut conn = pool.acquire().await.unwrap();
  create_test_role_model(&mut conn, name, permissions, is_default).await
}

/// Create a test role and return full Role model (takes connection)
pub async fn create_test_role_model(
  conn: &mut SqliteConnection,
  name: &str,
  permissions: &[&str],
  is_default: bool,
) -> Role {
  let create_data = CreateRoleData {
    name: name.to_string(),
    permissions: permissions.iter().map(|&p| p.to_string()).collect(),
    is_default,
  };
  CreateRoleQuery::run(conn, create_data).await.unwrap()
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
        "query": "mutation {{ authRegister(username: \"{username}\", password: \"{password}\", passwordConfirmation: \"{password}\") {{ user {{ uuid name }} }} }}"
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
  assert!(!data["data"]["authRegister"]["user"]["uuid"].is_null());

  data["data"]["authRegister"]["user"]["uuid"]
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
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::models::ROLE_PERMISSIONS;

  #[dps_auth_db_test]
  async fn test_create_test_user_no_role() {
    // Create user without role (will create and use default role automatically)
    let user = create_test_user_full_with_pool(&pool, "testuser", None, "password123", None).await;

    assert_eq!(user.name, "testuser");
    // Should have the default role ID (created automatically)
    assert!(user.role_id > 0);

    // Verify that default role is created automatically
    let mut conn = pool.acquire().await.unwrap();
    let default_role = GetDefaultRoleQuery::run(&mut conn).await.unwrap().unwrap();
    assert_eq!(user.role_id, default_role.id);
    assert_eq!(default_role.name, "user");
    assert!(default_role.is_default);
  }

  #[dps_auth_db_test]
  async fn test_create_test_user_custom_password() {
    let role_id = create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;

    // Create user with custom password
    let user =
      create_test_user_with_pool_and_password(&pool, "testuser", role_id, "custompass").await;

    assert_eq!(user.name, "testuser");
    assert_eq!(user.role_id, role_id);
  }

  #[dps_auth_db_test]
  async fn test_create_test_user_full() {
    let role_id = create_test_role_with_pool(&pool, "test_role", &["can_view_user_self"]).await;
    let metadata = serde_json::json!({"key": "value"});

    // Create user with all parameters
    let user = create_test_user_full_with_pool(
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

  #[dps_auth_db_test]
  async fn test_create_test_role_id() {
    // Create role and get ID
    let role_id = create_test_role_with_pool(
      &pool,
      "test_role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    assert!(role_id > 0);
  }

  #[dps_auth_db_test]
  async fn test_create_test_role_model() {
    // Create role and get full model
    let role = create_test_role_model_with_pool(
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

  #[dps_auth_db_test]
  async fn test_create_test_role_empty_permissions() {
    // Create role with no permissions
    let role = create_test_role_model_with_pool(&pool, "empty_role", &[], false).await;

    assert_eq!(role.name, "empty_role");
    assert!(!role.is_default);
    assert_eq!(role.permissions.len(), 0);
  }

  #[dps_auth_db_test]
  async fn test_create_test_role_all_permissions() {
    // Create role with all available permissions
    let role = create_test_role_model_with_pool(&pool, "admin_role", ROLE_PERMISSIONS, false).await;

    assert_eq!(role.name, "admin_role");
    assert_eq!(role.permissions.len(), ROLE_PERMISSIONS.len());
  }

  // Tests for GraphQL schema helper methods
  #[dps_auth_db_test]
  async fn test_create_test_query_schema_no_context() {
    use super::*;

    let schema = create_test_query_schema(TestEmptyQuery, None, None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
  async fn test_create_test_query_schema_with_pool() {
    use super::*;

    let schema = create_test_query_schema(TestEmptyQuery, Some(pool), None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
  async fn test_create_test_query_schema_with_session() {
    use super::*;

    let session = SessionContext::new(None);
    let schema = create_test_query_schema(TestEmptyQuery, None, Some(session), None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
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

  #[dps_auth_db_test]
  async fn test_create_test_query_schema_all_context() {
    use super::*;

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

  #[dps_auth_db_test]
  async fn test_create_test_mutation_schema_no_context() {
    use super::*;

    let schema = create_test_mutation_schema(EmptyMutation, None, None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
  async fn test_create_test_mutation_schema_with_pool() {
    use super::*;

    let schema = create_test_mutation_schema(EmptyMutation, Some(pool), None, None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
  async fn test_create_test_mutation_schema_with_session() {
    use super::*;

    let session = SessionContext::new(None);
    let schema = create_test_mutation_schema(EmptyMutation, None, Some(session), None);

    // Verify schema was created successfully
    assert!(schema.execute("{ dummy }").await.is_ok());
  }

  #[dps_auth_db_test]
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

  #[dps_auth_db_test]
  async fn test_create_test_mutation_schema_all_context() {
    use super::*;

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

  #[dps_auth_db_test]
  async fn test_create_test_user_with_uuid_success() {
    use crate::queries::users::GetUserByUuidQuery;

    // Test data
    let test_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let test_username = "testuser";
    let test_password = "password123";
    let test_metadata = serde_json::json!({"key": "value"});

    // Create user with specific UUID
    let user = create_test_user_with_pool_and_uuid(
      &pool,
      test_uuid,
      test_username,
      None, // Use default role
      test_password,
      Some(test_metadata.clone()),
    )
    .await;

    // Verify user was created with correct data
    assert_eq!(user.uuid, test_uuid);
    assert_eq!(user.name, test_username);
    assert!(user.role_id > 0); // Should have default role
    assert!(user.password_hash.starts_with("$argon2")); // Password should be hashed
    assert_eq!(user.metadata_json, Some(test_metadata.to_string()));

    // Verify user can be retrieved by UUID
    let mut conn = pool.acquire().await.unwrap();
    let retrieved_user = GetUserByUuidQuery::run(&mut conn, test_uuid).await.unwrap();
    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.uuid, test_uuid);
    assert_eq!(retrieved_user.name, test_username);
  }
}
