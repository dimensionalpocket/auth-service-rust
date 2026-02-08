use axum::http::StatusCode;
use tracing::instrument;

/// Handler for the root endpoint "/"
///
/// Returns a simple "OK" string with 200 status code
#[instrument]
pub async fn root_handler() -> &'static str {
  "OK"
}

/// Handler for the health check endpoint "/health"
///
/// Returns a simple "OK" string with 200 status code
#[instrument]
pub async fn health_handler() -> &'static str {
  "OK"
}

/// Handler for 404 Not Found responses
///
/// Returns a plain "NOT FOUND" message with 404 status code
#[instrument]
pub async fn not_found_handler() -> (StatusCode, &'static str) {
  (StatusCode::NOT_FOUND, "NOT FOUND")
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  #[dps_auth_db_test]
  async fn test_root_handler_returns_ok() {
    let result = root_handler().await;
    assert_eq!(result, "OK");
  }

  #[dps_auth_db_test]
  async fn test_health_handler_returns_ok() {
    let result = health_handler().await;
    assert_eq!(result, "OK");
  }

  #[dps_auth_db_test]
  async fn test_not_found_handler_returns_404() {
    let (status, message) = not_found_handler().await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(message, "NOT FOUND");
  }
}
