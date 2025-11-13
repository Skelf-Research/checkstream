//! HTTP routes and handlers

use axum::{
    routing::{get, post},
    Router,
};

use crate::config::ProxyConfig;

pub fn create_router(_config: ProxyConfig) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .route("/v1/chat/completions", post(chat_completions))
        .fallback(fallback)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn metrics() -> String {
    // TODO: Return Prometheus metrics
    "# HELP checkstream_requests_total Total requests\n".to_string()
}

async fn chat_completions() -> &'static str {
    // TODO: Implement chat completions proxy
    "Not implemented yet"
}

async fn fallback() -> &'static str {
    "Not found"
}
