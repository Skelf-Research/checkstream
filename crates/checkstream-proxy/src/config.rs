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

    /// Token buffer holdback size
    #[serde(default = "default_holdback")]
    pub token_holdback: usize,

    /// Maximum buffer capacity
    #[serde(default = "default_buffer_capacity")]
    pub max_buffer_capacity: usize,

    /// Telemetry configuration
    #[serde(default)]
    pub telemetry: TelemetryConfig,
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
            token_holdback: default_holdback(),
            max_buffer_capacity: default_buffer_capacity(),
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

fn default_holdback() -> usize {
    10
}

fn default_buffer_capacity() -> usize {
    1000
}

fn default_true() -> bool {
    true
}
