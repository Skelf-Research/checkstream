//! Model loading and management for Candle-based ML classifiers

use candle_core::{Device, DType};
use candle_nn::VarBuilder;
use checkstream_core::Result;
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokenizers::Tokenizer;

/// Configuration for loading Candle models
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Source of the model
    pub source: ModelSource,

    /// Path to tokenizer file (optional, for text models)
    pub tokenizer_path: Option<PathBuf>,

    /// Device to run inference on
    pub device: DeviceType,

    /// Model format (SafeTensors, PyTorch, etc.)
    pub format: ModelFormat,

    /// Use quantization for faster inference
    pub quantize: bool,
}

/// Source location for model weights
#[derive(Debug, Clone)]
pub enum ModelSource {
    /// Load from local file system
    LocalPath(PathBuf),

    /// Download from Hugging Face Hub
    HuggingFace {
        repo_id: String,
        revision: Option<String>,
        filename: String,
    },
}

/// Device type for inference
#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    /// CPU inference (always available)
    Cpu,
    /// CUDA GPU inference (if available)
    Cuda(usize), // GPU index
    /// Metal (Apple Silicon)
    Metal(usize),
}

/// Model file format
#[derive(Debug, Clone, Copy)]
pub enum ModelFormat {
    /// SafeTensors format (recommended)
    SafeTensors,
    /// PyTorch format
    PyTorch,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            source: ModelSource::LocalPath(PathBuf::new()),
            tokenizer_path: None,
            device: DeviceType::Cpu,
            format: ModelFormat::SafeTensors,
            quantize: false,
        }
    }
}

impl ModelConfig {
    /// Create a new model configuration from local path
    pub fn from_local(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ModelSource::LocalPath(path.into()),
            ..Default::default()
        }
    }

    /// Create a new model configuration from Hugging Face
    pub fn from_hf(repo_id: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            source: ModelSource::HuggingFace {
                repo_id: repo_id.into(),
                revision: None,
                filename: filename.into(),
            },
            ..Default::default()
        }
    }

    /// Set tokenizer path
    pub fn with_tokenizer(mut self, path: impl Into<PathBuf>) -> Self {
        self.tokenizer_path = Some(path.into());
        self
    }

    /// Set device
    pub fn with_device(mut self, device: DeviceType) -> Self {
        self.device = device;
        self
    }

    /// Enable quantization
    pub fn with_quantization(mut self, enable: bool) -> Self {
        self.quantize = enable;
        self
    }

    /// Set model format
    pub fn with_format(mut self, format: ModelFormat) -> Self {
        self.format = format;
        self
    }

    /// Set Hugging Face revision
    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        if let ModelSource::HuggingFace { repo_id, filename, .. } = self.source {
            self.source = ModelSource::HuggingFace {
                repo_id,
                revision: Some(revision.into()),
                filename,
            };
        }
        self
    }
}

/// Loaded Candle model with weights and tokenizer
pub struct LoadedModel {
    /// VarBuilder for loading model weights
    var_builder: VarBuilder<'static>,

    /// Device the model is on
    device: Device,

    /// Tokenizer for text preprocessing
    tokenizer: Option<Arc<Tokenizer>>,

    /// Model metadata
    metadata: ModelMetadata,

    /// Model weights path (for reference)
    weights_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelMetadata {
    /// Model name/identifier
    pub name: String,

    /// Model version
    pub version: String,

    /// Maximum sequence length (for text models)
    pub max_seq_length: Option<usize>,

    /// Model dimensions (hidden size, etc.)
    pub hidden_size: Option<usize>,

    /// Number of labels (for classification)
    pub num_labels: Option<usize>,
}

impl LoadedModel {
    /// Load a model from configuration
    pub fn load(config: ModelConfig) -> Result<Self> {
        // Get model weights path
        let weights_path = Self::resolve_model_path(&config)?;

        // Create device
        let device = Self::create_device(config.device)?;

        // Load weights into VarBuilder
        let var_builder = match config.format {
            ModelFormat::SafeTensors => {
                VarBuilder::from_pth(&weights_path, DType::F32, &device)
                    .map_err(|e| checkstream_core::Error::classifier(format!("Failed to load SafeTensors: {}", e)))?
            }
            ModelFormat::PyTorch => {
                VarBuilder::from_pth(&weights_path, DType::F32, &device)
                    .map_err(|e| checkstream_core::Error::classifier(format!("Failed to load PyTorch weights: {}", e)))?
            }
        };

        // Load tokenizer if specified
        let tokenizer = Self::load_tokenizer(&config)?;

        // Extract or create metadata
        let metadata = Self::extract_metadata(&weights_path)?;

        Ok(Self {
            var_builder,
            device,
            tokenizer,
            metadata,
            weights_path,
        })
    }

    /// Resolve model path from source
    fn resolve_model_path(config: &ModelConfig) -> Result<PathBuf> {
        match &config.source {
            ModelSource::LocalPath(path) => {
                if !path.exists() {
                    return Err(checkstream_core::Error::config(format!(
                        "Model file not found: {:?}",
                        path
                    )));
                }
                Ok(path.clone())
            }
            ModelSource::HuggingFace {
                repo_id,
                revision,
                filename,
            } => {
                // Download from Hugging Face Hub
                let api = Api::new()
                    .map_err(|e| checkstream_core::Error::config(format!("Failed to initialize HF API: {}", e)))?;

                let repo = api.repo(Repo::with_revision(
                    repo_id.clone(),
                    RepoType::Model,
                    revision.clone().unwrap_or_else(|| "main".to_string()),
                ));

                let model_path = repo
                    .get(filename)
                    .map_err(|e| checkstream_core::Error::config(format!("Failed to download model from HF: {}", e)))?;

                Ok(model_path)
            }
        }
    }

