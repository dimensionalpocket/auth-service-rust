use crate::models::Site;
use sqlx::SqliteConnection;

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
  pub async fn run(
    conn: &mut SqliteConnection,
    data: UpdateSiteData,
  ) -> Result<Option<Site>, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Build the UPDATE query dynamically based on provided fields
    let mut set_clauses = vec!["updated_ts = ?".to_string()];

    // Add each field if provided
    if data.slug.is_some() {
      set_clauses.push("slug = ?".to_string());
    }

    if data.subdomain.is_some() {
      set_clauses.push("subdomain = ?".to_string());
    }

    if data.port.is_some() {
      set_clauses.push("port = ?".to_string());
    }

    if data.protocol.is_some() {
      set_clauses.push("protocol = ?".to_string());
    }

    if data.metadata_json.is_some() {
      set_clauses.push("metadata_json = ?".to_string());
    }

    // Construct the SQL
    let sql = format!("UPDATE sites SET {} WHERE id = ?", set_clauses.join(", "));

    // Execute the query with proper binding
    let mut query = sqlx::query(&sql).bind(now);

    // Bind each value in order
    if let Some(slug) = data.slug {
      query = query.bind(slug);
    }

    if let Some(subdomain) = data.subdomain {
      query = query.bind(subdomain);
    }

    if let Some(port) = data.port {
      query = query.bind(port);
    }

    if let Some(protocol) = data.protocol {
      query = query.bind(protocol);
    }

    if let Some(metadata_json) = data.metadata_json {
      query = query.bind(metadata_json);
    }

    query = query.bind(data.id);

    let result = query.execute(&mut *conn).await?;

    if result.rows_affected() == 0 {
      return Ok(None);
    }

    // Return updated site
    sqlx::query_as::<_, Site>(
            "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE id = ?"
        )
        .bind(data.id)
        .fetch_one(&mut *conn)
        .await
        .map(Some)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_update_site_success() {
    let (pool, _tmp) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Create a site first
    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"description": "test"}"#.to_string()),
    };
    let site = CreateSiteQuery::run(&mut conn, create_data).await.unwrap();

    // Update the site
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("updated-site".to_string()),
      subdomain: Some(None), // Set to null
      port: Some(Some(8080)),
      protocol: Some("http".to_string()),
      metadata_json: Some(None), // Set to null
    };

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let updated_site = UpdateSiteQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_site.id, site.id);
    assert_eq!(updated_site.slug, "updated-site");
    assert_eq!(updated_site.subdomain, None);
    assert_eq!(updated_site.port, Some(8080));
    assert_eq!(updated_site.protocol, "http");
    assert_eq!(updated_site.metadata_json, None);
    assert!(updated_site.updated_ts > site.updated_ts);
    assert_eq!(updated_site.created_ts, site.created_ts);
  }

  #[tokio::test]
  async fn test_update_site_partial_update() {
    let (pool, _tmp) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Create a site first
    let create_data = CreateSiteData {
      slug: "partial-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"description": "test"}"#.to_string()),
    };
    let site = CreateSiteQuery::run(&mut conn, create_data).await.unwrap();

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update only the slug
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("partial-updated".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let updated_site = UpdateSiteQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_site.id, site.id);
    assert_eq!(updated_site.slug, "partial-updated");
    assert_eq!(updated_site.subdomain, Some("www".to_string())); // unchanged
    assert_eq!(updated_site.port, Some(443)); // unchanged
    assert_eq!(updated_site.protocol, "https"); // unchanged
    assert_eq!(
      updated_site.metadata_json,
      Some(r#"{"description": "test"}"#.to_string())
    ); // unchanged
    assert!(updated_site.updated_ts > site.updated_ts);
  }

  #[tokio::test]
  async fn test_update_site_not_found() {
    let (pool, _tmp) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    let update_data = UpdateSiteData {
      id: 999,
      slug: Some("nonexistent".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = UpdateSiteQuery::run(&mut conn, update_data).await.unwrap();
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_update_site_no_changes() {
    let (pool, _tmp) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Create a site first
    let create_data = CreateSiteData {
      slug: "no-changes-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"description": "test"}"#.to_string()),
    };
    let site = CreateSiteQuery::run(&mut conn, create_data).await.unwrap();

    // Add a delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update with no actual changes (empty payload)
    let update_data = UpdateSiteData {
      id: site.id,
      slug: None,
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let updated_site = UpdateSiteQuery::run(&mut conn, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_site.id, site.id);
    assert_eq!(updated_site.slug, site.slug);
    assert_eq!(updated_site.subdomain, site.subdomain);
    assert_eq!(updated_site.port, site.port);
    assert_eq!(updated_site.protocol, site.protocol);
    assert_eq!(updated_site.metadata_json, site.metadata_json);
    assert!(updated_site.updated_ts > site.updated_ts); // updated_ts should still change
  }
}
