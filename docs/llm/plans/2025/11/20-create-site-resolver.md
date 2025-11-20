# Plan: create-site-model

Goal
Implement a new GraphQL mutation resolver createSite and a SiteService::create_site following patterns in [`src/graphql/resolvers/create_user.rs`](src/graphql/resolvers/create_user.rs:1) and
[`src/services/user_service.rs`](src/services/user_service.rs:1). The resolver will enforce
`can_create_site` permission via [`src/services/user_role_service.rs`](src/services/user_role_service.rs:1).

Files to create or modify
- Create: [`src/services/site_service.rs`](src/services/site_service.rs:1)
- Modify: [`src/services/mod.rs`](src/services/mod.rs:1) (export SiteService)
- Create: [`src/graphql/resolvers/create_site.rs`](src/graphql/resolvers/create_site.rs:1)
- Modify: [`src/graphql/resolvers/mod.rs`](src/graphql/resolvers/mod.rs:1) (add module & export)
- Modify: [`src/graphql/schema.rs`](src/graphql/schema.rs:1) (include CreateSiteResolver in Mutation)
- Create tests: near [`src/graphql/resolvers/create_site.rs`](src/graphql/resolvers/create_site.rs:1) like create_user tests

Design notes
- Follow the same error mapping and validation approach used by `UserService` and `CreateUserResolver`.
- Use `CreateSiteQuery::run` at [`src/queries/sites/create_site.rs`](src/queries/sites/create_site.rs:1) to persist data.
- Use `sqlx::SqlitePool` from GraphQL context like in `create_user.rs`.
- Use `UserRoleService::check_user_permission(pool, &user, "can_create_site")` to authorize.
- If permission check returns false, return a generic GraphQL error message "Forbidden".
- Map database/query errors to a narrow end-user message: "Failed to create site".
- Keep implementation limited to files listed; do not add new dependencies.
- Database migrations for sites table must already be run (exists in codebase).

Service: SiteService
- New file: [`src/services/site_service.rs`](src/services/site_service.rs:1)
- Provide an error enum `SiteError` similar to `UserError` with variants:
  - `SlugAlreadyExists(String)` (optional)
  - `DatabaseError(sqlx::Error)`
  - `ValidationError(String)`
- Public struct `SiteService` with:
  - async fn create_site(pool: &SqlitePool, data: CreateSiteData) -> Result<Site, SiteError>
- Responsibilities:
  - Validate `slug`: must be 3..=20 characters, start with a letter, may contain letters, numbers, hyphen, and underscore, and must end with a letter or number
  - Delegate creation to `CreateSiteQuery::run(pool, data).await`
  - Map sqlx errors to `SiteError::DatabaseError` and surface specific uniqueness errors if desired

Example SiteService::create_site (sample)
```rust
use crate::models::Site;
use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
use sqlx::SqlitePool;

pub enum SiteError {
  DatabaseError(sqlx::Error),
  ValidationError(String),
}

pub struct SiteService;

impl SiteService {
  pub async fn create_site(pool: &SqlitePool, data: CreateSiteData) -> Result<Site, SiteError> {
    // Basic validation
    if data.slug.trim().is_empty() {
      return Err(SiteError::ValidationError("Slug cannot be empty".to_string()));
    }
    // Delegate to query
    CreateSiteQuery::run(pool, data)
      .await
      .map_err(SiteError::DatabaseError)
  }
}
```

GraphQL resolver: createSite
- New file: [`src/graphql/resolvers/create_site.rs`](src/graphql/resolvers/create_site.rs:1)
- Follow `CreateUserResolver` structure:
  - `CreateSiteInput` (slug, subdomain, port, protocol, metadata_json)
  - `CreateSiteResponse` (id, slug, subdomain, port, protocol, metadata_json, created_ts, updated_ts)
  - `CreateSiteResolver` with method `create_site(&self, ctx: &Context<'_>, input: CreateSiteInput) -> Result<CreateSiteResponse>`
