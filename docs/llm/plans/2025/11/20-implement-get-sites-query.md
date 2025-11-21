# Implement getSites GraphQL Query

## Overview
Implement a new GraphQL query `getSites` that returns all sites from the database without requiring authentication. The query should return only the specified fields: id, slug, subdomain, port, and protocol.

## Requirements
1. Return an array with all sites in the database
2. Only return fields: id, slug, subdomain, port, and protocol
3. No authentication required
4. Follow existing code patterns and conventions

## Implementation Plan

### 1. Create GraphQL Type for Site Response
**File**: `src/graphql/types/site.rs` (new)

Create a GraphQL type that represents the site data to be returned:
```rust
use async_graphql::{Object, SimpleObject};

#[derive(SimpleObject, Debug)]
pub struct SiteListing {
    pub id: i64,
    pub slug: String,
    pub subdomain: Option<String>,
    pub port: Option<i64>,
    pub protocol: String,
}
```

Update `src/graphql/types/mod.rs` to include the new type:
```rust
pub mod site;
pub use site::SiteListing;
```

### 2. Create Database Query
**File**: `src/queries/sites/get_all_sites.rs` (new)

Implement the database query to fetch all sites:
```rust
use crate::models::Site;
use sqlx::SqlitePool;

pub struct GetAllSitesQuery;

impl GetAllSitesQuery {
    pub async fn run(pool: &SqlitePool) -> Result<Vec<Site>, sqlx::Error> {
        sqlx::query_as::<_, Site>(
            "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json 
             FROM sites"
        )
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_utils::create_test_database;
    use crate::queries::sites::{CreateSiteData, CreateSiteQuery};

    #[tokio::test]
    async fn test_get_all_sites_empty() {
        let (pool, _tmp) = create_test_database().await;
        let sites = GetAllSitesQuery::run(&pool).await.unwrap();
        assert_eq!(sites.len(), 0);
    }

    #[tokio::test]
    async fn test_get_all_sites_with_data() {
        let (pool, _tmp) = create_test_database().await;
        
        // Create test sites
        let site1_data = CreateSiteData {
            slug: "site1".to_string(),
            subdomain: Some("www".to_string()),
            port: Some(443),
            protocol: None,
            metadata_json: None,
        };
        
        let site2_data = CreateSiteData {
            slug: "site2".to_string(),
            subdomain: None,
            port: Some(80),
            protocol: Some("http".to_string()),
            metadata_json: None,
        };
        
        CreateSiteQuery::run(&pool, site1_data).await.unwrap();
        CreateSiteQuery::run(&pool, site2_data).await.unwrap();
        
        let sites = GetAllSitesQuery::run(&pool).await.unwrap();
        assert_eq!(sites.len(), 2);
    }
}
```

Update `src/queries/sites/mod.rs`:
```rust
pub mod create_site;
pub mod get_all_sites;

pub use create_site::{CreateSiteData, CreateSiteQuery};
pub use get_all_sites::GetAllSitesQuery;
```

### 3. Create Service Layer Method
**File**: `src/services/site_service.rs` (modify)

Add a method to get all sites:
```rust
impl SiteService {
    // ... existing methods ...

    /// Get all sites from the database
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// * `Ok(Vec<Site>)` - Vector of all sites
    /// * `Err(SiteError)` - Database operation failed
    pub async fn get_all_sites(pool: &SqlitePool) -> Result<Vec<Site>, SiteError> {
        GetAllSitesQuery::run(pool).await.map_err(SiteError::DatabaseError)
    }
}
```

Add test for the new method:
```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[tokio::test]
    async fn test_get_all_sites() {
        let (pool, _temp_file) = create_test_database().await;

        // Create test sites
        let data1 = CreateSiteData {
            slug: "test1".to_string(),
            subdomain: Some("www".to_string()),
            port: Some(443),
            protocol: None,
            metadata_json: None,
        };

        let data2 = CreateSiteData {
            slug: "test2".to_string(),
            subdomain: None,
            port: Some(80),
            protocol: Some("http".to_string()),
            metadata_json: None,
        };

        SiteService::create_site(&pool, data1).await.unwrap();
        SiteService::create_site(&pool, data2).await.unwrap();

        let sites = SiteService::get_all_sites(&pool).await.unwrap();
        assert_eq!(sites.len(), 2);
    }
}
```

