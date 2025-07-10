use axum::{middleware, routing::get, Router};
use dp_auth_service::{
  graphql::schema::create_schema,
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, root_handler},
  },
  middleware::{logging::rest_logging_middleware, request_id::request_id_middleware},
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
  axum::serve(listener, app).await.unwrap();
}
