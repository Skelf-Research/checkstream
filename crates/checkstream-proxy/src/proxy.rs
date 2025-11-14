//! Core proxy logic

use anyhow::Result;
use checkstream_classifiers::{
    ClassifierRegistry, ClassifierPipeline, StreamingPipeline,
};
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::ProxyConfig;

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    /// Loaded configuration
    pub config: Arc<ProxyConfig>,

    /// Classifier registry with all loaded classifiers
    pub registry: Arc<ClassifierRegistry>,

    /// Pre-built pipelines for each phase
    pub pipelines: Arc<Pipelines>,

    /// HTTP client for backend requests
    pub http_client: reqwest::Client,
}

/// Pre-built pipelines for the three phases
pub struct Pipelines {
    /// Phase 1: Ingress (pre-generation validation)
    pub ingress: ClassifierPipeline,

    /// Phase 2: Midstream (streaming checks)
    pub midstream: ClassifierPipeline,

    /// Phase 3: Egress (post-generation compliance)
    pub egress: ClassifierPipeline,
}

impl AppState {
    /// Initialize application state from configuration
    pub async fn new(config: ProxyConfig) -> Result<Self> {
        info!("Initializing application state");

        // Load classifier registry
        info!("Loading classifiers from: {}", config.classifiers_config);
        let registry = ClassifierRegistry::from_file(&config.classifiers_config).await?;
        info!("Loaded {} classifiers", registry.count());

        // Build pipelines for each phase
        let pipelines = Self::build_pipelines(&config, &registry)?;

        // Create HTTP client for backend requests
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()?;

        Ok(Self {
            config: Arc::new(config),
            registry: Arc::new(registry),
            pipelines: Arc::new(pipelines),
            http_client,
        })
    }

    /// Build pipelines from configuration
    fn build_pipelines(config: &ProxyConfig, registry: &ClassifierRegistry) -> Result<Pipelines> {
        info!("Building pipelines");

        // Build Phase 1: Ingress pipeline
        info!("Building ingress pipeline: {}", config.pipelines.ingress_pipeline);
        let ingress = registry.build_pipeline(&config.pipelines.ingress_pipeline)?;

        // Build Phase 2: Midstream pipeline
        info!("Building midstream pipeline: {}", config.pipelines.midstream_pipeline);
        let midstream = registry.build_pipeline(&config.pipelines.midstream_pipeline)?;

        // Build Phase 3: Egress pipeline
        info!("Building egress pipeline: {}", config.pipelines.egress_pipeline);
        let egress = registry.build_pipeline(&config.pipelines.egress_pipeline)?;

        Ok(Pipelines {
            ingress,
            midstream,
            egress,
        })
    }
}

/// Phase 1: Ingress - Validate request before sending to LLM
pub async fn execute_ingress(
    state: &AppState,
    prompt: &str,
) -> Result<IngressResult> {
    debug!("Phase 1: Executing ingress checks on prompt");

    let start = std::time::Instant::now();
    let result = state.pipelines.ingress.execute(prompt).await?;
    let latency = start.elapsed();

    // Record metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "ingress")
        .record(latency.as_micros() as f64);

    // Check against safety threshold
    let should_block = result.final_decision
        .as_ref()
        .map_or(false, |d| d.score > state.config.pipelines.safety_threshold);

    if should_block {
        metrics::counter!("checkstream_decisions_total", "phase" => "ingress", "action" => "block")
            .increment(1);

        info!(
            "Phase 1: BLOCKED - Score: {:.3}, Latency: {:?}",
            result.final_decision.as_ref().unwrap().score,
            latency
        );
    } else {
        metrics::counter!("checkstream_decisions_total", "phase" => "ingress", "action" => "pass")
            .increment(1);

        debug!("Phase 1: PASSED - Latency: {:?}", latency);
    }

    Ok(IngressResult {
        blocked: should_block,
        result,
        latency,
    })
}

/// Phase 2: Midstream - Check streaming chunks as they arrive
pub async fn execute_midstream_chunk(
    streaming: &mut StreamingPipeline,
    chunk: String,
    threshold: f32,
) -> Result<MidstreamResult> {
    debug!("Phase 2: Checking chunk: {:?}", chunk);

    let start = std::time::Instant::now();
    let result = streaming.execute_chunk(chunk).await?;
    let latency = start.elapsed();

    // Record metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "midstream")
        .record(latency.as_micros() as f64);

    // Check if this chunk should be redacted
    let should_redact = result.final_decision
        .as_ref()
        .map_or(false, |d| d.score > threshold);

    if should_redact {
        metrics::counter!("checkstream_decisions_total", "phase" => "midstream", "action" => "redact")
            .increment(1);

        debug!(
            "Phase 2: REDACTING chunk - Score: {:.3}",
            result.final_decision.as_ref().unwrap().score
        );
    }

    Ok(MidstreamResult {
        redacted: should_redact,
        result,
        latency,
    })
}

/// Phase 3: Egress - Final compliance check on complete response
pub async fn execute_egress(
    state: &AppState,
    full_text: &str,
) -> Result<EgressResult> {
    info!("Phase 3: Executing egress compliance check");

    let start = std::time::Instant::now();
    let result = state.pipelines.egress.execute(full_text).await?;
    let latency = start.elapsed();

    // Record metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "egress")
        .record(latency.as_micros() as f64);

    metrics::counter!("checkstream_decisions_total", "phase" => "egress", "action" => "complete")
        .increment(1);

    info!("Phase 3: COMPLETE - Latency: {:?}", latency);

    Ok(EgressResult {
        result,
        latency,
    })
}

/// Result from Phase 1: Ingress
pub struct IngressResult {
    pub blocked: bool,
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
}

/// Result from Phase 2: Midstream chunk check
pub struct MidstreamResult {
    pub redacted: bool,
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
}

/// Result from Phase 3: Egress
pub struct EgressResult {
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
}
