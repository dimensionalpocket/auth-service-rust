use crate::graphql::query::Query;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};

/// GraphQL schema type definition for the Dimensional Pocket Auth Service
pub type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Creates and returns the complete GraphQL schema for the auth service.
///
/// This function initializes the GraphQL schema with:
/// - Query resolver: Handles all read operations (currently server timestamp)
/// - EmptyMutation: Placeholder for future write operations (user auth, etc.)
/// - EmptySubscription: Placeholder for future real-time features
///
/// The schema is fully introspectable and self-documenting through GraphQL's
/// built-in introspection system, accessible via GraphQL Playground in development.
pub fn create_schema() -> AppSchema {
  Schema::build(Query::new(), EmptyMutation, EmptySubscription).finish()
}
