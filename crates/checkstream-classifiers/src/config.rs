//! Configuration for classifiers and model loading

use crate::{DeviceType, ModelConfig, ModelFormat, ModelSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for all classifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierConfig {
    /// Model configurations by name
    #[serde(default)]
    pub models: HashMap<String, ModelConfigSpec>,

    /// Pipeline configurations
    #[serde(default)]
    pub pipelines: HashMap<String, PipelineConfigSpec>,

    /// Default device to use
    #[serde(default)]
    pub default_device: DeviceSpec,

    /// Enable quantization by default
    #[serde(default)]
    pub default_quantize: bool,

    /// Model cache directory
    #[serde(default = "default_models_dir")]
    pub models_dir: PathBuf,
}

/// Pipeline configuration specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfigSpec {
    /// Pipeline description
    pub description: Option<String>,

    /// Pipeline stages
    pub stages: Vec<StageConfigSpec>,
}

/// Stage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StageConfigSpec {
    /// Single classifier execution
    Single {
        name: String,
        classifier: String,
    },

    /// Parallel execution
    Parallel {
        name: String,
        classifiers: Vec<String>,
        aggregation: AggregationStrategySpec,
    },

    /// Sequential execution
    Sequential {
        name: String,
        classifiers: Vec<String>,
    },

    /// Conditional execution
    Conditional {
        name: String,
        classifier: String,
        condition: ConditionSpec,
    },
}

/// Aggregation strategy specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationStrategySpec {
    All,
    MaxScore,
    MinScore,
    FirstPositive { threshold: f32 },
    Unanimous,
    WeightedAverage,
}

/// Condition specification for conditional execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionSpec {
    /// Execute if any previous result score > threshold
    AnyAboveThreshold { threshold: f32 },

    /// Execute if all previous results score > threshold
    AllAboveThreshold { threshold: f32 },

    /// Execute if specific classifier triggered
    ClassifierTriggered { classifier: String },

    /// Always execute
    Always,
}

/// Model configuration specification (for YAML/config files)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigSpec {
    /// Model source specification
    #[serde(flatten)]
    pub source: ModelSourceSpec,

    /// Tokenizer path (optional)
    pub tokenizer: Option<PathBuf>,

    /// Device override
    pub device: Option<DeviceSpec>,

    /// Model format
    #[serde(default)]
    pub format: ModelFormatSpec,

    /// Enable quantization
    pub quantize: Option<bool>,

    /// Model tier (for latency targeting)
    pub tier: Option<String>,
}

/// Model source specification (for config files)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelSourceSpec {
    /// Local file path
    Local {
        path: PathBuf,
    },

    /// Hugging Face Hub
    HuggingFace {
        repo_id: String,
        filename: String,
        revision: Option<String>,
    },
}

/// Device specification (for config files)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceSpec {
    Cpu,
    Cuda { index: Option<usize> },
    Metal { index: Option<usize> },
}

impl Default for DeviceSpec {
    fn default() -> Self {
        Self::Cpu
    }
}

/// Model format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormatSpec {
    SafeTensors,
    PyTorch,
}

impl Default for ModelFormatSpec {
    fn default() -> Self {
        Self::SafeTensors
    }
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            pipelines: HashMap::new(),
            default_device: DeviceSpec::Cpu,
            default_quantize: false,
            models_dir: default_models_dir(),
        }
    }
}

impl ClassifierConfig {
    /// Load from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load from file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&content)?)
    }

    /// Convert to ModelConfig for loading
    pub fn to_model_config(&self, name: &str) -> Option<ModelConfig> {
        let spec = self.models.get(name)?;

        let source = match &spec.source {
            ModelSourceSpec::Local { path } => ModelSource::LocalPath(path.clone()),
            ModelSourceSpec::HuggingFace {
                repo_id,
                filename,
                revision,
            } => ModelSource::HuggingFace {
                repo_id: repo_id.clone(),
                revision: revision.clone(),
                filename: filename.clone(),
            },
        };

        let device = spec
            .device
            .as_ref()
            .unwrap_or(&self.default_device)
            .to_device_type();

        let format = match spec.format {
            ModelFormatSpec::SafeTensors => ModelFormat::SafeTensors,
            ModelFormatSpec::PyTorch => ModelFormat::PyTorch,
        };

        let quantize = spec.quantize.unwrap_or(self.default_quantize);

        let config = ModelConfig {
            source,
            tokenizer_path: spec.tokenizer.clone(),
            device,
            format,
            quantize,
        };

        Some(config)
    }

    /// Get all model names
    pub fn model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// Get all pipeline names
    pub fn pipeline_names(&self) -> Vec<String> {
        self.pipelines.keys().cloned().collect()
    }

    /// Get pipeline configuration by name
    pub fn get_pipeline(&self, name: &str) -> Option<&PipelineConfigSpec> {
        self.pipelines.get(name)
    }
}

