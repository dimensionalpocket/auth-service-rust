use axum::{
  routing::get,
  Router,
};
use dp_auth_service::{
  handlers::{graphql::{graphql_get_handler, graphql_post_handler}, rest::{health_handler, root_handler}},
  graphql::schema::create_schema,
};
use tower_http::cors::CorsLayer;
use std::env;

#[tokio::main]
async fn main() {
  // Load environment variables from .env file
  dotenvy::dotenv().ok();
  
  let schema = create_schema();
  
  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route("/graphql", get(graphql_get_handler).post(graphql_post_handler))
    .layer(CorsLayer::permissive())
    .with_state(schema);

  // Get port from environment variable, default to 3000
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string());
  let bind_address = format!("0.0.0.0:{}", port);

  let listener = tokio::net::TcpListener::bind(&bind_address)
    .await
    .unwrap();
    
  println!("Server running on http://{}", bind_address);
  axum::serve(listener, app).await.unwrap();
}