//! Shebe HTTP server entry point
//!
//! Starts the REST API server for the Shebe RAG service.

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use shebe::core::config::Config;
use shebe::core::services::Services;
use shebe::core::xdg::{migrate_legacy_paths, XdgDirs};
use shebe::http::{self, middleware as http_middleware};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shebe=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Shebe RAG service");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Initialize XDG directories
    let xdg = XdgDirs::new();
    xdg.log_paths();

    // Ensure XDG directories exist
    xdg.ensure_dirs_exist()?;

    // Run migration from legacy paths (if needed)
    if let Err(e) = migrate_legacy_paths(&xdg) {
        tracing::warn!("Migration warning: {}", e);
        tracing::info!("Continuing with current paths...");
    }

    // Load configuration
    let config = Config::load()?;

    // Log configuration details
    config.log_config();

    // Create shared services
    let services = Arc::new(Services::new(config.clone()));

    // Build the API router
    let app = Router::new()
        // Health check endpoint
        .route("/health", get(http::health_handler))
        // API v1 endpoints
        .route("/api/v1/index", post(http::index_handler))
        .route("/api/v1/search", post(http::search_handler))
        .route("/api/v1/sessions", get(http::list_sessions_handler))
        .route(
            "/api/v1/sessions/:session_id",
            delete(http::delete_session_handler),
        )
        // Add middleware
        .layer(middleware::from_fn(http_middleware::log_request))
        .layer(CorsLayer::permissive())
        // Add shared state
        .with_state(services);

    // Bind to address and start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {}", addr);
    tracing::info!("Service ready - Health check at http://{}/health", addr);

    // Serve the application
    axum::serve(listener, app).await?;

    Ok(())
}
