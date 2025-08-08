use axum::{middleware, routing::get, Router};
use dp_auth_service::{
  database::Database,
  graphql::schema::create_schema,
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, not_found_handler, root_handler},
  },
  middleware::{
    logging::rest_logging_middleware,
    request_id::request_id_middleware,
    session::create_session_middleware,
  },
  utils::get_secret_from_env::get_secret_from_env,
  services::shutdown_service::ShutdownService,
};
use std::env;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
  // Initialize tracing
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  // Read session secret from environment
  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("Failed to read session secret from DP_AUTH_SECRET_KEY");

  // Get database URL from environment (temporary - will be moved in later phases)
  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  // Initialize database connection (migrations run separately via script)
  let _database = Database::new(&database_url)
    .await
    .expect("Failed to connect to database");

  // Create schema with database pool
  let schema = create_schema();

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route(
      "/graphql",
      get(graphql_get_handler)
        .post(graphql_post_handler)
        .layer(middleware::from_fn(create_session_middleware(session_secret))), // Session middleware only for GraphQL
    )
    .fallback(not_found_handler)
    .layer(
      ServiceBuilder::new()
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(rest_logging_middleware))
        .layer(CorsLayer::permissive()),
    )
    .with_state(schema);

  // Get port from environment variable, default to 3000
  let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
  let bind_address = format!("0.0.0.0:{port}");

  let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();

  println!("Server running on http://{bind_address}");

  // Create the server with graceful shutdown
  let server = axum::serve(listener, app).with_graceful_shutdown(async {
    let signal_name = ShutdownService::wait_for_shutdown_signal().await;
    ShutdownService::log_shutdown_start(signal_name);
    // The actual graceful shutdown is handled by axum's with_graceful_shutdown
    // This future completes when the signal is received, triggering axum's shutdown
  });

  // Run the server
  if let Err(e) = server.await {
    tracing::error!("Server error: {}", e);
  }
}
