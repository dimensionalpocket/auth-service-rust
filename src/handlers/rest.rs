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

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_root_handler_returns_ok() {
    let result = root_handler().await;
    assert_eq!(result, "OK");
  }

  #[tokio::test]
  async fn test_health_handler_returns_ok() {
    let result = health_handler().await;
    assert_eq!(result, "OK");
  }
}
