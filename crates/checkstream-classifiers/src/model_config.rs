//! Model configuration and registry structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Model registry containing all available models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub version: String,
    pub models: HashMap<String, ModelConfig>,
}

/// Configuration for a single model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name
    #[serde(default)]
    pub name: String,

    /// Model version
    #[serde(default)]
    pub version: String,

    /// Model description
    #[serde(default)]
    pub description: String,

    /// Model source (where to load from)
    pub source: ModelSource,

    /// Model architecture configuration
    pub architecture: ArchitectureConfig,

    /// Inference settings
    #[serde(default)]
    pub inference: InferenceConfig,

    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,

    /// Preprocessing steps
    #[serde(default)]
    pub preprocessing: Vec<PreprocessingStep>,
}

/// Model source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ModelSource {
    /// Load from local filesystem
    Local {
        path: PathBuf,
    },

    /// Download from HuggingFace Hub
    HuggingFace {
        repo: String,
        #[serde(default = "default_revision")]
        revision: String,
    },

    /// Use built-in implementation
    Builtin {
        implementation: String,
    },
}

fn default_revision() -> String {
    "main".to_string()
}

/// Model architecture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ArchitectureConfig {
    /// BERT for sequence classification
    BertSequenceClassification {
        num_labels: usize,
        #[serde(default)]
        labels: Vec<String>,
    },

    /// DistilBERT for sequence classification
    DistilBertSequenceClassification {
        num_labels: usize,
        #[serde(default)]
        labels: Vec<String>,
    },

    /// RoBERTa for sequence classification
    RobertaSequenceClassification {
        num_labels: usize,
        #[serde(default)]
        labels: Vec<String>,
    },

    /// Sentence transformer (embedding-based)
    SentenceTransformer {
        #[serde(default = "default_pooling")]
        pooling: String,
    },

    /// DeBERTa for sequence classification
    DebertaSequenceClassification {
        num_labels: usize,
        #[serde(default)]
        labels: Vec<String>,
    },

    /// Custom architecture (requires code implementation)
    Custom {
        implementation: String,
    },
}

fn default_pooling() -> String {
    "mean".to_string()
}

/// Inference configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Device to run on (cpu, cuda, mps)
    #[serde(default = "default_device")]
    pub device: String,

    /// Maximum sequence length
    #[serde(default = "default_max_length")]
    pub max_length: usize,

    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Classification threshold
    #[serde(default = "default_threshold")]
    pub threshold: f32,

    /// Quantization settings
    #[serde(default)]
    pub quantization: Option<QuantizationConfig>,
}

fn default_device() -> String {
    "cpu".to_string()
}

fn default_max_length() -> usize {
    512
}

fn default_batch_size() -> usize {
    1
}

fn default_threshold() -> f32 {
    0.5
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            device: default_device(),
            max_length: default_max_length(),
            batch_size: default_batch_size(),
            threshold: default_threshold(),
            quantization: None,
        }
    }
}

/// Quantization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    pub enabled: bool,
    #[serde(default = "default_quantization_method")]
    pub method: String,
    #[serde(default = "default_quantization_dtype")]
    pub dtype: String,
}

fn default_quantization_method() -> String {
    "dynamic".to_string()
}

fn default_quantization_dtype() -> String {
    "int8".to_string()
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output type (multi-label, single-label, regression)
    #[serde(default = "default_output_type")]
    pub output_type: String,

    /// Aggregation method for multi-label (max, mean, any)
    #[serde(default = "default_aggregation")]
    pub aggregation: String,
}

fn default_output_type() -> String {
    "single-label".to_string()
}

fn default_aggregation() -> String {
    "max".to_string()
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            output_type: default_output_type(),
            aggregation: default_aggregation(),
        }
    }
}

/// Preprocessing step configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PreprocessingStep {
    Lowercase,
    RemoveUrls,
    Truncate { max_length: usize },
    NormalizeWhitespace,
    RemoveEmojis,
    Custom { implementation: String },
}

impl ModelRegistry {
    /// Load model registry from YAML file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let registry: ModelRegistry = serde_yaml::from_str(&contents)?;
        Ok(registry)
    }

    /// Get a model configuration by name
    pub fn get_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_config() {
        let yaml = r#"
version: "1.0"
models:
  toxicity:
    name: "toxic-bert"
    version: "1.0"
    description: "BERT-based toxicity classifier"
    source:
      type: huggingface
      repo: "unitary/toxic-bert"
      revision: "main"
    architecture:
      type: bert-sequence-classification
      num_labels: 6
      labels:
        - toxic
        - severe_toxic
        - obscene
        - threat
        - insult
        - identity_hate
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
    output:
      output_type: "multi-label"
      aggregation: "max"
"#;

        let registry: ModelRegistry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(registry.version, "1.0");
        assert_eq!(registry.models.len(), 1);

        let toxicity = registry.get_model("toxicity").unwrap();
        assert_eq!(toxicity.name, "toxic-bert");
        assert_eq!(toxicity.inference.max_length, 512);
    }

    #[test]
    fn test_local_source() {
        let yaml = r#"
version: "1.0"
models:
  local-model:
    source:
      type: local
      path: "./models/my-model"
    architecture:
      type: bert-sequence-classification
      num_labels: 2
"#;

        let registry: ModelRegistry = serde_yaml::from_str(yaml).unwrap();
        let model = registry.get_model("local-model").unwrap();

        match &model.source {
            ModelSource::Local { path } => {
                assert_eq!(path.to_str().unwrap(), "./models/my-model");
            }
            _ => panic!("Expected local source"),
        }
    }
}
