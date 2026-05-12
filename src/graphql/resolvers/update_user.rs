use crate::database::Databases;
use crate::graphql::types::UserRole;
use crate::middleware::session::SessionContext;
use crate::orchestrators::user::UpdateUserOrchestrator;
use crate::types::{user::update_user_input::UpdateUserInput, DpsAuthApiConfig, UserError};
use async_graphql::{Context, Object, Result};
use tracing::instrument;

#[derive(async_graphql::SimpleObject)]
pub struct UpdateUserResponse {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  pub role: UserRole,
  #[graphql(name = "metadataJson")]
  pub metadata_json: Option<String>,
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UpdateUserResolver;

#[Object]
impl UpdateUserResolver {
  #[instrument(skip(ctx, password, password_confirmation), fields(id = %id))]
  #[allow(clippy::too_many_arguments)]
  #[graphql(name = "updateUser")]
  async fn update_user(
    &self,
    ctx: &Context<'_>,
    id: i64,
    name: Option<String>,
    #[graphql(name = "roleId")] role_id: Option<i64>,
    password: Option<String>,
    #[graphql(name = "passwordConfirmation")] password_confirmation: Option<String>,
    #[graphql(name = "metadataJson")] metadata_json: Option<String>,
  ) -> Result<UpdateUserResponse> {
    let databases = ctx.data::<Databases>()?;
    let config = ctx.data::<DpsAuthApiConfig>()?;

    let session_context = SessionContext::from_context(ctx)?;

    let input = UpdateUserInput {
      id,
      name,
      role_id,
      password,
      password_confirmation,
      metadata_json,
    };

    match UpdateUserOrchestrator::run(databases, session_context.clone(), config, input).await {
      Ok(user_with_role) => Ok(UpdateUserResponse {
        id: user_with_role.user.id,
        uuid: user_with_role.user.uuid,
        name: user_with_role.user.name,
        role: UserRole::from(user_with_role.role),
        metadata_json: user_with_role.user.metadata_json,
        created_ts: user_with_role.user.created_ts,
        updated_ts: user_with_role.user.updated_ts,
      }),
      Err(UserError::UserNotFound(user_id)) => Err(async_graphql::Error::new(format!(
        "User with ID {user_id} not found"
      ))),
      Err(UserError::UsernameAlreadyExists(username)) => Err(async_graphql::Error::new(format!(
        "Username '{username}' is already in use"
      ))),
      Err(UserError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(UserError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(UserError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to update user: {}", err);
        Err(async_graphql::Error::new("Failed to update user"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_mutation_schema, create_test_role_with_databases,
    create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_db_test]
  async fn test_update_user_success() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(
          id: $USER_ID,
          name: "updateduser",
          roleId: $ROLE_ID,
          password: "newpassword123",
          passwordConfirmation: "newpassword123",
          metadataJson: "{\"updated\": true}"
        ) {
          id
          uuid
          name
          role {
            id
            name
            permissions
          }
          metadataJson
          createdTs
          updatedTs
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string())
    .replace("$ROLE_ID", &admin_role_id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(user_data["name"].as_str().unwrap(), "updateduser");
    assert_eq!(
      user_data["role"]["id"].as_str().unwrap(),
      admin_role_id.to_string()
    );
    assert_eq!(user_data["role"]["name"].as_str().unwrap(), "admin");
    assert!(user_data["createdTs"].as_i64().unwrap() > 0);
    assert!(user_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[dps_auth_db_test]
  async fn test_update_user_partial_name_only() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, name: "updatedname") {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(user_data["name"].as_str().unwrap(), "updatedname");
  }

  #[dps_auth_db_test]
  async fn test_update_user_partial_role_only() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let moderator_role_id = create_test_role_with_databases(&databases, "moderator", &[]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, roleId: $ROLE_ID) {
          id
          role {
            id
            name
          }
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string())
    .replace("$ROLE_ID", &moderator_role_id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(
      user_data["role"]["id"].as_str().unwrap(),
      moderator_role_id.to_string()
    );
    assert_eq!(user_data["role"]["name"].as_str().unwrap(), "moderator");
  }

  #[dps_auth_db_test]
  async fn test_update_user_partial_password_with_confirmation() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(
          id: $USER_ID,
          password: "newpassword123",
          passwordConfirmation: "newpassword123"
        ) {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(user_data["name"].as_str().unwrap(), "targetuser");
  }

  #[dps_auth_db_test]
  async fn test_update_user_partial_metadata() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, metadataJson: "{\"updated\": true}") {
          id
          metadataJson
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(
      user_data["metadataJson"].as_str().unwrap(),
      "{\"updated\": true}"
    );
  }

  #[dps_auth_db_test]
  async fn test_update_user_unauthenticated() {
    let session_context = SessionContext::new(None);

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: 1, name: "test") {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("No valid session"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_forbidden() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let regular_user = create_test_user_with_databases(&databases, "regular", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: 1, name: "test") {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_not_found() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: 999, name: "test") {
          id
          name
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_validation_error_short_username() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, name: "ab") {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Validation error"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_validation_error_short_password() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(
          id: $USER_ID,
          password: "123",
          passwordConfirmation: "123"
        ) {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Validation error"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_conflict() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;
    let _existing_user =
      create_test_user_with_databases(&databases, "existinguser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, name: "existinguser") {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("already in use"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_password_confirmation_only_password() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, password: "newpass123") {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("must both be provided"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_password_confirmation_only_confirmation() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID, passwordConfirmation: "newpass123") {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("must both be provided"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_password_confirmation_mismatch() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(
          id: $USER_ID,
          password: "newpass123",
          passwordConfirmation: "different123"
        ) {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("do not match"));
  }

  #[dps_auth_db_test]
  async fn test_update_user_no_updates() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user = create_test_user_with_databases(&databases, "targetuser", user_role_id).await;

    let session_payload = ServiceSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateUserResolver;
    let schema = create_test_mutation_schema(
      mutation,
      databases.clone(),
      Some(session_context),
      Some(create_test_config()),
    );

    let query = r#"
      mutation {
        updateUser(id: $USER_ID) {
          id
          name
        }
      }
    "#
    .replace("$USER_ID", &target_user.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let user_data = &data["updateUser"];

    assert_eq!(user_data["id"].as_i64().unwrap(), target_user.id);
    assert_eq!(user_data["name"].as_str().unwrap(), "targetuser");
  }
}
