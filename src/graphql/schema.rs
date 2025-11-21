use crate::graphql::resolvers::{
  CreateSessionResolver, CreateSiteResolver, CreateUserResolver, DeleteSiteResolver,
  GetCurrentSessionResolver, GetServerTimestampResolver, GetSitesResolver, UpdateSiteResolver,
};
use async_graphql::{EmptySubscription, MergedObject, Schema};

/// Root query object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL queries. It combines all
/// individual query resolvers into a single unified interface using MergedObject.
///
/// Available queries:
/// - getServerTimestamp: Get current server time for synchronization
/// - getCurrentSession: Get current authenticated user session information
/// - getSites: Get all sites in the database (no authentication required)
///
/// Future queries will be added here as the service expands to include
/// user authentication, profile management, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Query(
  GetServerTimestampResolver,
  GetCurrentSessionResolver,
  GetSitesResolver,
);

impl Query {
  pub fn new() -> Self {
    Self::default()
  }
}

/// Root mutation object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL mutations. It combines all
/// individual mutation resolvers into a single unified interface using MergedObject.
///
/// Available mutations:
/// - createUser: Create a new user account
/// - createSession: Authenticate user and create session (sign-in)
/// - createSite: Create a new site (requires can_create_site permission)
/// - updateSite: Update an existing site (requires can_update_site permission)
/// - deleteSite: Delete an existing site (requires can_delete_site permission)
///
/// Future mutations will be added here as the service expands to include
/// user management, authentication, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Mutation(
  CreateUserResolver,
  CreateSessionResolver,
  CreateSiteResolver,
  DeleteSiteResolver,
  UpdateSiteResolver,
);

impl Mutation {
  pub fn new() -> Self {
    Self(
      CreateUserResolver,
      CreateSessionResolver,
      CreateSiteResolver,
      DeleteSiteResolver,
      UpdateSiteResolver,
    )
  }
}

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
/// use dps_auth_api::graphql::schema::build_schema;
/// let schema = build_schema().finish();
/// ```
///
/// Database-dependent usage:
/// ```rust,no_run
/// use dps_auth_api::graphql::schema::build_schema;
/// use sqlx::SqlitePool;
/// # async fn example(database_pool: SqlitePool) {
/// let schema = build_schema().data(database_pool).finish();
/// # }
/// ```
pub fn build_schema() -> async_graphql::SchemaBuilder<Query, Mutation, EmptySubscription> {
  Schema::build(Query::new(), Mutation::new(), EmptySubscription)
}
