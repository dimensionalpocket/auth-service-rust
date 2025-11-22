use crate::models::Site;
use crate::queries::sites::{
  CreateSiteData, CreateSiteQuery, DeleteSiteQuery, GetAllSitesQuery, UpdateSiteData,
  UpdateSiteQuery,
};
use sqlx::SqlitePool;

/// Custom error type for site operations
#[derive(Debug)]
pub enum SiteError {
  /// Slug is already in use
  SlugAlreadyExists(String),
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Input validation failed
  ValidationError(String),
  /// Site not found
  SiteNotFound(i64),
}

impl std::fmt::Display for SiteError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SiteError::SlugAlreadyExists(slug) => {
        write!(f, "Slug '{slug}' is already in use")
      }
      SiteError::DatabaseError(err) => write!(f, "Database error: {err}"),
      SiteError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
      SiteError::SiteNotFound(id) => write!(f, "Site with ID {id} not found"),
    }
  }
}

impl std::error::Error for SiteError {}

impl From<sqlx::Error> for SiteError {
  fn from(err: sqlx::Error) -> Self {
    SiteError::DatabaseError(err)
  }
}

/// Service for site management operations
pub struct SiteService;

impl SiteService {
  /// Create a new site with validation
  ///
  /// This method validates the input (slug) and delegates creation to the query layer.
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `data` - Site creation data
  ///
  /// # Returns
  /// * `Ok(Site)` - Successfully created site
  /// * `Err(SiteError)` - Creation failed due to validation, uniqueness, or database error
  pub async fn create_site(pool: &SqlitePool, data: CreateSiteData) -> Result<Site, SiteError> {
    // Input validation
    Self::validate_slug(&data.slug)?;

    // Clone slug for error handling before moving data
    let slug_clone = data.slug.clone();

    // Delegate to query
    CreateSiteQuery::run(pool, data).await.map_err(|err| {
      // Check if this is a unique constraint violation
      if let Some(sqlite_err) = err.as_database_error() {
        if let Some(code) = sqlite_err.code() {
          if code == "1555" || code == "2067" {
            // SQLite UNIQUE constraint violated
            return SiteError::SlugAlreadyExists(slug_clone);
          }
        }
      }
      SiteError::DatabaseError(err)
    })
  }