### 4. Create GraphQL Resolver
**File**: `src/graphql/resolvers/get_sites.rs` (new)

Implement the GraphQL resolver:
```rust
use crate::graphql::types::SiteListing;
use crate::services::SiteService;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// Sites query resolver for retrieving site information
#[derive(Default, Debug)]
pub struct GetSitesResolver;

#[Object]
impl GetSitesResolver {
    /// Returns all sites in the database.
    ///
    /// This query does not require authentication and returns basic site information
    /// including id, slug, subdomain, port, and protocol for all sites.
    ///
    /// Example response:
    /// ```json
    /// {
    ///   "getSites": [
    ///     {
    ///       "id": 1,
    ///       "slug": "example",
    ///       "subdomain": "www",
    ///       "port": 443,
    ///       "protocol": "https"
    ///     }
    ///   ]
    /// }
    /// ```
    #[instrument(skip(self, ctx))]
    async fn get_sites(&self, ctx: &Context<'_>) -> Result<Vec<SiteListing>> {
        let pool = ctx.data::<SqlitePool>()?;
        let sites = SiteService::get_all_sites(pool).await?;
        
        let site_listings: Vec<SiteListing> = sites
            .into_iter()
            .map(|site| SiteListing {
                id: site.id,
                slug: site.slug,
                subdomain: site.subdomain,
                port: site.port,
                protocol: site.protocol,
            })
            .collect();
            
        Ok(site_listings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_utils::create_test_database;
    use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
    use async_graphql::*;

    #[tokio::test]
    async fn test_get_sites_empty() {
        let (pool, _tmp) = create_test_database().await;
        let query = GetSitesResolver;
        let schema = Schema::build(query, EmptyMutation, EmptySubscription)
            .data(pool)
            .finish();
        
        let result = schema.execute("{ getSites { id slug subdomain port protocol } }").await;
        
        assert!(result.errors.is_empty());
        let data = result.data.into_json().unwrap();
        let sites = data["getSites"].as_array().unwrap();
        assert_eq!(sites.len(), 0);
    }

    #[tokio::test]
    async fn test_get_sites_with_data() {
        let (pool, _tmp) = create_test_database().await;
        
        // Create test sites
        let site1_data = CreateSiteData {
            slug: "alpha".to_string(),
            subdomain: Some("www".to_string()),
            port: Some(443),
            protocol: None,
            metadata_json: None,
        };
        
        let site2_data = CreateSiteData {
            slug: "beta".to_string(),
            subdomain: None,
            port: Some(80),
            protocol: Some("http".to_string()),
            metadata_json: None,
        };
        
        CreateSiteQuery::run(&pool, site1_data).await.unwrap();
        CreateSiteQuery::run(&pool, site2_data).await.unwrap();
        
        let query = GetSitesResolver;
        let schema = Schema::build(query, EmptyMutation, EmptySubscription)
            .data(pool)
            .finish();
        
        let result = schema.execute("{ getSites { id slug subdomain port protocol } }").await;
        
        assert!(result.errors.is_empty());
        let data = result.data.into_json().unwrap();
        let sites = data["getSites"].as_array().unwrap();
        assert_eq!(sites.len(), 2);
        
        // Verify field structure
        assert!(sites[0]["id"].is_number());
        assert!(sites[0]["slug"].is_string());
        assert!(sites[0]["subdomain"].is_string() || sites[0]["subdomain"].is_null());
        assert!(sites[0]["port"].is_number() || sites[0]["port"].is_null());
        assert!(sites[0]["protocol"].is_string());
    }
}
```

### 5. Update Resolver Module
**File**: `src/graphql/resolvers/mod.rs` (modify)

Add the new resolver:
```rust
pub mod create_session;
pub mod create_site;
pub mod create_user;
pub mod get_current_session;
pub mod get_server_timestamp;
pub mod get_sites;

pub use create_session::CreateSessionResolver;
pub use create_site::CreateSiteResolver;
pub use create_user::CreateUserResolver;
pub use get_current_session::GetCurrentSessionResolver;
pub use get_server_timestamp::GetServerTimestampResolver;
pub use get_sites::GetSitesResolver;
```

### 6. Update GraphQL Schema
**File**: `src/graphql/schema.rs` (modify)

Add the new resolver to the Query struct:
```rust
use crate::graphql::resolvers::{
  CreateSessionResolver, CreateSiteResolver, CreateUserResolver, GetCurrentSessionResolver,
  GetServerTimestampResolver, GetSitesResolver,
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
pub struct Query(GetServerTimestampResolver, GetCurrentSessionResolver, GetSitesResolver);

impl Query {
  pub fn new() -> Self {
    Self::default()
  }
}

// ... rest of the file remains unchanged ...
```

Update the documentation comments to include the new query.

### 7. Update GraphQL Types Module
**File**: `src/graphql/types/mod.rs` (modify)

Ensure the module structure is correct:
```rust
pub mod session_payload;
pub mod site;

pub use session_payload::SessionPayload;
pub use site::SiteListing;
```

## Testing Strategy

### Unit Tests
1. **Database Query Tests**: Test `GetAllSitesQuery` with empty and populated databases
2. **Service Layer Tests**: Test `SiteService::get_all_sites` method
3. **Resolver Tests**: Test GraphQL resolver with mock data and verify response structure

### Integration Tests
1. **GraphQL Schema Tests**: Test the complete GraphQL query execution
2. **End-to-End Tests**: Test the query through the full GraphQL handler

### Manual Testing
Test the GraphQL query using a GraphQL client:
```graphql
query {
  getSites {
    id
    slug
    subdomain
    port
    protocol
  }
}
```

## Files to Create/Modify

### New Files
1. `src/graphql/types/site.rs` - GraphQL type definition
2. `src/queries/sites/get_all_sites.rs` - Database query implementation
3. `src/graphql/resolvers/get_sites.rs` - GraphQL resolver

### Modified Files
1. `src/graphql/types/mod.rs` - Export new type
2. `src/queries/sites/mod.rs` - Export new query
3. `src/services/site_service.rs` - Add get_all_sites method
4. `src/graphql/resolvers/mod.rs` - Export new resolver
5. `src/graphql/schema.rs` - Add resolver to Query struct

## Implementation Notes

1. **No Authentication**: The query should be accessible without any authentication middleware
2. **Field Selection**: Only return the specified fields (id, slug, subdomain, port, protocol)
3. **Error Handling**: Follow existing error handling patterns using custom error types
4. **Testing**: Include comprehensive tests for all layers
5. **Documentation**: Add proper documentation comments following existing patterns

## Dependencies

No new dependencies are required. The implementation uses existing:
- `async_graphql` for GraphQL types and resolvers
- `sqlx` for database operations
- `tracing` for instrumentation
- `chrono` for timestamps (if needed)

## Security Considerations

1. **No Sensitive Data**: The query only returns non-sensitive site information
2. **No Authentication**: Intentionally public query, no security concerns
3. **Rate Limiting**: Consider adding rate limiting in production if needed
4. **Data Exposure**: Ensure no sensitive fields like metadata_json are exposed

## Performance Considerations

1. **Pagination**: Not needed for small, fixed-size datasets
2. **Caching**: Optional in-memory caching can be considered for frequently accessed data, but not critical for small datasets

## Future Enhancements

1. **Filtering**: Add filtering capabilities (by protocol, port, etc.)
2. **Field Selection**: Optimize database queries based on requested GraphQL fields
3. **Validation**: Add input validation for site configuration constraints