impl AggregationStrategySpec {
    /// Convert to runtime aggregation strategy
    pub fn to_aggregation_strategy(&self) -> crate::pipeline::AggregationStrategy {
        match self {
            Self::All => crate::pipeline::AggregationStrategy::All,
            Self::MaxScore => crate::pipeline::AggregationStrategy::MaxScore,
            Self::MinScore => crate::pipeline::AggregationStrategy::MinScore,
            Self::FirstPositive { threshold } => {
                crate::pipeline::AggregationStrategy::FirstPositive(*threshold)
            }
            Self::Unanimous => crate::pipeline::AggregationStrategy::Unanimous,
            Self::WeightedAverage => crate::pipeline::AggregationStrategy::WeightedAverage,
        }
    }
}

impl ConditionSpec {
    /// Convert to runtime condition function
    pub fn to_condition_fn(&self) -> Box<dyn Fn(&[crate::pipeline::PipelineResult]) -> bool + Send + Sync> {
        match self {
            Self::AnyAboveThreshold { threshold } => {
                let threshold = *threshold;
                Box::new(move |results| {
                    results.iter().any(|r| r.result.score > threshold)
                })
            }
            Self::AllAboveThreshold { threshold } => {
                let threshold = *threshold;
                Box::new(move |results| {
                    !results.is_empty() && results.iter().all(|r| r.result.score > threshold)
                })
            }
            Self::ClassifierTriggered { classifier } => {
                let classifier_name = classifier.clone();
                Box::new(move |results| {
                    results.iter().any(|r| r.classifier_name == classifier_name && r.result.score > 0.5)
                })
            }
            Self::Always => {
                Box::new(|_| true)
            }
        }
    }
}

impl DeviceSpec {
    /// Convert to DeviceType
    pub fn to_device_type(&self) -> DeviceType {
        match self {
            DeviceSpec::Cpu => DeviceType::Cpu,
            DeviceSpec::Cuda { index } => DeviceType::Cuda(index.unwrap_or(0)),
            DeviceSpec::Metal { index } => DeviceType::Metal(index.unwrap_or(0)),
        }
    }
}

fn default_models_dir() -> PathBuf {
    PathBuf::from("./models")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classifier_config_yaml() {
        let yaml = r#"
models:
  toxicity:
    repo_id: unitary/toxic-bert
    filename: model.safetensors
    device: cpu
    quantize: true
    tier: B

  sentiment:
    path: ./models/sentiment.safetensors
    tokenizer: ./models/sentiment-tokenizer.json
    device: cpu
    quantize: false

default_device: cpu
default_quantize: true
models_dir: ./my-models
"#;

        let config = ClassifierConfig::from_yaml(yaml).unwrap();

        assert_eq!(config.models.len(), 2);
        assert!(config.models.contains_key("toxicity"));
        assert!(config.models.contains_key("sentiment"));
        assert_eq!(config.models_dir, PathBuf::from("./my-models"));
        assert!(config.default_quantize);
    }

    #[test]
    fn test_to_model_config() {
        let yaml = r#"
models:
  test:
    repo_id: test/model
    filename: model.safetensors
    quantize: true
"#;

        let config = ClassifierConfig::from_yaml(yaml).unwrap();
        let model_config = config.to_model_config("test").unwrap();

        assert!(matches!(model_config.source, ModelSource::HuggingFace { .. }));
        assert!(model_config.quantize);
    }

    #[test]
    fn test_device_spec() {
        let yaml_cpu = r#"cpu"#;
        let spec: DeviceSpec = serde_yaml::from_str(yaml_cpu).unwrap();
        assert!(matches!(spec, DeviceSpec::Cpu));

        // Test device conversion
        let cpu_device = DeviceSpec::Cpu.to_device_type();
        assert!(matches!(cpu_device, DeviceType::Cpu));

        let cuda_device = DeviceSpec::Cuda { index: Some(1) }.to_device_type();
        assert!(matches!(cuda_device, DeviceType::Cuda(1)));

        let metal_device = DeviceSpec::Metal { index: None }.to_device_type();
        assert!(matches!(metal_device, DeviceType::Metal(0)));
    }

    #[test]
    fn test_device_spec_in_model() {
        // Test device spec in full model config
        let yaml = r#"
models:
  test:
    repo_id: test/model
    filename: model.safetensors
    device: cpu
"#;
        let config = ClassifierConfig::from_yaml(yaml).unwrap();
        assert!(config.models.contains_key("test"));
    }
}
