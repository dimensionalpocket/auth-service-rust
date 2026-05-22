# Plan: Create updateSite Resolver

## Overview
Create a new GraphQL resolver `updateSite` that allows partial updates to site records using PATCH semantics. The resolver will update only the fields provided in the payload, leaving unspecified fields unchanged. Explicit null values will set the corresponding database columns to NULL.

## Requirements
- Takes a site ID and partial update payload
- All payload fields are optional (PATCH semantics)
- Missing fields are not updated
- Explicit null values set database columns to NULL
- `createdTs` and `updatedTs` cannot be updated directly
- `updatedTs` automatically updates to current timestamp when any changes occur
- Requires "can_update_site" permission
- Follows existing codebase patterns

## Files to Create/Modify

### 1. Create Query Object: `src/queries/sites/update_site.rs`
```rust
use crate::models::Site;
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct UpdateSiteData {
    pub id: i64,
    pub slug: Option<String>,
    pub subdomain: Option<Option<String>>, // Option<Option<T>> for explicit null handling
    pub port: Option<Option<i64>>,
    pub protocol: Option<String>,
    pub metadata_json: Option<Option<String>>,
}

pub struct UpdateSiteQuery;

impl UpdateSiteQuery {
    pub async fn run(pool: &SqlitePool, data: UpdateSiteData) -> Result<Option<Site>, sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        
        // Build dynamic query using sqlx::query() with conditional binds
        let mut query = sqlx::query("UPDATE sites SET updated_ts = ?").bind(now);
        let mut set_clauses = Vec::new();
        let mut binds: Vec<Box<dyn sqlx::Encode<sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send>> = Vec::new();
        
        // Add conditional SET clauses based on provided fields
        // Return updated site or None if not found
    }
}
```

### 2. Update Query Module: `src/queries/sites/mod.rs`
```rust
pub mod create_site;
pub mod get_all_sites;
pub mod update_site;

pub use create_site::{CreateSiteData, CreateSiteQuery};
pub use get_all_sites::GetAllSitesQuery;
pub use update_site::{UpdateSiteData, UpdateSiteQuery};
```

### 3. Update Site Service: `src/services/site_service.rs`
Add new method to SiteService:
```rust
/// Update an existing site with partial data
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Site ID to update
/// * `data` - Partial update data
///
/// # Returns
/// * `Ok(Some(Site))` - Successfully updated site
/// * `Ok(None)` - Site not found
/// * `Err(SiteError)` - Update failed due to validation, uniqueness, or database error
pub async fn update_site(pool: &SqlitePool, id: i64, data: UpdateSiteData) -> Result<Option<Site>, SiteError> {
    // Validate slug if provided
    // Check slug uniqueness if being updated
    // Delegate to UpdateSiteQuery
}
```

Add new error variant if needed:
```rust
pub enum SiteError {
    // ... existing variants
    SiteNotFound(i64),
}
```

### 4. Create Resolver: `src/graphql/resolvers/update_site.rs`
```rust
use crate::middleware::session::SessionContext;
use crate::queries::sites::UpdateSiteData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use async_graphql::{Context, InputObject, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(InputObject)]
pub struct UpdateSiteInput {
    pub id: i64,
    pub slug: Option<String>,
    pub subdomain: Option<String>,
    pub port: Option<i64>,
    pub protocol: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(async_graphql::SimpleObject)]
pub struct UpdateSiteResponse {
    pub id: i64,
    pub slug: String,
    pub subdomain: Option<String>,
    pub port: Option<i64>,
    pub protocol: String,
    pub metadata_json: Option<String>,
    pub created_ts: i64,
    pub updated_ts: i64,
}

#[derive(Default, Debug)]
pub struct UpdateSiteResolver;

#[Object]
impl UpdateSiteResolver {
    #[instrument(skip(ctx, input), fields(id = %input.id))]
    async fn update_site(
        &self,
        ctx: &Context<'_>,
        input: UpdateSiteInput,
    ) -> Result<Option<UpdateSiteResponse>> {
        // Authentication and permission checks
        // Convert Input to UpdateSiteData with proper null handling
        // Call SiteService::update_site
        // Handle errors and return response
    }
}
```

### 5. Update Resolver Module: `src/graphql/resolvers/mod.rs`
```rust
pub mod create_session;
pub mod create_site;
pub mod create_user;
pub mod get_current_session;
pub mod get_server_timestamp;
pub mod get_sites;
pub mod update_site;

pub use create_session::CreateSessionResolver;
pub use create_site::CreateSiteResolver;
pub use create_user::CreateUserResolver;
pub use get_current_session::GetCurrentSessionResolver;
pub use get_server_timestamp::GetServerTimestampResolver;
pub use get_sites::GetSitesResolver;
pub use update_site::UpdateSiteResolver;
```

