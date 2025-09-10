use crate::graphql::mutation::Mutation;
use crate::graphql::query::Query;
use async_graphql::{EmptySubscription, Schema};

/// GraphQL schema type definition for the Dimensional Pocket Auth Service
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Creates and returns a GraphQL schema builder for the auth service.
///
/// This function initializes a GraphQL schema builder with:
/// - Query resolver: Handles all read operations (server timestamp, user queries)
/// - Mutation resolver: Handles all write operations (user creation, etc.)
/// - EmptySubscription: Placeholder for future real-time features
///
/// The returned builder can be customized with data context (e.g., database pool)
/// before calling `.finish()` to create the final schema.
///
/// # Examples
///
/// Database-independent usage:
/// ```rust
/// use dp_auth_service::graphql::schema::build_schema;
/// let schema = build_schema().finish();
/// ```
///
/// Database-dependent usage:
/// ```rust,no_run
/// use dp_auth_service::graphql::schema::build_schema;
/// use sqlx::SqlitePool;
/// # async fn example(database_pool: SqlitePool) {
/// let schema = build_schema().data(database_pool).finish();
/// # }
/// ```
pub fn build_schema() -> async_graphql::SchemaBuilder<Query, Mutation, EmptySubscription> {
  Schema::build(Query::new(), Mutation::new(), EmptySubscription)
}