    /// Create Candle device from device type
    fn create_device(device_type: DeviceType) -> Result<Device> {
        match device_type {
            DeviceType::Cpu => Ok(Device::Cpu),
            DeviceType::Cuda(idx) => {
                Device::new_cuda(idx)
                    .map_err(|e| checkstream_core::Error::classifier(format!("Failed to create CUDA device: {}", e)))
            }
            DeviceType::Metal(idx) => {
                Device::new_metal(idx)
                    .map_err(|e| checkstream_core::Error::classifier(format!("Failed to create Metal device: {}", e)))
            }
        }
    }

    /// Load tokenizer from configuration
    fn load_tokenizer(config: &ModelConfig) -> Result<Option<Arc<Tokenizer>>> {
        if let Some(tokenizer_path) = &config.tokenizer_path {
            let tok = Tokenizer::from_file(tokenizer_path)
                .map_err(|e| checkstream_core::Error::classifier(format!("Failed to load tokenizer: {}", e)))?;
            Ok(Some(Arc::new(tok)))
        } else if let ModelSource::HuggingFace { repo_id, revision, .. } = &config.source {
            // Try to download tokenizer from HF if not provided
            let api = Api::new()
                .map_err(|e| checkstream_core::Error::config(format!("Failed to initialize HF API: {}", e)))?;

            let repo = api.repo(Repo::with_revision(
                repo_id.clone(),
                RepoType::Model,
                revision.clone().unwrap_or_else(|| "main".to_string()),
            ));

            // Try common tokenizer filenames
            for filename in ["tokenizer.json", "vocab.json"] {
                if let Ok(tokenizer_path) = repo.get(filename) {
                    if let Ok(tok) = Tokenizer::from_file(&tokenizer_path) {
                        return Ok(Some(Arc::new(tok)));
                    }
                }
            }

            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Extract metadata from model
    fn extract_metadata(weights_path: &Path) -> Result<ModelMetadata> {
        // Try to extract from filename or config
        let name = weights_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ModelMetadata {
            name,
            version: "1.0".to_string(),
            max_seq_length: Some(512), // Default, can be configured
            hidden_size: Some(768),    // Common for BERT-base
            num_labels: Some(2),       // Binary classification default
        })
    }

    /// Get reference to VarBuilder for building model layers
    pub fn var_builder(&self) -> &VarBuilder<'static> {
        &self.var_builder
    }

    /// Get reference to the device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get reference to the tokenizer
    pub fn tokenizer(&self) -> Option<&Arc<Tokenizer>> {
        self.tokenizer.as_ref()
    }

    /// Get model metadata
    pub fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    /// Check if model has a tokenizer
    pub fn has_tokenizer(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// Get weights path
    pub fn weights_path(&self) -> &Path {
        &self.weights_path
    }
}

/// Model registry for managing multiple loaded models
pub struct ModelRegistry {
    models: std::collections::HashMap<String, Arc<LoadedModel>>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }

    /// Register a model with a name
    pub fn register(&mut self, name: impl Into<String>, model: LoadedModel) {
        self.models.insert(name.into(), Arc::new(model));
    }

    /// Get a model by name
    pub fn get(&self, name: &str) -> Option<Arc<LoadedModel>> {
        self.models.get(name).cloned()
    }

    /// Load and register a model from configuration
    pub fn load_and_register(&mut self, name: impl Into<String>, config: ModelConfig) -> Result<()> {
        let model = LoadedModel::load(config)?;
        self.register(name, model);
        Ok(())
    }

    /// Check if a model is registered
    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    /// Get list of registered model names
    pub fn model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// Clear all models from registry
    pub fn clear(&mut self) {
        self.models.clear();
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to discover model files in a directory
pub fn discover_models(models_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut model_paths = Vec::new();

    let entries = std::fs::read_dir(models_dir.as_ref())
        .map_err(|e| checkstream_core::Error::config(format!("Failed to read models directory: {}", e)))?;

    for entry in entries {
        let entry = entry.map_err(|e| checkstream_core::Error::config(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        // Look for SafeTensors or PyTorch files
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "safetensors" || ext == "pt" || ext == "pth" || ext == "bin" {
                model_paths.push(path);
            }
        }
    }

    Ok(model_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_local() {
        let config = ModelConfig::from_local("/path/to/model.safetensors")
            .with_tokenizer("/path/to/tokenizer.json")
            .with_device(DeviceType::Cpu)
            .with_quantization(true);

        assert!(matches!(config.source, ModelSource::LocalPath(_)));
        assert!(config.quantize);
    }

    #[test]
    fn test_model_config_hf() {
        let config = ModelConfig::from_hf("distilbert-base-uncased", "model.safetensors")
            .with_revision("main")
            .with_device(DeviceType::Cpu);

        if let ModelSource::HuggingFace { repo_id, revision, filename } = &config.source {
            assert_eq!(repo_id, "distilbert-base-uncased");
            assert_eq!(revision.as_ref().unwrap(), "main");
            assert_eq!(filename, "model.safetensors");
        } else {
            panic!("Expected HuggingFace source");
        }
    }

    #[test]
    fn test_model_registry() {
        let mut registry = ModelRegistry::new();

        assert!(!registry.has_model("test"));
        assert_eq!(registry.model_names().len(), 0);

        registry.clear();
        assert_eq!(registry.model_names().len(), 0);
    }
}
