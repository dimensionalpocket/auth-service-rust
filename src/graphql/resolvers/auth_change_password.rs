use crate::middleware::session::SessionContext;
use crate::orchestrators::auth_orchestrator::AuthOrchestrator;
use crate::services::UserError;
use async_graphql::{Context, Object, Result, SimpleObject};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for password change response
#[derive(SimpleObject)]
pub struct AuthChangePasswordResponse {
  /// Success message
  pub message: String,
  /// Timestamp when the password was changed
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// GraphQL mutation for changing user password
#[derive(Default, Debug)]
pub struct AuthChangePasswordResolver;

#[Object]
impl AuthChangePasswordResolver {
  /// Change the password for the currently authenticated user.
  ///
  /// This mutation:
  /// - Requires a valid authenticated session
  /// - Verifies the current password before allowing the change
  /// - Validates the new password and confirmation match
  /// - Updates the password hash in the database
  /// - Returns a success message with timestamp
  ///
  /// # Arguments
  /// * `current_password` - Current password for verification
  /// * `new_password` - New password to set
  /// * `new_password_confirmation` - Confirmation of the new password
  ///
  /// # Returns
  /// * `AuthChangePasswordResponse` - Success message and timestamp
  ///
  /// # Errors
  /// * Returns GraphQL error if no valid session exists
  /// * Returns GraphQL error if current password is incorrect
  /// * Returns GraphQL error if new password validation fails
  /// * Returns GraphQL error if password confirmation doesn't match
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(self, ctx, current_password, new_password, new_password_confirmation))]
  #[graphql(name = "authChangePassword")]
  async fn auth_change_password(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "currentPassword")] current_password: String,
    #[graphql(name = "newPassword")] new_password: String,
    #[graphql(name = "newPasswordConfirmation")] new_password_confirmation: String,
  ) -> Result<AuthChangePasswordResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match AuthOrchestrator::change_authenticated_user_password(
      pool,
      session_context.clone(),
      &current_password,
      &new_password,
      &new_password_confirmation,
    )
    .await
    {
      Ok(updated_user) => Ok(AuthChangePasswordResponse {
        message: "Password changed successfully".to_string(),
        updated_ts: updated_user.updated_ts,
      }),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::PasswordHashingFailed(_)) => {
        tracing::error!("Password hashing failed during password change");
        Err(async_graphql::Error::new("Failed to process password"))
      }
      Err(err) => {
        tracing::error!("Failed to change password: {}", err);
        Err(async_graphql::Error::new("Failed to change password"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::{SessionContext, SessionPayload};
  use crate::services::{AuthService, UserService};
  use crate::test_utils::{
    create_test_database, create_test_mutation_schema, create_test_role_model,
  };
  use sqlx::SqliteConnection;

  // Test secret - 32 bytes for AES-256
  const TEST_SECRET: &[u8] = &[
    0x42, 0xf4, 0x25, 0xc2, 0x93, 0x2e, 0x8c, 0xaf, 0xaa, 0xcd, 0xd4, 0x5b, 0x50, 0x28, 0xa4, 0x8d,
    0xcd, 0x74, 0xd4, 0xe2, 0xad, 0xd4, 0xa1, 0xc4, 0xdf, 0xc6, 0x2a, 0xdf, 0xb5, 0x74, 0x4d, 0xb8,
  ];

  async fn create_authenticated_session(
    conn: &mut SqliteConnection,
    username: &str,
    password: &str,
  ) -> (SessionContext, i64) {
    // Create user
    let user = UserService::create_user(conn, username, password)
      .await
      .unwrap();

    // Create session (we don't need the result for this test)
    let _auth_result = AuthService::login(conn, username, password, TEST_SECRET)
      .await
      .unwrap();

    // Create session payload (simplified for testing)
    let session_payload = SessionPayload {
      sub: user.id,
      iat: chrono::Utc::now().timestamp(),
      exp: chrono::Utc::now().timestamp() + 3600,
    };

    let session_context = SessionContext::new(Some(session_payload));
    (session_context, user.id)
  }

  #[tokio::test]
  async fn test_auth_change_password_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create authenticated user
    let session_context = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      create_authenticated_session(&mut conn, "testuser", "oldpassword123")
        .await
        .0
    };

    // Create GraphQL schema
    let mutation = AuthChangePasswordResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test: Change password
    let query = r#"
      mutation {
        authChangePassword(
          currentPassword: "oldpassword123",
          newPassword: "newpassword456",
          newPasswordConfirmation: "newpassword456"
        ) {
          message
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should succeed
    assert!(
      result.errors.is_empty(),
      "GraphQL errors: {:?}",
      result.errors
    );

    let data = result.data.into_json().unwrap();
    let response = &data["authChangePassword"];

    assert_eq!(
      response["message"].as_str().unwrap(),
      "Password changed successfully"
    );
    assert!(response["updatedTs"].as_i64().unwrap() > 0);
  }

  #[tokio::test]
  async fn test_auth_change_password_invalid_current_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create authenticated user
    let session_context = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      create_authenticated_session(&mut conn, "testuser", "correctpassword")
        .await
        .0
    };

    // Create GraphQL schema
    let mutation = AuthChangePasswordResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test: Try to change with wrong current password
    let query = r#"
      mutation {
        authChangePassword(
          currentPassword: "wrongpassword", 
          newPassword: "newpassword456", 
          newPasswordConfirmation: "newpassword456" 
        ) {
          message
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return error
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
      .message
      .contains("Current password is incorrect"));
  }

  #[tokio::test]
  async fn test_auth_change_password_password_confirmation_mismatch() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create authenticated user
    let session_context = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      create_authenticated_session(&mut conn, "testuser", "currentpassword")
        .await
        .0
    };

    // Create GraphQL schema
    let mutation = AuthChangePasswordResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test: Try to change with mismatched confirmation
    let query = r#"
      mutation {
        authChangePassword(
          currentPassword: "currentpassword", 
          newPassword: "newpassword456", 
          newPasswordConfirmation: "differentpassword" 
        ) {
          message
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return error
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Passwords do not match"));
  }

  #[tokio::test]
  async fn test_auth_change_password_no_authentication() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create session without authentication
    let session_context = SessionContext::new(None);

    // Create GraphQL schema
    let mutation = AuthChangePasswordResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test: Try to change password without authentication
    let query = r#"
      mutation {
        authChangePassword(
          currentPassword: "anypassword", 
          newPassword: "newpassword456", 
          newPasswordConfirmation: "newpassword456" 
        ) {
          message
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return authentication error
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[tokio::test]
  async fn test_auth_change_password_invalid_new_password() {
    let (pool, _temp_file) = create_test_database().await;

    // Setup: Create authenticated user
    let session_context = {
      let mut conn = pool.acquire().await.unwrap();
      create_test_role_model(&mut conn, "user", &["can_view_user_self"], true).await;
      create_authenticated_session(&mut conn, "testuser", "currentpassword")
        .await
        .0
    };

    // Create GraphQL schema
    let mutation = AuthChangePasswordResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test: Try to change with invalid new password (too short)
    let query = r#"
      mutation {
        authChangePassword(
          currentPassword: "currentpassword", 
          newPassword: "123", 
          newPasswordConfirmation: "123" 
        ) {
          message
          updatedTs
        }
      }
    "#;

    let result = schema.execute(query).await;

    // Verify: Should return validation error
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("at least 6 characters"));
  }
}
