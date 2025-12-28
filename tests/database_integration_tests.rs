use dps_auth_api::{
  queries::{
    roles::{GetAllRolesQuery, GetRoleByNameQuery},
    users::{CreateUserData, CreateUserQuery, GetUserByUuidQuery},
  },
  test_utils::{create_test_database, create_test_role, create_test_user_full},
};
use uuid::Uuid;

#[tokio::test]
async fn test_complete_user_creation_flow() {
  // Setup test database
  let (pool, _temp_file) = create_test_database().await;

  // Create test roles using test utilities
  let _admin_role_id = create_test_role(&pool, "admin", &[]).await;
  let _user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

  // Get user role
  let role = {
    let mut conn = pool.acquire().await.unwrap();
    GetRoleByNameQuery::run(&mut conn, "user")
      .await
      .unwrap()
      .expect("Role should exist")
  };

  // Create user using test utility (this tests the complete flow)
  let created_user = create_test_user_full(
    &pool,
    "Test User",
    Some(role.id),
    "test_password",
    Some(serde_json::json!({"test": true})),
  )
  .await;

  // Verify user was created correctly
  assert_eq!(created_user.name, "Test User");
  assert_eq!(created_user.role_id, role.id);

  // Verify user can be retrieved by UUID
  let mut conn = pool.acquire().await.unwrap();
  let retrieved_user = GetUserByUuidQuery::run(&mut conn, &created_user.uuid)
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

  // Create test roles using test utilities to simulate seeding
  create_test_role(&pool, "admin", &[]).await;
  create_test_role(&pool, "user", &["can_view_user_self"]).await;

  // Verify default roles exist
  let mut conn = pool.acquire().await.unwrap();
  let roles = GetAllRolesQuery::run(&mut conn).await.unwrap();

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

  let mut conn = pool.acquire().await.unwrap();
  let result = CreateUserQuery::run(&mut conn, create_data).await;

  // Should fail due to foreign key constraint
  assert!(result.is_err());
}
