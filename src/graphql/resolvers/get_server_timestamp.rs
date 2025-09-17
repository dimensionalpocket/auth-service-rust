use crate::services::ServerService;
use async_graphql::{Object, Result};
use tracing::instrument;

/// Server timestamp query resolver providing current server time information
#[derive(Default, Debug)]
pub struct GetServerTimestampResolver;

#[Object]
impl GetServerTimestampResolver {
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
  async fn get_server_timestamp(&self) -> Result<String> {
    let timestamp = ServerService::get_server_timestamp();
    Ok(timestamp.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use async_graphql::*;

  #[tokio::test]
  async fn test_get_server_timestamp_calls_service() {
    let query = GetServerTimestampResolver;
    let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
    let result = schema.execute("{ getServerTimestamp }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let timestamp_str = data["getServerTimestamp"].as_str().unwrap();
    let timestamp: u64 = timestamp_str.parse().unwrap();
    assert!(timestamp > 0);
  }

  #[tokio::test]
  async fn test_get_server_timestamp_returns_string() {
    let query = GetServerTimestampResolver;
    let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
    let result = schema.execute("{ getServerTimestamp }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let timestamp_str = data["getServerTimestamp"].as_str().unwrap();
    // Should be parseable as u64
    assert!(timestamp_str.parse::<u64>().is_ok());
  }
}
