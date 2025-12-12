//! Proxy configuration
//!
//! Supports both single-tenant (backward compatible) and multi-tenant configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

// =============================================================================
// Multi-Tenant Configuration
// =============================================================================

/// Streaming format for backend responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StreamFormat {
    /// OpenAI SSE format (default): data: {...}\n\n with choices[0].delta.content
    #[default]
    OpenAi,
    /// Anthropic SSE format: event types with delta.text
    Anthropic,
    /// Custom configurable format
    Custom(StreamFormatConfig),
}

/// Configuration for custom streaming formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFormatConfig {
    /// Format type: "sse" or "ndjson"
    #[serde(default = "default_stream_format_type")]
    pub format: String,
    /// JSONPath to content field (e.g., "data.content" or "choices[0].delta.content")
    pub content_path: String,
    /// Marker indicating stream end (e.g., "[DONE]" or {"done": true})
    #[serde(default)]
    pub done_marker: Option<String>,
    /// Event types to extract content from (for event-based streams)
    #[serde(default)]
    pub content_events: Vec<String>,
}

fn default_stream_format_type() -> String {
    "sse".to_string()
}

/// Per-tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Unique tenant identifier
    pub id: String,
    /// Display name for the tenant
    #[serde(default)]
    pub name: String,
    /// Backend LLM API URL for this tenant
    pub backend_url: String,
    /// Policy file path or policy pack name
    #[serde(default = "default_policy_path")]
    pub policy_path: String,
    /// Classifiers configuration file path (optional, can share with default)
    #[serde(default)]
    pub classifiers_config: Option<String>,
    /// Streaming format for this tenant's backend
    #[serde(default)]
    pub stream_format: StreamFormat,
    /// Pipeline settings (optional, inherits from default if not set)
    #[serde(default)]
    pub pipelines: Option<PipelineSettings>,
    /// API keys that map to this tenant (for API key-based routing)
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Token buffer holdback size (optional, inherits from default)
    #[serde(default)]
    pub token_holdback: Option<usize>,
    /// Maximum buffer capacity (optional, inherits from default)
    #[serde(default)]
    pub max_buffer_capacity: Option<usize>,
}

fn default_policy_path() -> String {
    "./policies/default.yaml".to_string()
}

/// Multi-tenant configuration wrapper
///
/// This extends ProxyConfig with multi-tenant support while maintaining
/// backward compatibility with single-tenant configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTenantConfig {
    /// Default/fallback configuration (backward compatible with ProxyConfig)
    #[serde(flatten)]
    pub default: ProxyConfig,
    /// Named tenant configurations
    #[serde(default)]
    pub tenants: HashMap<String, TenantConfig>,
}

impl MultiTenantConfig {
    /// Load multi-tenant configuration from file
    pub fn load(config_path: &str, cli: &crate::Cli) -> anyhow::Result<Self> {
        let mut config = if Path::new(config_path).exists() {
            let content = std::fs::read_to_string(config_path)?;
            serde_yaml::from_str(&content)?
        } else {
            Self::default()
        };

        // Apply CLI overrides to default config
        if let Some(backend) = &cli.backend {
            config.default.backend_url = backend.clone();
        }

        if let Some(policy) = &cli.policy {
            config.default.policy_path = policy.clone();
        }

        Ok(config)
    }

    /// Check if multi-tenant mode is enabled
    pub fn is_multi_tenant(&self) -> bool {
        !self.tenants.is_empty()
    }

    /// Get tenant config by ID, falling back to default
    pub fn get_tenant(&self, tenant_id: &str) -> Option<&TenantConfig> {
        self.tenants.get(tenant_id)
    }

    /// Build API key to tenant ID mapping
    pub fn build_api_key_index(&self) -> HashMap<String, String> {
        let mut index = HashMap::new();
        for (tenant_id, config) in &self.tenants {
            for api_key in &config.api_keys {
                index.insert(api_key.clone(), tenant_id.clone());
            }
        }
        index
    }
}

impl Default for MultiTenantConfig {
    fn default() -> Self {
        Self {
            default: ProxyConfig::default(),
            tenants: HashMap::new(),
        }
    }
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
