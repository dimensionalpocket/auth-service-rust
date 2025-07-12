use axum::{middleware, routing::get, Router};
use dp_auth_service::{
  database::Database,
  graphql::schema::create_schema,
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, root_handler},
  },
  middleware::{logging::rest_logging_middleware, request_id::request_id_middleware},
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

  // Initialize database connection (migrations run separately via script)
  let _database = Database::new()
    .await
    .expect("Failed to connect to database");

  // Create schema with database pool
  let schema = create_schema();

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route(
      "/graphql",
      get(graphql_get_handler).post(graphql_post_handler),
    )
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
