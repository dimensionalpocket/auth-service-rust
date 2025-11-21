# 21-implement-delete-site-resolver.md

## Overview
Implement a deleteSite GraphQL resolver that requires "can_delete_site" permission. This includes creating the resolver, service method, database query, and registering the mutation in the GraphQL schema.

## Implementation Plan

### 1. Database Query Module
**File**: `src/queries/sites/delete_site.rs`
- Create `DeleteSiteQuery` struct with `run` method
- Take pool and site_id parameters
- Execute DELETE SQL query with rows_affected check
- Return Result<(), sqlx::Error>

### 2. Site Service Update
**File**: `src/services/site_service.rs`
- Add static `delete_site` method to `SiteService`
- Take pool and site_id parameters
- Call the database query
- Map sqlx::Error::RowNotFound to SiteError::SiteNotFound
- Return Result<(), SiteError>

### 3. GraphQL Resolver
**File**: `src/graphql/resolvers/delete_site.rs`
- Create `DeleteSiteResolver` struct with `delete_site` method
- Accept site_id as input parameter
- Extract user from GraphQL context using SessionContext
- Check "can_delete_site" permission using UserRoleService
- Call SiteService::delete_site()
- Return DeleteSiteResponse or appropriate error

### 4. GraphQL Schema Update
**File**: `src/graphql/schema.rs`
- Add `DeleteSiteResolver` to the Mutation struct
- Update the Mutation::new() function to include `DeleteSiteResolver`
- Update documentation comments to include deleteSite mutation

### 5. Resolver Module Update
**File**: `src/graphql/resolvers/mod.rs`
- Add `delete_site` module export
- Include `DeleteSiteResolver` in the public exports

### 6. Permission System
- The "can_delete_site" permission string is used in the resolver
- No schema changes needed for permissions (they're checked at runtime)

## Code Samples

### Database Query (delete_site.rs)
```rust
use sqlx::SqlitePool;

pub struct DeleteSiteQuery;

impl DeleteSiteQuery {
    pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<(), sqlx::Error> {
        let result = sqlx::query("DELETE FROM sites WHERE id = ?")
            .bind(site_id)
            .execute(pool)
            .await?;
            
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_utils::create_test_database;
    use crate::queries::sites::{CreateSiteData, CreateSiteQuery};

    #[tokio::test]
    async fn test_delete_site_success() {
        let (pool, _tmp) = create_test_database().await;

        // Create a site first
        let create_data = CreateSiteData {
            slug: "test-site".to_string(),
            subdomain: None,
            port: None,
            protocol: None,
            metadata_json: None,
        };
        let site = CreateSiteQuery::run(&pool, create_data).await.unwrap();

        // Delete the site
        DeleteSiteQuery::run(&pool, site.id).await.unwrap();

        // Verify site is deleted
        let result = sqlx::query("SELECT COUNT(*) FROM sites WHERE id = ?")
            .bind(site.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let count: i64 = result.get(0);
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_site_not_found() {
        let (pool, _tmp) = create_test_database().await;

        // Try to delete non-existent site
        let result = DeleteSiteQuery::run(&pool, 999).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));
    }
}
```

### Service Method (site_service.rs)
```rust
impl SiteService {
    pub async fn delete_site(
        pool: &SqlitePool,
        site_id: i64,
    ) -> Result<(), SiteError> {
        queries::sites::delete_site::DeleteSiteQuery::run(pool, site_id)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => SiteError::SiteNotFound(site_id),
                _ => SiteError::DatabaseError(e.to_string()),
            })
    }
}
```

### GraphQL Resolver (delete_site.rs)
```rust
use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site deletion response
#[derive(async_graphql::SimpleObject)]
pub struct DeleteSiteResponse {
    /// Whether the deletion was successful
    pub success: bool,
    /// Success or error message
    pub message: String,
}

/// Site deletion mutation resolver
#[derive(Default, Debug)]
pub struct DeleteSiteResolver;

#[Object]
impl DeleteSiteResolver {
    /// Deletes an existing site.
    ///
    /// This mutation:
    /// - Requires user authentication
    /// - Checks if the user has "can_delete_site" permission
    /// - Deletes the site from the database
    /// - Returns success confirmation
    ///
    /// # Arguments
    /// * `site_id` - ID of the site to delete
    ///
    /// # Returns
    /// * `DeleteSiteResponse` - Success confirmation
    ///
    /// # Errors
    /// * Returns "Authentication required" if user is not authenticated
    /// * Returns "User not found" if authenticated user doesn't exist in database
    /// * Returns "Forbidden" if user lacks "can_delete_site" permission
    /// * Returns GraphQL error if site is not found
    /// * Returns GraphQL error if database operation fails
    #[instrument(skip(ctx), fields(site_id = %site_id))]
    async fn delete_site(
        &self,
        ctx: &Context<'_>,
        site_id: i64,
    ) -> Result<DeleteSiteResponse> {
        let pool = ctx.data::<SqlitePool>()?;

        // Get session context and extract user
        let session_context = SessionContext::from_context(ctx)?;
        let user_id = session_context.user_id().ok_or("Authentication required")?;
        let user = GetUserByIdQuery::run(pool, user_id)
            .await
            .map_err(|_| "Failed to fetch user")?
            .ok_or("User not found")?;

        // Check permissions
        let allowed = UserRoleService::check_user_permission(pool, &user, "can_delete_site").await?;
        if !allowed {
            return Err(async_graphql::Error::new("Forbidden"));
        }

        match SiteService::delete_site(pool, site_id).await {
            Ok(_) => Ok(DeleteSiteResponse {
                success: true,
                message: "Site deleted successfully".to_string(),
            }),
            Err(SiteError::SiteNotFound(id)) => Err(async_graphql::Error::new(format!(
                "Site with ID {id} not found"
            ))),
            Err(err) => {
                tracing::error!("Failed to delete site: {}", err);
                Err(async_graphql::Error::new("Failed to delete site"))
            }
        }
    }
}
```

### Schema Addition
```rust
// In schema.rs, update Mutation struct and constructor
#[derive(MergedObject, Default)]
pub struct Mutation(
  CreateUserResolver,
  CreateSessionResolver,
  CreateSiteResolver,
  UpdateSiteResolver,
  DeleteSiteResolver,  // Add this
);

impl Mutation {
  pub fn new() -> Self {
    Self(
      CreateUserResolver,
      CreateSessionResolver,
      CreateSiteResolver,
      UpdateSiteResolver,
      DeleteSiteResolver,  // Add this
    )
  }
}

// Also update the documentation comments to include:
// - deleteSite: Delete an existing site (requires can_delete_site permission)
```

## Dependencies
- No new external dependencies required
- Uses existing sqlx, async-graphql, and service patterns
- Follows existing error handling patterns

## Testing Considerations
- Test successful deletion
- Test permission denied scenarios
- Test non-existent site deletion
- Test database error handling
- Integration test with GraphQL endpoint

## Files to Modify/Create
1. `src/queries/sites/delete_site.rs` (new)
2. `src/queries/sites/mod.rs` (update)
3. `src/services/site_service.rs` (update)
4. `src/graphql/resolvers/delete_site.rs` (new)
5. `src/graphql/resolvers/mod.rs` (update)
6. `src/graphql/schema.rs` (update)

## Backwards Compatibility
- No breaking changes to existing functionality
- New mutation is additive only
- Existing site queries and mutations remain unchanged