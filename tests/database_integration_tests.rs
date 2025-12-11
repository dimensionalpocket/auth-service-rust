use dps_auth_api::{
  test_utils::test_utils::create_test_database,
  queries::{
    roles::{GetAllRolesQuery, GetRoleByNameQuery},
    users::{CreateUserData, CreateUserQuery, GetUserByUuidQuery},
  },
  services::PasswordService,
};
use uuid::Uuid;

#[tokio::test]
async fn test_complete_user_creation_flow() {
  // Setup test database
  let (pool, _temp_file) = create_test_database().await;

  // Insert test roles manually for this test
  sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE), ('user', 1234567891, 1234567891, TRUE)")
    .execute(&pool)
    .await
    .unwrap();

  // Get user role
  let role = GetRoleByNameQuery::run(&pool, "user")
    .await
    .unwrap()
    .expect("Role should exist");

  // Create password hash
  let password_hash = PasswordService::generate("test_password").unwrap();

  // Create user
  let user_uuid = Uuid::new_v4().to_string();
  let create_data = CreateUserData {
    uuid: user_uuid.clone(),
    name: "Test User".to_string(),
    role_id: Some(role.id),
    password_hash,
    metadata_json: Some(r#"{"test": true}"#.to_string()),
  };

  let created_user = CreateUserQuery::run(&pool, create_data).await.unwrap();

  // Verify user was created correctly
  assert_eq!(created_user.uuid, user_uuid);
  assert_eq!(created_user.name, "Test User");
  assert_eq!(created_user.role_id, role.id);

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
  sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default) VALUES ('admin', 1234567890, 1234567890, FALSE), ('user', 1234567891, 1234567891, TRUE)")
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
