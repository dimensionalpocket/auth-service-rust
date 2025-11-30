use crate::middleware::session::SessionContext;
use crate::queries::sites::{CreateSiteData, GetSiteByIdQuery, UpdateSiteData};
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use sqlx::SqlitePool;

pub struct SiteOrchestrator;

impl SiteOrchestrator {
  pub async fn create_site_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    create_data: CreateSiteData,
  ) -> Result<crate::models::Site, SiteError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::ValidationError("User not found".to_string()))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_create_site")
      .await
      .map_err(SiteError::DatabaseError)?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Create site
    SiteService::create_site(pool, create_data).await
  }

  pub async fn remove_site_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
  ) -> Result<crate::models::Site, SiteError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::ValidationError("User not found".to_string()))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_delete_site")
      .await
      .map_err(SiteError::DatabaseError)?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Delete site
    SiteService::delete_site(pool, site_id).await
  }

  pub async fn update_site_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
    update_data: UpdateSiteData,
  ) -> Result<Option<crate::models::Site>, SiteError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::ValidationError("User not found".to_string()))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_update_site")
      .await
      .map_err(SiteError::DatabaseError)?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Update site
    SiteService::update_site(pool, site_id, update_data).await
  }

  pub async fn get_site_details_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
  ) -> Result<crate::models::Site, SiteError> {
    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::ValidationError("User not found".to_string()))?;

    let allowed = UserRoleService::check_user_permission(pool, &user, "can_view_site_details")
      .await
      .map_err(SiteError::DatabaseError)?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get site details
    GetSiteByIdQuery::run(pool, site_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::SiteNotFound(site_id))
  }
}