  /// Validate slug according to business rules
  fn validate_slug(slug: &str) -> Result<(), SiteError> {
    if slug.trim().is_empty() {
      return Err(SiteError::ValidationError(
        "Slug cannot be empty".to_string(),
      ));
    }

    if slug.len() < 3 {
      return Err(SiteError::ValidationError(
        "Slug must be at least 3 characters long".to_string(),
      ));
    }

    if slug.len() > 20 {
      return Err(SiteError::ValidationError(
        "Slug cannot be longer than 20 characters".to_string(),
      ));
    }

    // Must start with a letter
    if !slug.chars().next().unwrap().is_ascii_alphabetic() {
      return Err(SiteError::ValidationError(
        "Slug must start with a letter".to_string(),
      ));
    }

    // Must end with a letter or number
    if !slug.chars().last().unwrap().is_ascii_alphanumeric() {
      return Err(SiteError::ValidationError(
        "Slug must end with a letter or number".to_string(),
      ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !slug
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
      return Err(SiteError::ValidationError(
        "Slug can only contain letters, numbers, underscores, and hyphens".to_string(),
      ));
    }

    Ok(())
  }

  /// Get all sites from the database
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  ///
  /// # Returns
  /// * `Ok(Vec<Site>)` - Vector of all sites
  /// * `Err(SiteError)` - Database operation failed
  pub async fn get_all_sites(pool: &SqlitePool) -> Result<Vec<Site>, SiteError> {
    GetAllSitesQuery::run(pool)
      .await
      .map_err(SiteError::DatabaseError)
  }

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
  pub async fn update_site(
    pool: &SqlitePool,
    id: i64,
    data: UpdateSiteData,
  ) -> Result<Option<Site>, SiteError> {
    // Validate slug if provided
    if let Some(ref slug) = data.slug {
      Self::validate_slug(slug)?;

      // Check slug uniqueness (excluding current site)
      let existing_site = sqlx::query_as::<_, Site>(
        "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE slug = ? AND id != ?"
      )
      .bind(slug)
      .bind(id)
      .fetch_optional(pool)
      .await
      .map_err(SiteError::DatabaseError)?;

      if existing_site.is_some() {
        return Err(SiteError::SlugAlreadyExists(slug.clone()));
      }
    }

    // Delegate to query
    match UpdateSiteQuery::run(pool, UpdateSiteData { id, ..data }).await {
      Ok(Some(site)) => Ok(Some(site)),
      Ok(None) => Err(SiteError::SiteNotFound(id)),
      Err(err) => Err(SiteError::DatabaseError(err)),
    }
  }

  /// Delete a site by ID
  ///
  /// # Arguments
  /// * `pool` - Database connection pool
  /// * `site_id` - ID of site to delete
  ///
  /// # Returns
  /// * `Ok(Site)` - Successfully deleted site data
  /// * `Err(SiteError)` - Deletion failed due to site not found or database error
  pub async fn delete_site(pool: &SqlitePool, site_id: i64) -> Result<Site, SiteError> {
    DeleteSiteQuery::run(pool, site_id)
      .await
      .map_err(|e| match e {
        sqlx::Error::RowNotFound => SiteError::SiteNotFound(site_id),
        _ => SiteError::DatabaseError(e),
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_create_site_success() {
    let (pool, _temp_file) = create_test_database().await;

    let data = CreateSiteData {
      slug: "example".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: Some(r#"{"a":1}"#.to_string()),
    };

    let site = SiteService::create_site(&pool, data).await.unwrap();

    assert_eq!(site.slug, "example");
    assert_eq!(site.protocol, "https");
    assert_eq!(site.subdomain, Some("www".to_string()));
    assert_eq!(site.port, Some(443));
    assert_eq!(site.metadata_json, Some(r#"{"a":1}"#.to_string()));
    assert!(site.created_ts > 0);
    assert_eq!(site.created_ts, site.updated_ts);
  }

  #[tokio::test]
  async fn test_create_site_slug_already_exists() {
    let (pool, _temp_file) = create_test_database().await;

    let data1 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    SiteService::create_site(&pool, data1).await.unwrap();

    let data2 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    let result = SiteService::create_site(&pool, data2).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SlugAlreadyExists(slug) => assert_eq!(slug, "duplicate"),
      _ => panic!("Expected SlugAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_validate_slug_empty() {
    let result = SiteService::validate_slug("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_slug_whitespace_only() {
    let result = SiteService::validate_slug("   ");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_slug_too_short() {
    let result = SiteService::validate_slug("ab");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 3 characters"));
  }

  #[tokio::test]
  async fn test_validate_slug_too_long() {
    let result = SiteService::validate_slug(&"a".repeat(21));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 20 characters"));
  }

  #[tokio::test]
  async fn test_validate_slug_invalid_start() {
    let result = SiteService::validate_slug("123example");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("must start with a letter"));
  }

  #[tokio::test]
  async fn test_validate_slug_invalid_end() {
    let result = SiteService::validate_slug("example-");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("must end with a letter or number"));
  }

  #[tokio::test]
  async fn test_validate_slug_invalid_characters() {
    let result = SiteService::validate_slug("test@example");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("can only contain"));
  }

  #[tokio::test]
  async fn test_validate_slug_valid_cases() {
    assert!(SiteService::validate_slug("example").is_ok());
    assert!(SiteService::validate_slug("test-site-123").is_ok());
    assert!(SiteService::validate_slug("my_site").is_ok());
    assert!(SiteService::validate_slug("abc").is_ok());
    assert!(SiteService::validate_slug("z9x").is_ok());
  }

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
    assert_eq!(sites[0].slug, "test1");
    assert_eq!(sites[1].slug, "test2");
  }

  #[tokio::test]
  async fn test_update_site_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "update-test".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };
    let site = SiteService::create_site(&pool, create_data).await.unwrap();

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update the site
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("updated-slug".to_string()),
      subdomain: Some(None), // Set to null
      port: Some(Some(8080)),
      protocol: Some("http".to_string()),
      metadata_json: Some(None), // Set to null
    };

    let updated_site = SiteService::update_site(&pool, site.id, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_site.id, site.id);
    assert_eq!(updated_site.slug, "updated-slug");
    assert_eq!(updated_site.subdomain, None);
    assert_eq!(updated_site.port, Some(8080));
    assert_eq!(updated_site.protocol, "http");
    assert_eq!(updated_site.metadata_json, None);
    assert!(updated_site.updated_ts > site.updated_ts);
    assert_eq!(updated_site.created_ts, site.created_ts);
  }

  #[tokio::test]
  async fn test_update_site_slug_already_exists() {
    let (pool, _temp_file) = create_test_database().await;

    // Create two sites
    let data1 = CreateSiteData {
      slug: "site1".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let data2 = CreateSiteData {
      slug: "site2".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let site1 = SiteService::create_site(&pool, data1).await.unwrap();
    SiteService::create_site(&pool, data2).await.unwrap();

    // Try to update site1 with site2's slug
    let update_data = UpdateSiteData {
      id: site1.id,
      slug: Some("site2".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = SiteService::update_site(&pool, site1.id, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SlugAlreadyExists(slug) => assert_eq!(slug, "site2"),
      _ => panic!("Expected SlugAlreadyExists error"),
    }
  }

  #[tokio::test]
  async fn test_update_site_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let update_data = UpdateSiteData {
      id: 999,
      slug: Some("nonexistent".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = SiteService::update_site(&pool, 999, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SiteNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected SiteNotFound error"),
    }
  }

  #[tokio::test]
  async fn test_update_site_invalid_slug() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "valid-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = SiteService::create_site(&pool, create_data).await.unwrap();

    // Try to update with invalid slug
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("ab".to_string()), // Too short
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = SiteService::update_site(&pool, site.id, update_data).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Validation error"));
  }

  #[tokio::test]
  async fn test_delete_site_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "delete-test".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };
    let site = SiteService::create_site(&pool, create_data).await.unwrap();

    // Delete the site
    let deleted_site = SiteService::delete_site(&pool, site.id).await.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_site.id, site.id);
    assert_eq!(deleted_site.slug, site.slug);
    assert_eq!(deleted_site.subdomain, site.subdomain);
    assert_eq!(deleted_site.port, site.port);
    assert_eq!(deleted_site.protocol, site.protocol);
    assert_eq!(deleted_site.metadata_json, site.metadata_json);
    assert_eq!(deleted_site.created_ts, site.created_ts);
    assert_eq!(deleted_site.updated_ts, site.updated_ts);

    // Verify site is deleted from database
    let all_sites = SiteService::get_all_sites(&pool).await.unwrap();
    assert_eq!(all_sites.len(), 0);
  }

  #[tokio::test]
  async fn test_delete_site_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    let result = SiteService::delete_site(&pool, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SiteNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected SiteNotFound error"),
    }
  }
}
