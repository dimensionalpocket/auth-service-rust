use dps_auth_api::middleware::session::SessionContext;
use dps_auth_api::services::CreateSessionForUserService;
use dps_auth_api::test_utils as test_utils;
use dps_auth_api::test_utils::{
  create_test_role_model_with_databases, create_test_user_with_databases,
};
use dps_auth_api::utils::session_context_sub_to_user_id;
use dps_auth_session::DpsAuthSession;
use dps_auth_test_macros::dps_auth_db_test;
use dps_config::DpsConfig;

/// Creates a DpsConfig with custom session functions that use a JSON sub format:
/// `{"user_id": <id>, "roles": ["admin"]}`
fn create_config_with_custom_session_functions() -> DpsConfig {
  let mut config = DpsConfig::new();
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));

  // Custom session_user_to_sub_fn: returns JSON with user_id and roles
  config.set_session_user_to_sub_fn(Box::new(|user_record: &serde_json::Value| {
    let user_id = user_record
      .get("id")
      .and_then(|v| v.as_i64())
      .ok_or_else(|| anyhow::anyhow!("Missing or invalid user id"))?;

    let sub = serde_json::json!({
      "user_id": user_id,
      "roles": ["admin"]
    })
    .to_string();

    Ok(sub)
  }));

  // Custom session_sub_to_user_id_fn: parses JSON sub to extract user_id
  config.set_session_sub_to_user_id_fn(Box::new(|sub: &str| {
    let parsed: serde_json::Value =
      serde_json::from_str(sub).map_err(|e| anyhow::anyhow!("Invalid JSON sub: {e}"))?;

    let user_id = parsed
      .get("user_id")
      .and_then(|v| v.as_i64())
      .ok_or_else(|| anyhow::anyhow!("Missing or invalid user_id in sub"))?;

    Ok(user_id)
  }));

  config
}

#[dps_auth_db_test]
async fn test_custom_session_user_to_sub_fn_generates_json_sub() {
  create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
  let user = create_test_user_with_databases(&databases, "testuser", 1).await;

  let config = create_config_with_custom_session_functions();
  let token = CreateSessionForUserService::run(&user, &config).unwrap();

  // Decode token and verify sub is custom JSON format
  let secret = config.get_auth_api_session_secret_bytes().unwrap();
  let payload = DpsAuthSession::decode_token(&token, &secret).unwrap();

  // Parse the sub as JSON and verify structure
  let sub_json: serde_json::Value = serde_json::from_str(&payload.sub).unwrap();
  assert_eq!(sub_json["user_id"].as_i64(), Some(user.id));
  assert_eq!(sub_json["roles"][0].as_str(), Some("admin"));
}

#[dps_auth_db_test]
async fn test_custom_session_sub_to_user_id_fn_extracts_user_id() {
  let config = create_config_with_custom_session_functions();

  // Create a session context with a custom JSON sub
  let custom_sub = serde_json::json!({
    "user_id": 42,
    "roles": ["admin", "editor"]
  })
  .to_string();

  let payload = dps_auth_api::middleware::session::SessionPayload {
    sub: custom_sub,
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(payload));

  let user_id = session_context_sub_to_user_id(&session_context, &config).unwrap();
  assert_eq!(user_id, 42);
}

#[dps_auth_db_test]
async fn test_custom_session_functions_round_trip() {
  create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
  let user = create_test_user_with_databases(&databases, "roundtrip_user", 1).await;

  let config = create_config_with_custom_session_functions();

  // Step 1: Create session token (uses custom session_user_to_sub_fn)
  let token = CreateSessionForUserService::run(&user, &config).unwrap();

  // Step 2: Decode token to get payload
  let secret = config.get_auth_api_session_secret_bytes().unwrap();
  let payload = DpsAuthSession::decode_token(&token, &secret).unwrap();

  // Step 3: Create session context from payload
  let session_context = SessionContext::new(Some(payload));

  // Step 4: Extract user ID (uses custom session_sub_to_user_id_fn)
  let extracted_user_id = session_context_sub_to_user_id(&session_context, &config).unwrap();

  // Verify round-trip: extracted user ID matches original
  assert_eq!(extracted_user_id, user.id);
}

#[dps_auth_db_test]
async fn test_custom_session_sub_to_user_id_fn_invalid_json_returns_error() {
  let config = create_config_with_custom_session_functions();

  let payload = dps_auth_api::middleware::session::SessionPayload {
    sub: "not_json".to_string(),
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(payload));

  let result = session_context_sub_to_user_id(&session_context, &config);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("Invalid JSON sub"));
}

#[dps_auth_db_test]
async fn test_custom_session_sub_to_user_id_fn_missing_user_id_returns_error() {
  let config = create_config_with_custom_session_functions();

  let payload = dps_auth_api::middleware::session::SessionPayload {
    sub: serde_json::json!({"roles": ["admin"]}).to_string(),
    iat: 1000,
    exp: 2000,
  };
  let session_context = SessionContext::new(Some(payload));

  let result = session_context_sub_to_user_id(&session_context, &config);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("Missing or invalid user_id"));
}
