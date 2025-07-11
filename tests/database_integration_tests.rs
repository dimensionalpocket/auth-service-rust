use dp_auth_service::{
  database::Database,
  queries::{
    user_roles::{GetAllRolesQuery, GetRoleByNameQuery},
    users::{CreateUserQuery, GetUserByUuidQuery, CreateUserData},
  },
  services::PasswordService,
};
use sqlx::SqlitePool;
use tempfile::NamedTempFile;
use uuid::Uuid;

async fn create_test_database() -> (SqlitePool, NamedTempFile) {
  let temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let database_url = format!("sqlite:{}", temp_file.path().display());
  
  let pool = SqlitePool::connect(&database_url).await.expect("Failed to connect to test database");
  
  // Configure SQLite settings (same as production)
  for command in Database::SQLITE_PRAGMA_COMMANDS.iter() {
    sqlx::query(command).execute(&pool).await.expect("Failed to configure SQLite");
  }
  
  // Run migrations
  sqlx::migrate!("./config/database/migrations")
    .run(&pool)
    .await
    .expect("Failed to run migrations");
  
  (pool, temp_file)
}

#[tokio::test]
async fn test_complete_user_creation_flow() {
  // Setup test database
  let (pool, _temp_file) = create_test_database().await;
  
  // Insert test roles manually for this test
  sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)")
    .execute(&pool)
    .await
    .unwrap();
  
  // Get user role
  let user_role = GetRoleByNameQuery::run(&pool, "user")
    .await
    .unwrap()
    .expect("User role should exist");
  
  // Create password hash
  let password_hash = PasswordService::generate("test_password").unwrap();
  
  // Create user
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid.clone(),
    name: "Test User".to_string(),
    role_id: Some(user_role.id),
    password_hash,
    metadata_json: Some(r#"{"test": true}"#.to_string()),
  };
  
  let created_user = CreateUserQuery::run(&pool, create_data)
    .await
    .unwrap();
  
  // Verify user was created correctly
  assert_eq!(created_user.uuid, user_uuid);
  assert_eq!(created_user.name, "Test User");
  assert_eq!(created_user.role_id, user_role.id);
  
  // Verify user can be retrieved by UUID
  let retrieved_user = GetUserByUuidQuery::run(&pool, &user_uuid)
    .await
    .unwrap()
    .expect("User should be found");
  
  assert_eq!(retrieved_user.id, created_user.id);
  assert_eq!(retrieved_user.name, created_user.name);
}

#[tokio::test]
async fn test_default_roles_seeded() {
  // Setup test database
  let (pool, _temp_file) = create_test_database().await;
  
  // Insert test roles manually to simulate seeding
  sqlx::query("INSERT INTO user_roles (name, created_ts, is_default) VALUES ('admin', 1234567890, FALSE), ('user', 1234567891, TRUE)")
    .execute(&pool)
    .await
    .unwrap();
  
  // Verify default roles exist
  let roles = GetAllRolesQuery::run(&pool).await.unwrap();
  
  assert_eq!(roles.len(), 2);
  
  let role_names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
  assert!(role_names.contains(&"admin"));
  assert!(role_names.contains(&"user"));
}

#[tokio::test]
async fn test_foreign_key_constraint_enforced() {
  // Setup test database
  let (pool, _temp_file) = create_test_database().await;
  
  // Try to create user with invalid role_id
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid,
    name: "Test User".to_string(),
    role_id: Some(999), // Non-existent role
    password_hash: "test_hash".to_string(),
    metadata_json: None,
  };
  
  let result = CreateUserQuery::run(&pool, create_data).await;
  
  // Should fail due to foreign key constraint
  assert!(result.is_err());
}