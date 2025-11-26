use crate::graphql::schema::AppSchema;
use axum::{middleware::from_fn, routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

#[derive(Debug)]
pub struct DpsAuthApi {
  pub(crate) config: Arc<DpsAuthApiConfig>,
}

#[derive(Debug, Clone)]
pub struct DpsAuthApiConfig {
  pub port: u16,
  pub sqlite_main_file_path: String,
  pub session_secret: Vec<u8>,
  pub cookie_domain: String,
  pub api_path: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
  pub sqlite_main_pool_size: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DpsAuthApiError {
  InvalidSecretLength { actual: usize, expected: usize },
  MissingRequiredConfig { field: String },
  DatabaseError(String),
  NetworkError(String),
  ConfigurationError(String),
}

impl std::fmt::Display for DpsAuthApiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DpsAuthApiError::InvalidSecretLength { actual, expected } => {
        write!(
          f,
          "Invalid secret length: got {actual} bytes, expected {expected}"
        )
      }
      DpsAuthApiError::MissingRequiredConfig { field } => {
        write!(f, "Missing required configuration: {field}")
      }
      DpsAuthApiError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      DpsAuthApiError::NetworkError(msg) => write!(f, "Network error: {msg}"),
      DpsAuthApiError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
    }
  }
}

impl std::error::Error for DpsAuthApiError {}

impl DpsAuthApi {
  /// Create a new DpsAuthApi instance from DpsConfig
  ///
  /// This validates the configuration and returns an error if required fields are missing
  /// or invalid. The server will not start if configuration is invalid.
  ///
  /// # Errors
  ///
  /// Returns `DpsAuthApiError::MissingRequiredConfig` if session_secret is not set
  /// Returns `DpsAuthApiError::InvalidSecretLength` if session_secret is not exactly 32 bytes
  ///
  /// # Example
  ///
  /// ```rust
  /// use dps_config::DpsConfig;
  /// use dps_auth_api::DpsAuthApi;
  ///
  /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut config = DpsConfig::new();
  /// config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  /// let server = DpsAuthApi::new(config)?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn new(dps_config: dps_config::DpsConfig) -> Result<Self, DpsAuthApiError> {
    // Extract session secret (required)
    let session_secret = dps_config.get_auth_api_session_secret_bytes().ok_or(
      DpsAuthApiError::MissingRequiredConfig {
        field: "auth_api_session_secret".to_string(),
      },
    )?;

    // Validate session secret length
    if session_secret.len() != 32 {
      return Err(DpsAuthApiError::InvalidSecretLength {
        actual: session_secret.len(),
        expected: 32,
      });
    }

    // Build resolved config with defaults from DpsConfig
    let config = DpsAuthApiConfig {
      port: dps_config.get_auth_api_port().unwrap_or(3000),
      sqlite_main_file_path: dps_config.get_auth_api_sqlite_main_file_path(),
      session_secret,
      cookie_domain: format!(".{}", dps_config.get_domain()),
      api_path: format!("/{}", dps_config.get_api_path()),
      insecure_cookie: dps_config.get_auth_api_insecure_cookie(),
      development_mode: dps_config.get_development_mode(),
      sqlite_main_pool_size: dps_config.get_auth_api_sqlite_main_pool_size(),
    };

