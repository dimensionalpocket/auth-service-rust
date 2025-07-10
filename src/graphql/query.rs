use crate::graphql::queries::GetServerTimestampQuery;
use async_graphql::MergedObject;

/// Root query object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL queries. It combines all
/// individual query resolvers into a single unified interface using MergedObject.
///
/// Available queries:
/// - getServerTimestamp: Get current server time for synchronization
///
/// Future queries will be added here as the service expands to include
/// user authentication, profile management, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampQuery);

impl Query {
  pub fn new() -> Self {
    Self(GetServerTimestampQuery)
  }
}
