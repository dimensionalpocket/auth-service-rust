use crate::graphql::query::Query;
use crate::graphql::mutation::Mutation;
use async_graphql::{EmptySubscription, Schema};

/// GraphQL schema type definition for the Dimensional Pocket Auth Service
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Creates and returns the complete GraphQL schema for the auth service.
///
/// This function initializes the GraphQL schema with:
/// - Query resolver: Handles all read operations (server timestamp, user queries)
/// - Mutation resolver: Handles all write operations (user creation, etc.)
/// - EmptySubscription: Placeholder for future real-time features
///
/// The schema is fully introspectable and self-documenting through GraphQL's
/// built-in introspection system, accessible via GraphQL Playground in development.
pub fn create_schema() -> AppSchema {
  Schema::build(Query::new(), Mutation::new(), EmptySubscription).finish()
}
