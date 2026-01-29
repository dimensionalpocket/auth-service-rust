use crate::services::GetServerTimestampService;
use async_graphql::{Object, Result};
use tracing::instrument;

/// Server timestamp query resolver providing current server time information
#[derive(Default, Debug)]
pub struct ServerTimestampResolver;

#[Object]
impl ServerTimestampResolver {
  /// Returns the current server timestamp in milliseconds since Unix epoch.
  ///
  /// This timestamp represents the exact moment the server processed this request,
  /// which is useful for:
  /// - Client-server time synchronization
  /// - Debugging and logging purposes  
  /// - Measuring request processing time
  /// - Ensuring data consistency across distributed systems
  ///
  /// The returned value is a string representation of milliseconds since
  /// January 1, 1970 00:00:00 UTC (Unix epoch).
  ///
  /// Example response: "1706356800000"
  #[instrument]
  #[graphql(name = "serverTimestamp")]
  async fn server_timestamp(&self) -> Result<String> {
    let timestamp = GetServerTimestampService::run();
    Ok(timestamp.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::create_test_query_schema;

  #[tokio::test]
  async fn test_server_timestamp_calls_service() {
    let query = ServerTimestampResolver;
    let schema = create_test_query_schema(query, None, None, None);
    let result = schema.execute("{ serverTimestamp }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let timestamp_str = data["serverTimestamp"].as_str().unwrap();
    let timestamp: u64 = timestamp_str.parse().unwrap();
    assert!(timestamp > 0);
  }

  #[tokio::test]
  async fn test_server_timestamp_returns_string() {
    let query = ServerTimestampResolver;
    let schema = create_test_query_schema(query, None, None, None);
    let result = schema.execute("{ serverTimestamp }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let timestamp_str = data["serverTimestamp"].as_str().unwrap();
    // Should be parseable as u64
    assert!(timestamp_str.parse::<u64>().is_ok());
  }
}
