use crate::models::DemoConfig;
use crate::server::{routes, static_files, websocket};
use crate::state::DemoAppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

/// Build the Axum application
pub fn build_app(config: DemoConfig) -> Router {
    let state = DemoAppState::new(config);

    // CORS configuration for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        // Health
        .route("/health", get(routes::health))
        // Configuration
        .route("/config", get(routes::get_config).put(routes::update_config))
        .route(
            "/config/issues",
            get(routes::get_issue_config).put(routes::update_issue_config),
        )
        // Traffic generation
        .route("/traffic/start", post(routes::start_traffic))
        .route("/traffic/stop", post(routes::stop_traffic))
        .route("/traffic/status", get(routes::traffic_status))
        // Statistics
        .route("/stats", get(routes::get_stats))
        .route("/stats/heatmap", get(routes::get_heatmap))
        .route("/stats/timeline", get(routes::get_timeline))
        .route("/stats/reset", post(routes::reset_metrics))
        // Events
        .route("/events", get(routes::list_events))
        .route("/events/:id", get(routes::get_event))
        // Policies
        .route("/policies", get(routes::list_policies))
        .route("/policies/:name", get(routes::get_policy));

    Router::new()
        .nest("/api", api_routes)
        .route("/ws", get(websocket::websocket_handler))
        .fallback(static_files::serve_static)
        .layer(cors)
        .with_state(state)
}

/// Run the server
pub async fn run_server(config: DemoConfig, addr: SocketAddr) -> anyhow::Result<()> {
    let app = build_app(config);

    tracing::info!("Starting CheckStream Demo server on {}", addr);
    tracing::info!("Open http://{} in your browser", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
