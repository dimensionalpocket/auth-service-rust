use crate::graphql::types::UserRole;
use crate::middleware::session::SessionContext;
use crate::orchestrators::user::GetUserOrchestrator;
use crate::types::UserError;
use async_graphql::{Context, Error, Object};
use tracing::instrument;

/// GraphQL output type for complete user details (admin only)
#[derive(async_graphql::SimpleObject)]
pub struct UserDetailsResponse {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  /// The user's role information
  pub role: UserRole,
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

    match GetUserOrchestrator::run(pool, session_context.clone(), id).await {
      Ok(user) => Ok(UserDetailsResponse {
        id: user.user.id,
        uuid: user.user.uuid,
        name: user.user.name,
        role: UserRole::from(user.role),
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
  use crate::middleware::session::SessionContext;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::test_utils::{
    create_test_query_schema, create_test_role_model_with_pool, create_test_user_full_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_success() {
    // Insert admin role with can_view_user_details permission
    let admin_role = create_test_role_model_with_pool(
      &pool,
      "admin",
      &["is_admin", "can_view_user_details"],
      false,
    )
    .await;
    let admin_role_id = admin_role.id;

    // Insert user role without special permissions
    let user_role = create_test_role_model_with_pool(&pool, "user", &[], true).await;
    let user_role_id = user_role.id;

    // Insert admin user
    let admin_user =
      create_test_user_full_with_pool(&pool, "admin", Some(admin_role_id), "test_password", None)
        .await;
    let admin_user_id = admin_user.id;

    // Create test user
    let create_data = CreateUserData {
      uuid: "target-user-uuid".to_string(),
      name: "target_user".to_string(),
      role_id: Some(user_role_id),
      password_hash: "hashed_password".to_string(),
      metadata_json: None,
    };
    let target_user = {
      let mut conn = pool.acquire().await.unwrap();
      CreateUserQuery::run(&mut conn, create_data).await.unwrap()
    }; // Connection released before schema.execute()

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = create_test_query_schema(query_resolver, Some(pool), Some(session_context), None);

    let query = format!(
      r#"
            query {{
                user(id: {}) {{
                    id
                    uuid
                    name
                    role {{
                        id
                        name
                        permissions
                    }}
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
    assert_eq!(user_data["role"]["name"].as_str().unwrap(), "user");
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_forbidden() {
    // Insert user role without can_view_user_details permission
    let user_role =
      create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let user_role_id = user_role.id;

    // Insert regular user
    let user =
      create_test_user_full_with_pool(&pool, "user", Some(user_role_id), "test_password", None)
        .await;
    let user_id = user.id;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = create_test_query_schema(query_resolver, Some(pool), Some(session_context), None);

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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let query_resolver = UserResolver;
    let schema = create_test_query_schema(query_resolver, Some(pool), Some(session_context), None);

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

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_not_found() {
    // Insert admin role with can_view_user_details permission
    let admin_role = create_test_role_model_with_pool(
      &pool,
      "admin",
      &["is_admin", "can_view_user_details"],
      false,
    )
    .await;
    let admin_role_id = admin_role.id;

    // Insert admin user
    let admin_user =
      create_test_user_full_with_pool(&pool, "admin", Some(admin_role_id), "test_password", None)
        .await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = UserResolver;
    let schema = create_test_query_schema(query_resolver, Some(pool), Some(session_context), None);

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
