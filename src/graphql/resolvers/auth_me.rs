use crate::graphql::types::{UserRole, UserWithRoleResponse};
use crate::middleware::session::SessionContext;
use crate::orchestrators::auth::AuthMeOrchestrator;
use crate::types::SessionError;
use async_graphql::{Context, Object, Result};
use tracing::instrument;

/// GraphQL output type for current authenticated user information
#[derive(async_graphql::SimpleObject)]
pub struct AuthMeResponse {
  /// The authenticated user's information
  pub user: UserWithRoleResponse,
  /// When the current session was created (seconds since Unix epoch)
  #[graphql(name = "sessionIat")]
  pub session_iat: i64,
  /// When the current session expires (seconds since Unix epoch)
  #[graphql(name = "sessionExp")]
  pub session_exp: i64,
}

/// Current authenticated user query resolver
#[derive(Default, Debug)]
pub struct AuthMeResolver;

#[Object]
impl AuthMeResolver {
  /// Returns information about the currently authenticated user.
  ///
  /// This query retrieves detailed user information for the authenticated user,
  /// including both user profile data and current session information.
  /// The response contains:
  /// - User profile (nested): ID, UUID, username, role information (id, name, permissions), timestamps
  /// - Session data: When session was created and when it expires
  ///
  /// Returns `null` if no valid session token was provided in the request.
  ///
  /// # Authentication
  ///
  /// This query requires a valid session token to be provided either:
  /// - In the `Authorization` header as `Bearer <token>`
  /// - In the `DpsAuthSession` cookie
  ///
  /// # Examples
  ///
  /// **Authenticated request:**
  /// ```graphql
  /// query {
  ///   authMe {
  ///     user {
  ///       id
  ///       uuid
  ///       name
  ///       role {
  ///         id
  ///         name
  ///         permissions
  ///       }
  ///       createdTs
  ///       updatedTs
  ///     }
  ///     sessionIat
  ///     sessionExp
  ///   }
  /// }
  /// ```
  ///
  /// **Response for authenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "authMe": {
  ///       "user": {
  ///         "id": 123,
  ///         "uuid": "550e8400-e29b-41d4-a716-446655440000",
  ///         "name": "johndoe",
  ///         "role": {
  ///           "id": "2",
  ///           "name": "user",
  ///           "permissions": ["can_view_user_self"]
  ///         },
  ///         "createdTs": 1706356800,
  ///         "updatedTs": 1706356800
  ///       },
  ///       "sessionIat": 1706356800,
  ///       "sessionExp": 1706616000
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// **Response for unauthenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "authMe": null
  ///   }
  /// }
  /// ```
  #[instrument(skip(self, ctx))]
  #[graphql(name = "authMe")]
  async fn auth_me(&self, ctx: &Context<'_>) -> Result<Option<AuthMeResponse>> {
    let pool = ctx.data::<sqlx::SqlitePool>()?;

    // Try to get session context, but don't fail if it's missing
    let session_context = match SessionContext::from_context(ctx) {
      Ok(context) => context,
      Err(_) => return Ok(None),
    };

    match AuthMeOrchestrator::run(pool, session_context.clone()).await {
      Ok(Some(auth_me_result)) => Ok(Some(AuthMeResponse {
        user: UserWithRoleResponse {
          id: auth_me_result.user_id,
          name: auth_me_result.username,
          role: UserRole::from(auth_me_result.role),
          uuid: Some(auth_me_result.uuid),
          created_ts: Some(auth_me_result.created_ts),
          updated_ts: Some(auth_me_result.updated_ts),
        },
        session_iat: auth_me_result.session_iat,
        session_exp: auth_me_result.session_exp,
      })),
      Ok(None) => Ok(None),
      Err(session_error) => {
        let user_message = map_session_error_to_user_message(&session_error);
        Err(async_graphql::Error::new(user_message))
      }
    }
  }
}

/// Map internal SessionError types to user-friendly messages
fn map_session_error_to_user_message(error: &SessionError) -> &'static str {
  match error {
    SessionError::AuthenticationError(_) => "Authentication required",
    SessionError::DatabaseError(_) => "Internal server error",
    SessionError::PasswordVerificationError(_) => "Internal server error",
    SessionError::AuthSessionError(_) => "Internal server error",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_query_schema, create_test_user_full_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_auth_me_with_authenticated_user() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    // Setup: Create a test user
    let user = create_test_user_full_with_pool(
      &pool,
      "testuser",
      None, // Will create and use default role
      "test_password",
      None,
    )
    .await;
    let user_id = user.id;

    let query = AuthMeResolver;
    let payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload.clone()));

    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema
      .execute(
        "{ authMe { user { id uuid name role { id name permissions } createdTs updatedTs } sessionIat sessionExp } }",
      )
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["authMe"]["user"]["id"], user_id);
    assert_eq!(data["authMe"]["user"]["name"], "testuser");
    assert_eq!(data["authMe"]["user"]["role"]["name"], "user");
    assert_eq!(data["authMe"]["sessionIat"], 1706356800);
    assert_eq!(data["authMe"]["sessionExp"], 1706616000);
  }

  #[tokio::test]
  async fn test_auth_me_with_unauthenticated_user() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let query = AuthMeResolver;
    let session_context = SessionContext::new(None);

    let schema = create_test_query_schema(query, Some(pool), Some(session_context), None);

    let result = schema.execute("{ authMe { user { id uuid name } } }").await;

    // Should succeed with null response when no session
    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["authMe"].is_null());
  }

  #[tokio::test]
  async fn test_auth_me_missing_context() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let query = AuthMeResolver;

    let schema = create_test_query_schema(query, Some(pool), None, None);
    let result = schema.execute("{ authMe { user { id uuid name } } }").await;

    // Should succeed with null response when no session context
    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["authMe"].is_null());
  }

  #[tokio::test]
  async fn test_map_session_error_to_user_message() {
    use dps_auth_session::DpsAuthSessionError;

    assert_eq!(
      map_session_error_to_user_message(&SessionError::AuthenticationError("test".to_string())),
      "Authentication required"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::DatabaseError("test".to_string())),
      "Internal server error"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::PasswordVerificationError(
        "test".to_string()
      )),
      "Internal server error"
    );

    assert_eq!(
      map_session_error_to_user_message(&SessionError::AuthSessionError(
        DpsAuthSessionError::EncodingError("test".to_string())
      )),
      "Internal server error"
    );
  }
}