- Implementation steps inside resolver:
  1. Get `SqlitePool` from `ctx.data::<SqlitePool>()?`
  2. Get `SessionContext` from GraphQL context using `SessionContext::from_context(ctx)?`
  3. Extract user_id from session: `session_context.user_id().ok_or("Authentication required")?`
  4. Fetch full User object: `GetUserByIdQuery::run(pool, user_id).await?.ok_or("User not found")?`
  5. Call `UserRoleService::check_user_permission(pool, &user, "can_create_site").await` and if false return `Err(async_graphql::Error::new("Forbidden"))`
  6. Build `CreateSiteData` from input and call `SiteService::create_site(pool, create_data).await`
  7. On success map `Site` -> `CreateSiteResponse`. On service errors, log and return `Err(async_graphql::Error::new("Failed to create site"))`

Example resolver (sample)
```rust
use crate::services::{SiteService, UserRoleService};
use crate::models::{Site, User};
use crate::queries::sites::CreateSiteData;
use crate::queries::users::GetUserByIdQuery;
use crate::middleware::session::SessionContext;
use async_graphql::{Context, InputObject, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(InputObject)]
pub struct CreateSiteInput {
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: Option<String>,
  pub metadata_json: Option<String>,
}

#[derive(async_graphql::SimpleObject)]
pub struct CreateSiteResponse {
  pub id: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
  pub metadata_json: Option<String>,
  pub created_ts: i64,
  pub updated_ts: i64,
}

pub struct CreateSiteResolver;

#[Object]
impl CreateSiteResolver {
  #[instrument(skip(ctx, input), fields(slug = %input.slug))]
  async fn create_site(&self, ctx: &Context<'_>, input: CreateSiteInput) -> Result<CreateSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context and extract user
    let session_context = SessionContext::from_context(ctx)?;
    let user_id = session_context.user_id().ok_or("Authentication required")?;
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(|_| "Failed to fetch user")?
      .ok_or("User not found")?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_create_site").await?;
    if !allowed {
      return Err(async_graphql::Error::new("Forbidden"));
    }

    let create_data = CreateSiteData {
      slug: input.slug,
      subdomain: input.subdomain,
      port: input.port,
      protocol: input.protocol,
      metadata_json: input.metadata_json,
    };

    match SiteService::create_site(pool, create_data).await {
      Ok(site) => Ok(CreateSiteResponse {
        id: site.id,
        slug: site.slug,
        subdomain: site.subdomain,
        port: site.port,
        protocol: site.protocol,
        metadata_json: site.metadata_json,
        created_ts: site.created_ts,
        updated_ts: site.updated_ts,
      }),
      Err(err) => {
        tracing::error!("Failed to create site: {:?}", err);
        Err(async_graphql::Error::new("Failed to create site"))
      }
    }
  }
}
```

Exports and schema wiring
- Update [`src/services/mod.rs`](src/services/mod.rs:1) to `pub mod site_service;` and `pub use site_service::SiteService;`
- Update [`src/graphql/resolvers/mod.rs`](src/graphql/resolvers/mod.rs:1) to include `pub mod create_site;` and `pub use create_site::CreateSiteResolver;`
- Update [`src/graphql/schema.rs`](src/graphql/schema.rs:1) Mutation to include `CreateSiteResolver` in the MergedObject and constructor, similar to `CreateUserResolver`.

Tests
- Add tests in `#[cfg(test)] mod tests` inside `create_site.rs` similar to `create_user.rs`:
  - test_create_site_success: create admin user with can_create_site permission, create session context, create mutation using schema builder, assert no errors and response fields match
  - test_create_site_forbidden: create regular user without can_create_site permission, create session context, assert GraphQL error message equals "Forbidden"
  - test_create_site_unauthenticated: test with no session context, assert authentication error
- Use existing `create_test_database()` helper from `crate::database::test_utils`.

Implementation constraints
- Do not add new crates.
- Use the established pattern for user retrieval via SessionContext and GetUserByIdQuery.
- Keep messages to end users generic (Forbidden / Failed to create site) to avoid leaking internal details.

Next steps after plan approval
1. Create [`src/services/site_service.rs`](src/services/site_service.rs:1) with implementation and tests.
2. Export service in [`src/services/mod.rs`](src/services/mod.rs:1).
3. Create resolver [`src/graphql/resolvers/create_site.rs`](src/graphql/resolvers/create_site.rs:1) and tests.
4. Wire up modules and schema as described.
5. Run `cargo test` and fix compile issues.

End of plan
