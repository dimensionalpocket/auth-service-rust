pub mod config;
pub mod database;
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod queries;
pub mod services;
pub mod utils;

pub use config::{ServerConfig, ConfigError};

use axum::{middleware::from_fn, routing::get, Router};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn start_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database connection
    let _database = database::Database::new(&config.database_url).await?;

    // Create schema
    let schema = graphql::schema::create_schema();

    // Build router with config
    let app = Router::new()
        .route("/", get(handlers::rest::root_handler))
        .route("/health", get(handlers::rest::health_handler))
        .route(
            "/graphql",
            get({
                let development_mode = config.development_mode;
                move || handlers::graphql::graphql_get_handler(development_mode)
            })
            .post({
                let cookie_domain = config.cookie_domain.clone();
                let insecure_cookie = config.insecure_cookie;
                let development_mode = config.development_mode;
                let session_secret = config.session_secret.clone();
                move |state, request| {
                    handlers::graphql::graphql_post_handler(
                        state, 
                        request, 
                        cookie_domain.clone(), 
                        insecure_cookie, 
                        development_mode,
                        session_secret.clone()
                    )
                }
            })
            .layer(from_fn(
                middleware::session::create_session_middleware(config.session_secret.clone())
            )),
        )
        .fallback(handlers::rest::not_found_handler)
        .layer(
            ServiceBuilder::new()
                .layer(from_fn(middleware::request_id::request_id_middleware))
                .layer(from_fn(middleware::logging::rest_logging_middleware))
                .layer(CorsLayer::permissive()),
        )
        .with_state(schema);

    // Bind to configured port
    let bind_address = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&bind_address).await?;

    println!("Server running on http://{bind_address}");

    // Create server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let signal_name = services::shutdown_service::ShutdownService::wait_for_shutdown_signal().await;
        services::shutdown_service::ShutdownService::log_shutdown_start(signal_name);
    });

    server.await?;
    Ok(())
}
