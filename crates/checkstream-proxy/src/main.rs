//! CheckStream Proxy
//!
//! High-performance HTTP/SSE proxy for streaming LLM guardrails.
//!
//! This proxy sits between clients and LLM APIs (OpenAI, Anthropic, etc.),
//! applying real-time safety and compliance checks with sub-10ms latency.

use anyhow::Result;
use clap::Parser;
use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tracing::{info, warn};

mod config;
mod proxy;
mod routes;
mod security;
mod tenant;

use config::MultiTenantConfig;
pub use tenant::{TenantResolver, TenantRuntime};

/// Global shutdown flag
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Check if shutdown has been requested
pub fn is_shutting_down() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

#[derive(Parser, Debug)]
#[command(name = "checkstream-proxy")]
#[command(about = "CheckStream streaming guardrail proxy", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Backend LLM API URL
    #[arg(short, long)]
    backend: Option<String>,

    /// Policy file or policy pack name
    #[arg(short, long)]
    policy: Option<String>,

    /// Listen address
    #[arg(short = 'l', long, default_value = "0.0.0.0")]
    listen: String,

    /// Listen port
    #[arg(short = 'P', long, default_value = "8080")]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    init_tracing(cli.verbose);

    info!("Starting CheckStream Proxy");
    info!("Built with Rust for maximum performance");

    // Load configuration
    let config = MultiTenantConfig::load(&cli.config, &cli)?;
    info!("Configuration loaded successfully");
    info!("Backend: {}", config.default.backend_url);
    info!("Policy: {}", config.default.policy_path);
    info!("Classifiers: {}", config.default.classifiers_config);
    info!("Configured tenants: {}", config.tenants.len());

    // Initialize metrics
    let metrics_handle = init_metrics()?;

    // Initialize application state (load classifiers and build pipelines)
    info!("Initializing application state...");
    let state = proxy::AppState::new_multi_tenant(config, metrics_handle).await?;
    info!("Application state initialized successfully");

    // Create proxy server
    let addr: SocketAddr = format!("{}:{}", cli.listen, cli.port).parse()?;
    info!("Starting proxy server on {}", addr);

    // Build and run the server with graceful shutdown
    let app = routes::create_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Proxy listening on http://{}", addr);

    // Graceful shutdown handler
    let shutdown = async {
        shutdown_signal().await;
        SHUTDOWN.store(true, Ordering::SeqCst);
        warn!("Shutdown signal received, stopping server...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Listen for shutdown signals (SIGTERM, SIGINT)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Initialize tracing/logging
fn init_tracing(verbose: bool) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = if verbose {
        EnvFilter::new("checkstream=debug,tower_http=debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("checkstream=info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize metrics exporter and return handle for rendering
fn init_metrics() -> Result<PrometheusHandle> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("Failed to install metrics: {}", e))?;

    // Initialize baseline metrics
    metrics::describe_counter!(
        "checkstream_requests_total",
        "Total number of requests processed"
    );
    metrics::describe_counter!(
        "checkstream_decisions_total",
        "Total number of pipeline decisions by phase and action"
    );
    metrics::describe_histogram!(
        "checkstream_pipeline_latency_us",
        metrics::Unit::Microseconds,
        "Pipeline execution latency in microseconds by phase"
    );
    metrics::describe_counter!("checkstream_errors_total", "Total number of errors by type");

    info!("Metrics exporter initialized");
    Ok(handle)
}
