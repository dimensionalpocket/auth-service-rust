use crate::middleware::session::SessionContext;
use crate::orchestrators::user_orchestrator::UserOrchestrator;
use crate::services::UserError;
use async_graphql::{Context, Error, Object};
use tracing::instrument;

/// GraphQL output type for complete user details (admin only)
#[derive(async_graphql::SimpleObject)]
pub struct UserDetailsResponse {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  /// The user's role ID
  #[graphql(name = "roleId")]
  pub role_id: i64,
  /// The user's role name
  #[graphql(name = "roleName")]
  pub role_name: String,
  /// Timestamp when the user was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the user was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UserResolver;

#[Object]
impl UserResolver {
  /// Get complete user details by ID (admin only).
  ///
  /// This query:
  /// - Requires user authentication
  /// - Checks if the user has "can_view_user_details" permission
  /// - Returns complete user information including role details
  /// - Can be used to view any user's details (including self)
  ///
  /// # Arguments
  /// * `id` - The ID of the user to retrieve
  ///
  /// # Returns
  /// * `UserDetailsResponse` - Complete user details with role information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_view_user_details" permission
  /// * Returns "User with ID {id} not found" if target user doesn't exist
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(user_id = %id))]
  #[graphql(name = "user")]
  async fn user(&self, ctx: &Context<'_>, id: i64) -> Result<UserDetailsResponse, Error> {
    let pool = ctx.data::<sqlx::SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match UserOrchestrator::get_user_details_with_permission_check(
      pool,
      session_context.clone(),
      id,
    )
    .await
    {
      Ok(user) => Ok(UserDetailsResponse {
        id: user.user.id,
        uuid: user.user.uuid,
        name: user.user.name,
        role_id: user.user.role_id,
        role_name: user.role_name,
        created_ts: user.user.created_ts,
        updated_ts: user.user.updated_ts,
      }),
      Err(UserError::AuthenticationError(msg)) => Err(Error::new(msg)),
      Err(UserError::AuthorizationError(msg)) => Err(Error::new(msg)),
      Err(UserError::UserNotFound(user_id)) => {
        Err(Error::new(format!("User with ID {user_id} not found")))
      }
      Err(err) => {
        tracing::error!("Failed to get user details: {}", err);
        Err(Error::new("Failed to retrieve user details"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use async_graphql::{EmptyMutation, EmptySubscription, Schema};
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_get_user_details_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_view_user_details permission
    sqlx::query(
            "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_view_user_details\"]')"
        )
        .execute(&pool)
        .await
        .unwrap();

    // Insert user role without special permissions
    let user_role_result = sqlx::query(
            "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[]')"
        )
        .execute(&pool)
        .await
        .unwrap();
    let user_role_id = user_role_result.last_insert_rowid();

    // Insert admin user
    let admin_user_result = sqlx::query(
            "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
        )
        .execute(&pool)
        .await
        .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create test user
    let create_data = CreateUserData {
      uuid: "target-user-uuid".to_string(),
      name: "target_user".to_string(),
      role_id: Some(user_role_id),
      password_hash: "hashed_password".to_string(),
      metadata_json: None,
    };
    let target_user = CreateUserQuery::run(&pool, create_data).await.unwrap();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = format!(
      r#"
            query {{
                user(id: {}) {{
                    id
                    uuid
                    name
                    roleId
                    roleName
                    createdTs
                    updatedTs
                }}
            }}
            "#,
      target_user.id
    );

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["user"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(user_data["uuid"].as_str().unwrap(), "target-user-uuid");
    assert_eq!(user_data["name"].as_str().unwrap(), "target_user");
    assert_eq!(user_data["roleId"].as_i64().unwrap(), user_role_id);
    assert_eq!(user_data["roleName"].as_str().unwrap(), "user");
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[tokio::test]
  async fn test_get_user_details_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert user role without can_view_user_details permission
    sqlx::query(
            "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('user', 1234567890, 1234567890, TRUE, '[\"can_view_user_self\"]')"
        )
        .execute(&pool)
        .await
        .unwrap();

    // Insert regular user
    let user_result = sqlx::query(
            "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user-uuid', 1234567890, 1234567890, 'user', 1, 'hashed_password', NULL)"
        )
        .execute(&pool)
        .await
        .unwrap();
    let user_id = user_result.last_insert_rowid();

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
            query {
                user(id: 1) {
                    id
                    name
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[tokio::test]
  async fn test_get_user_details_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let query_resolver = UserResolver;
    let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
            query {
                user(id: 1) {
                    id
                    name
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[tokio::test]
  async fn test_get_user_details_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_view_user_details permission
    sqlx::query(
            "INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[\"is_admin\", \"can_view_user_details\"]')"
        )
        .execute(&pool)
        .await
        .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
            "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
        )
        .execute(&pool)
        .await
        .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
            query {
                user(id: 999) {
                    id
                    name
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("User with ID 999 not found"));
  }
}
