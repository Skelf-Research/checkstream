//! Proxy configuration

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Backend LLM API URL
    pub backend_url: String,

    /// Policy file path or policy pack name
    pub policy_path: String,

    /// Classifiers configuration file path
    #[serde(default = "default_classifiers_config")]
    pub classifiers_config: String,

    /// Token buffer holdback size
    #[serde(default = "default_holdback")]
    pub token_holdback: usize,

    /// Maximum buffer capacity
    #[serde(default = "default_buffer_capacity")]
    pub max_buffer_capacity: usize,

    /// Pipeline configuration
    #[serde(default)]
    pub pipelines: PipelineSettings,

    /// Telemetry configuration
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

/// Pipeline execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSettings {
    /// Pipeline to use for Phase 1 (Ingress)
    #[serde(default = "default_ingress_pipeline")]
    pub ingress_pipeline: String,

    /// Pipeline to use for Phase 2 (Midstream)
    #[serde(default = "default_midstream_pipeline")]
    pub midstream_pipeline: String,

    /// Pipeline to use for Phase 3 (Egress)
    #[serde(default = "default_egress_pipeline")]
    pub egress_pipeline: String,

    /// Safety threshold for blocking (0.0-1.0)
    #[serde(default = "default_safety_threshold")]
    pub safety_threshold: f32,

    /// Threshold for per-chunk redaction in streaming
    #[serde(default = "default_chunk_threshold")]
    pub chunk_threshold: f32,

    /// Pipeline timeout in milliseconds
    #[serde(default = "default_pipeline_timeout")]
    pub timeout_ms: u64,

    /// Streaming context configuration
    #[serde(default)]
    pub streaming: StreamingSettings,
}

/// Streaming classification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSettings {
    /// Number of chunks to include as context (0 = entire buffer)
    #[serde(default = "default_context_chunks")]
    pub context_chunks: usize,

    /// Maximum buffer size
    #[serde(default = "default_stream_buffer_size")]
    pub max_buffer_size: usize,
}

impl Default for PipelineSettings {
    fn default() -> Self {
        Self {
            ingress_pipeline: default_ingress_pipeline(),
            midstream_pipeline: default_midstream_pipeline(),
            egress_pipeline: default_egress_pipeline(),
            safety_threshold: default_safety_threshold(),
            chunk_threshold: default_chunk_threshold(),
            timeout_ms: default_pipeline_timeout(),
            streaming: StreamingSettings::default(),
        }
    }
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self {
            context_chunks: default_context_chunks(),
            max_buffer_size: default_stream_buffer_size(),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from file and CLI overrides
    pub fn load(config_path: &str, cli: &crate::Cli) -> anyhow::Result<Self> {
        // Try to load from file, or use defaults
        let mut config = if Path::new(config_path).exists() {
            let content = std::fs::read_to_string(config_path)?;
            serde_yaml::from_str(&content)?
        } else {
            Self::default()
        };

        // Apply CLI overrides
        if let Some(backend) = &cli.backend {
            config.backend_url = backend.clone();
        }

        if let Some(policy) = &cli.policy {
            config.policy_path = policy.clone();
        }

        Ok(config)
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            backend_url: "https://api.openai.com/v1".to_string(),
            policy_path: "./policies/default.yaml".to_string(),
            classifiers_config: default_classifiers_config(),
            token_holdback: default_holdback(),
            max_buffer_capacity: default_buffer_capacity(),
            pipelines: PipelineSettings::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable telemetry
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Aggregation mode
    #[serde(default)]
    pub mode: TelemetryMode,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: TelemetryMode::Aggregate,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryMode {
    /// Aggregate metrics only
    #[default]
    Aggregate,
    /// Full event logging
    Full,
}

fn default_classifiers_config() -> String {
    "./classifiers.yaml".to_string()
}

fn default_holdback() -> usize {
    10
}

fn default_buffer_capacity() -> usize {
    1000
}

fn default_ingress_pipeline() -> String {
    "basic-safety".to_string()
}

fn default_midstream_pipeline() -> String {
    "fast-triage".to_string()
}

fn default_egress_pipeline() -> String {
    "comprehensive-safety".to_string()
}

fn default_safety_threshold() -> f32 {
    0.7
}

fn default_chunk_threshold() -> f32 {
    0.8
}

fn default_pipeline_timeout() -> u64 {
    10
}

fn default_context_chunks() -> usize {
    5
}

fn default_stream_buffer_size() -> usize {
    100
}

fn default_true() -> bool {
    true
}