    Ok(DpsAuthApi {
      config: Arc::new(config),
    })
  }

  pub async fn start(self) -> Result<(), DpsAuthApiError> {
    // Phase 3B - Schema and router setup (now includes database initialization)
    let app = self.create_app().await?;

    // Phase 3C - Network binding and shutdown
    let listener = self.bind_listener().await?;
    println!("Server running on http://0.0.0.0:{}", self.config.port);

    let server = axum::serve(listener, app).with_graceful_shutdown(self.create_shutdown_handler());
    server
      .await
      .map_err(|e| DpsAuthApiError::NetworkError(e.to_string()))?;

    Ok(())
  }

  /// Creates a configured Axum router with GraphQL schema and all middleware.
  /// This method initializes the database and includes it in the GraphQL schema.
  /// This method is useful for testing and for getting a router without starting the server.
  pub async fn create_app(&self) -> Result<Router, DpsAuthApiError> {
    let database = self.initialize_database().await?;
    let schema = crate::graphql::schema::build_schema()
      .data(database.pool)
      .finish();
    Ok(self.build_router(schema))
  }

  /// Initialize database connection only (no migrations or seeds).
  /// This method is public for test usage.
  pub async fn initialize_database(&self) -> Result<crate::database::Database, DpsAuthApiError> {
    crate::database::Database::new_with_pool_size(
      &self.config.sqlite_main_file_path,
      Some(self.config.sqlite_main_pool_size as u32),
    )
    .await
    .map_err(|e| DpsAuthApiError::DatabaseError(e.to_string()))
  }

  /// Run database migrations only.
  pub async fn migrate_database(&self) -> Result<(), DpsAuthApiError> {
    let database = self.initialize_database().await?;
    database
      .migrate()
      .await
      .map_err(|e| DpsAuthApiError::DatabaseError(e.to_string()))
  }

  /// Run database seeds only.
  pub async fn seed_database(&self) -> Result<(), DpsAuthApiError> {
    let database = self.initialize_database().await?;
    database
      .seed()
      .await
      .map_err(|e| DpsAuthApiError::DatabaseError(e.to_string()))
  }

  async fn bind_listener(&self) -> Result<TcpListener, DpsAuthApiError> {
    let bind_address = format!("0.0.0.0:{}", self.config.port);
    TcpListener::bind(&bind_address)
      .await
      .map_err(|e| DpsAuthApiError::NetworkError(e.to_string()))
  }

  fn create_shutdown_handler(&self) -> impl std::future::Future<Output = ()> {
    async {
      let signal_name =
        crate::services::shutdown_service::ShutdownService::wait_for_shutdown_signal().await;
      crate::services::shutdown_service::ShutdownService::log_shutdown_start(signal_name);
    }
  }

  pub fn build_router(&self, schema: AppSchema) -> Router {
    Router::new()
      .route("/", get(crate::handlers::rest::root_handler))
      .route("/health", get(crate::handlers::rest::health_handler))
      .route(
        &format!("{}/graphql", self.config.api_path),
        get({
          let development_mode = self.config.development_mode;
          move || crate::handlers::graphql::graphql_get_handler(development_mode)
        })
        .post({
          let config = self.config.clone();
          move |state, request| {
            crate::handlers::graphql::graphql_post_handler(state, request, config.clone())
          }
        })
        .layer(from_fn(
          crate::middleware::session::create_session_middleware(self.config.session_secret.clone()),
        )),
      )
      .fallback(crate::handlers::rest::not_found_handler)
      .layer(
        ServiceBuilder::new()
          .layer(from_fn(
            crate::middleware::request_id::request_id_middleware,
          ))
          .layer(from_fn(crate::middleware::logging::rest_logging_middleware))
          .layer(CorsLayer::very_permissive()),
      )
      .with_state(schema)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::{
    body::Body,
    http::{Request, StatusCode},
  };
  use tempfile::NamedTempFile;
  use tower::ServiceExt;

  // Helper function to create a DpsAuthApi instance for tests
  fn create_test_server(db_path: &str) -> DpsAuthApi {
    let mut config = dps_config::DpsConfig::new();
    config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
    config.set_auth_api_sqlite_main_file_path(db_path);
    DpsAuthApi::new(config).unwrap()
  }

  // Helper function to create a DpsAuthApi instance with custom port
  fn create_test_server_with_port(db_path: &str, port: u16) -> DpsAuthApi {
    let mut config = dps_config::DpsConfig::new();
    config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
    config.set_auth_api_sqlite_main_file_path(db_path);
    config.set_auth_api_port(Some(port));
    DpsAuthApi::new(config).unwrap()
  }

  #[tokio::test]
  async fn test_initialize_database_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    let result = server.initialize_database().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_initialize_database_invalid_path() {
    let server = create_test_server("/invalid/path/that/does/not/exist/test.db");

    let result = server.initialize_database().await;
    assert!(matches!(result, Err(DpsAuthApiError::DatabaseError(_))));
  }

  #[tokio::test]
  async fn test_initialize_database_uses_configured_path() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    // Should use the configured path, not the default
    assert_eq!(server.config.sqlite_main_file_path, db_path);

    let result = server.initialize_database().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_start_method_setup_phases() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server_with_port(db_path, 0); // Use available port

    // Test individual phases without calling start() which would block
    // Phase 3A - Database initialization
    let database_result = server.initialize_database().await;
    assert!(database_result.is_ok());

    // Phase 3B - Router building
    let schema = crate::graphql::schema::build_schema().finish();
    let _router = server.build_router(schema);

    // Phase 3C - Network binding
    let listener_result = server.bind_listener().await;
    assert!(listener_result.is_ok());
  }

  #[tokio::test]
  async fn test_start_method_database_error_propagation() {
    let server = create_test_server("/invalid/path/test.db");

    // Test that database error is propagated from initialize_database()
    // Don't call start() as it would hang if database init somehow succeeded
    let result = server.initialize_database().await;
    assert!(matches!(result, Err(DpsAuthApiError::DatabaseError(_))));
  }

  #[tokio::test]
  async fn test_create_app_method() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    let app = server.create_app().await.unwrap();

    // Router should be created successfully
    assert_eq!(
      std::any::type_name_of_val(&app),
      std::any::type_name::<Router>()
    );
  }

  #[tokio::test]
  async fn test_build_router_basic_structure() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let router = server.build_router(schema);

    // Router should be created successfully
    assert_eq!(
      std::any::type_name_of_val(&router),
      std::any::type_name::<Router>()
    );
  }

  #[tokio::test]
  async fn test_build_router_root_endpoint() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    assert_eq!(&body[..], b"OK");
  }

  #[tokio::test]
  async fn test_build_router_health_endpoint() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(
        Request::builder()
          .uri("/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    assert_eq!(&body[..], b"OK");
  }

  #[tokio::test]
  async fn test_build_router_graphql_endpoint() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let query = r#"{"query": "{ getServerTimestamp }"}"#;

    let response = app
      .oneshot(
        Request::builder()
          .method("POST")
          .uri(format!("{}/graphql", server.config.api_path))
          .header("content-type", "application/json")
          .body(Body::from(query))
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain timestamp data
    assert!(body_str.contains("getServerTimestamp"));
    assert!(body_str.contains("data"));
  }

  #[tokio::test]
  async fn test_build_router_404_handler() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(
        Request::builder()
          .uri("/nonexistent")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "NOT FOUND");
  }

  #[tokio::test]
  async fn test_build_router_404_handler_with_query_string() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(
        Request::builder()
          .uri("/nonexistent?param=value")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "NOT FOUND");
  }

  #[tokio::test]
  async fn test_build_router_404_handler_with_post_method() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/nonexistent")
          .header("content-type", "application/json")
          .body(Body::from(r#"{"test": "data"}"#))
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "NOT FOUND");
  }

  // Phase 3C Tests - Network binding and shutdown handling

  #[tokio::test]
  async fn test_bind_listener_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server_with_port(db_path, 0); // Use port 0 to get any available port

    let result = server.bind_listener().await;
    assert!(result.is_ok());

    let listener = result.unwrap();
    let addr = listener.local_addr().unwrap();
    assert!(addr.port() > 0); // Should get an actual port
  }

  #[tokio::test]
  async fn test_bind_listener_uses_configured_port() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server_with_port(db_path, 0); // Use port 0 for available port

    let result = server.bind_listener().await;
    assert!(result.is_ok());

    // Verify the binding process works (actual port will be assigned by OS)
    let listener = result.unwrap();
    assert!(listener.local_addr().is_ok());
  }

  // Phase 3E Tests - Database operation separation

  #[tokio::test]
  async fn test_migrate_database_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    let result = server.migrate_database().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_migrate_database_invalid_path() {
    let server = create_test_server("/invalid/path/that/does/not/exist/test.db");

    let result = server.migrate_database().await;
    assert!(matches!(result, Err(DpsAuthApiError::DatabaseError(_))));
  }

  #[tokio::test]
  async fn test_seed_database_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    // First run migrations to set up tables
    let migrate_result = server.migrate_database().await;
    assert!(migrate_result.is_ok());

    // Then run seeds
    let seed_result = server.seed_database().await;
    assert!(seed_result.is_ok());
  }

  #[tokio::test]
  async fn test_seed_database_invalid_path() {
    let server = create_test_server("/invalid/path/that/does/not/exist/test.db");

    let result = server.seed_database().await;
    assert!(matches!(result, Err(DpsAuthApiError::DatabaseError(_))));
  }

  #[tokio::test]
  async fn test_database_operations_separation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    // Test that each operation can be called independently

    // 1. Initialize database (connection only)
    let database = server.initialize_database().await;
    assert!(database.is_ok());

    // 2. Run migrations separately
    let migrate_result = server.migrate_database().await;
    assert!(migrate_result.is_ok());

    // 3. Run seeds separately
    let seed_result = server.seed_database().await;
    assert!(seed_result.is_ok());
  }

  #[tokio::test]
  async fn test_initialize_database_is_public() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let server = create_test_server(db_path);

    // This test verifies that initialize_database is public and can be called from tests
    let result = server.initialize_database().await;
    assert!(result.is_ok());

    // Verify we get a Database instance
    let database = result.unwrap();
    assert_eq!(
      std::any::type_name_of_val(&database),
      "dps_auth_api::database::Database"
    );
  }

  #[tokio::test]
  async fn test_cors_headers_with_credentials() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
      .oneshot(
        Request::builder()
          .method("OPTIONS")
          .uri(format!("{}/graphql", server.config.api_path))
          .header("origin", "https://example.com")
          .header("access-control-request-method", "POST")
          .header(
            "access-control-request-headers",
            "content-type, authorization",
          )
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check that origin is reflected back
    assert_eq!(
      response
        .headers()
        .get("access-control-allow-origin")
        .unwrap(),
      "https://example.com"
    );

    // Check that credentials are allowed
    assert_eq!(
      response
        .headers()
        .get("access-control-allow-credentials")
        .unwrap(),
      "true"
    );
  }
}