### 6. Update GraphQL Schema: `src/graphql/schema.rs`
```rust
use crate::graphql::resolvers::{
  CreateSessionResolver, CreateSiteResolver, CreateUserResolver, GetCurrentSessionResolver,
  GetServerTimestampResolver, GetSitesResolver, UpdateSiteResolver,
};

// Update Mutation struct
#[derive(MergedObject, Default)]
pub struct Mutation(
  CreateUserResolver,
  CreateSessionResolver,
  CreateSiteResolver,
  UpdateSiteResolver,
);

impl Mutation {
  pub fn new() -> Self {
    Self(
      CreateUserResolver,
      CreateSessionResolver,
      CreateSiteResolver,
      UpdateSiteResolver,
    )
  }
}
```

## Implementation Details

### Dynamic SQL Query Construction with sqlx
The `UpdateSiteQuery::run` method will use sqlx's query builder:

```rust
pub async fn run(pool: &SqlitePool, data: UpdateSiteData) -> Result<Option<Site>, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    
    // Start with base query
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE sites SET updated_ts = ");
    query_builder.push_bind(now);
    
    let mut has_updates = false;
    
    // Conditionally add SET clauses
    if let Some(slug) = data.slug {
        query_builder.push(", slug = ?");
        query_builder.push_bind(slug);
        has_updates = true;
    }
    
    if let Some(subdomain) = data.subdomain {
        query_builder.push(", subdomain = ?");
        query_builder.push_bind(subdomain);
        has_updates = true;
    }
    
    if let Some(port) = data.port {
        query_builder.push(", port = ?");
        query_builder.push_bind(port);
        has_updates = true;
    }
    
    if let Some(protocol) = data.protocol {
        query_builder.push(", protocol = ?");
        query_builder.push_bind(protocol);
        has_updates = true;
    }
    
    if let Some(metadata_json) = data.metadata_json {
        query_builder.push(", metadata_json = ?");
        query_builder.push_bind(metadata_json);
        has_updates = true;
    }
    
    // Add WHERE clause and execute
    query_builder.push(" WHERE id = ?");
    query_builder.push_bind(data.id);
    
    let query = query_builder.build();
    
    let result = query.execute(pool).await?;
    
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    
    // Return updated site
    sqlx::query_as::<_, Site>(
        "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE id = ?"
    )
    .bind(data.id)
    .fetch_one(pool)
    .await
    .map(Some)
}
```
The `UpdateSiteQuery::run` method will:
1. Start with base UPDATE statement: `UPDATE sites SET updated_ts = ?`
2. Conditionally add SET clauses for each provided field
3. Use `Option<Option<T>>` pattern to distinguish between "not provided" and "explicitly null"
4. Add WHERE clause for site ID
5. Execute query and return updated site

### Null Handling Strategy
- `None` in input field → Don't include in UPDATE query
- `Some(None)` in UpdateSiteData → Set database column to NULL
- `Some(Some(value))` in UpdateSiteData → Set database column to value

### Validation Rules
- If slug is provided, apply same validation as create_site
- Check slug uniqueness (excluding current site)
- Validate protocol if provided
- Validate port range if provided

### Error Handling
- Site not found → Return Ok(None) from service, GraphQL null response
- Permission denied → GraphQL error with "Forbidden" message
- Validation errors → GraphQL error with validation message
- Database errors → GraphQL error with generic message

### Testing Strategy
1. Unit tests for UpdateSiteQuery with various field combinations
2. Unit tests for SiteService::update_site including validation
3. Integration tests for UpdateSiteResolver:
   - Successful partial update
   - Update with explicit null values
   - Update non-existent site
   - Permission denied scenarios
   - Authentication required scenarios
   - Slug uniqueness validation

## GraphQL Schema Addition
```graphql
input UpdateSiteInput {
  id: ID!
  slug: String
  subdomain: String
  port: Int
  protocol: String
  metadataJson: String
}

type UpdateSiteResponse {
  id: ID!
  slug: String!
  subdomain: String
  port: Int
  protocol: String!
  metadataJson: String
  createdTs: Int!
  updatedTs: Int!
}

type Mutation {
  # ... existing mutations
  updateSite(input: UpdateSiteInput!): UpdateSiteResponse
}
```

## Validation Examples
- Update only slug: `{id: 1, slug: "new-slug"}`
- Set subdomain to null: `{id: 1, subdomain: null}`
- Update multiple fields: `{id: 1, slug: "new", port: 8080}`
- No-op update (empty payload): `{id: 1}` → Should still update updated_ts

## Success Criteria
1. All tests pass
2. Code follows existing patterns and conventions
3. Proper error handling for all edge cases
4. Linting passes (`cargo clippy --allow-dirty --fix` and `cargo fmt`)
5. GraphQL mutation works as expected in manual testing
