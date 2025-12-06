use crate::graphql::resolvers::{
  AddSiteResolver, AuthChangePasswordResolver, AuthLoginResolver, AuthLogoutResolver,
  AuthMeResolver, AuthRegisterResolver, DeleteUserResolver, GetServerTimestampResolver,
  RemoveSiteResolver, SiteResolver, SitesResolver, UpdateSiteResolver, UserResolver, UsersResolver,
};
use async_graphql::{EmptySubscription, MergedObject, Schema};

/// Root query object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL queries. It combines all
/// individual query resolvers into a single unified interface using MergedObject.
///
/// Available queries:
/// - getServerTimestamp: Get current server time for synchronization
/// - authMe: Get current authenticated user profile
/// - sites: List all sites in the database (no authentication required)
/// - site: Get complete site details by ID (admin only, requires can_view_site_details permission)
/// - users: List all users with role information (requires can_list_users permission)
/// - user: Get complete user details by ID (requires can_view_user_details permission)
///
/// Future queries will be added here as the service expands to include
/// user authentication, profile management, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Query(
  GetServerTimestampResolver,
  AuthMeResolver,
  SitesResolver,
  SiteResolver,
  UsersResolver,
  UserResolver,
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
/// - authRegister: Register a new user account
/// - authLogin: Authenticate user and create session
/// - authLogout: Logout user by clearing session cookie
/// - authChangePassword: Change password for authenticated user
/// - addSite: Add a new site to the database (requires can_create_site permission)
/// - updateSite: Update an existing site (requires can_update_site permission)
/// - removeSite: Remove an existing site (requires can_delete_site permission)
/// - deleteUser: Delete an existing user (requires can_delete_user permission)
///
/// Future mutations will be added here as the service expands to include
/// user management, authentication, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Mutation(
  AuthRegisterResolver,
  AuthLoginResolver,
  AuthLogoutResolver,
  AuthChangePasswordResolver,
  AddSiteResolver,
  RemoveSiteResolver,
  UpdateSiteResolver,
  DeleteUserResolver,
);

impl Mutation {
  pub fn new() -> Self {
    Self(
      AuthRegisterResolver,
      AuthLoginResolver,
      AuthLogoutResolver,
      AuthChangePasswordResolver,
      AddSiteResolver,
      RemoveSiteResolver,
      UpdateSiteResolver,
      DeleteUserResolver,
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
