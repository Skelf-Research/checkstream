//! Model loading and management for classifier backends.
//!
//! The implementation is intentionally lightweight and avoids heavyweight runtime
//! ML dependencies in the default production path.

use checkstream_core::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Runtime device abstraction.
#[derive(Debug, Clone)]
pub enum Device {
    Cpu,
    Cuda(usize),
    Metal(usize),
}

/// Placeholder var-builder type kept for API compatibility.
#[derive(Debug, Default)]
pub struct VarBuilder<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

/// Placeholder tokenizer type kept for API compatibility.
#[derive(Debug, Default)]
pub struct Tokenizer;

/// Configuration for loading models.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Source of the model.
    pub source: ModelSource,

    /// Path to tokenizer file (optional, for text models).
    pub tokenizer_path: Option<PathBuf>,

    /// Device to run inference on.
    pub device: DeviceType,

    /// Model format (SafeTensors, PyTorch, etc.).
    pub format: ModelFormat,

    /// Use quantization for faster inference.
    pub quantize: bool,
}

/// Source location for model weights.
#[derive(Debug, Clone)]
pub enum ModelSource {
    /// Load from local file system.
    LocalPath(PathBuf),

    /// Model identifier from Hugging Face Hub.
    HuggingFace {
        repo_id: String,
        revision: Option<String>,
        filename: String,
    },
}

/// Device type for inference.
#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    /// CPU inference (always available).
    Cpu,
    /// CUDA GPU inference (if available).
    Cuda(usize),
    /// Metal (Apple Silicon).
    Metal(usize),
}

/// Model file format.
#[derive(Debug, Clone, Copy)]
pub enum ModelFormat {
    /// SafeTensors format (recommended).
    SafeTensors,
    /// PyTorch format.
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
    /// Create a new model configuration from local path.
    pub fn from_local(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ModelSource::LocalPath(path.into()),
            ..Default::default()
        }
    }

    /// Create a new model configuration from Hugging Face.
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

    /// Set tokenizer path.
    pub fn with_tokenizer(mut self, path: impl Into<PathBuf>) -> Self {
        self.tokenizer_path = Some(path.into());
        self
    }

    /// Set device.
    pub fn with_device(mut self, device: DeviceType) -> Self {
        self.device = device;
        self
    }

    /// Enable quantization.
    pub fn with_quantization(mut self, enable: bool) -> Self {
        self.quantize = enable;
        self
    }

    /// Set model format.
    pub fn with_format(mut self, format: ModelFormat) -> Self {
        self.format = format;
        self
    }

    /// Set Hugging Face revision.
    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        if let ModelSource::HuggingFace {
            repo_id, filename, ..
        } = self.source
        {
            self.source = ModelSource::HuggingFace {
                repo_id,
                revision: Some(revision.into()),
                filename,
            };
        }
        self
    }
}

/// Loaded model with metadata and optional tokenizer state.
pub struct LoadedModel {
    var_builder: VarBuilder<'static>,
    device: Device,
    tokenizer: Option<Arc<Tokenizer>>,
    metadata: ModelMetadata,
    weights_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelMetadata {
    /// Model name/identifier.
    pub name: String,

    /// Model version.
    pub version: String,

    /// Maximum sequence length (for text models).
    pub max_seq_length: Option<usize>,

    /// Model dimensions (hidden size, etc.).
    pub hidden_size: Option<usize>,

    /// Number of labels (for classification).
    pub num_labels: Option<usize>,
}

impl LoadedModel {
    /// Load a model from configuration.
    pub fn load(config: ModelConfig) -> Result<Self> {
        let weights_path = Self::resolve_model_path(&config)?;
        let device = Self::create_device(config.device)?;
        let tokenizer = Self::load_tokenizer(&config)?;
        let metadata = Self::extract_metadata(&weights_path)?;

        let _ = config.format;

        Ok(Self {
            var_builder: VarBuilder::default(),
            device,
            tokenizer,
            metadata,
            weights_path,
        })
    }

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
            } => Err(checkstream_core::Error::config(format!(
                "HuggingFace model loading is disabled in hardened mode (repo={}, revision={:?}, file={})",
                repo_id, revision, filename
            ))),
        }
    }

    fn create_device(device_type: DeviceType) -> Result<Device> {
        match device_type {
            DeviceType::Cpu => Ok(Device::Cpu),
            DeviceType::Cuda(idx) => Err(checkstream_core::Error::classifier(format!(
                "CUDA device {} requires external ML runtime",
                idx
            ))),
            DeviceType::Metal(idx) => Err(checkstream_core::Error::classifier(format!(
                "Metal device {} requires external ML runtime",
                idx
            ))),
        }
    }

    fn load_tokenizer(config: &ModelConfig) -> Result<Option<Arc<Tokenizer>>> {
        if let Some(tokenizer_path) = &config.tokenizer_path {
            if !tokenizer_path.exists() {
                return Err(checkstream_core::Error::classifier(format!(
                    "Tokenizer file not found: {:?}",
                    tokenizer_path
                )));
            }
            return Ok(Some(Arc::new(Tokenizer)));
        }

        Ok(None)
    }

    fn extract_metadata(weights_path: &Path) -> Result<ModelMetadata> {
        let name = weights_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ModelMetadata {
            name,
            version: "1.0".to_string(),
            max_seq_length: Some(512),
            hidden_size: Some(768),
            num_labels: Some(2),
        })
    }

    /// Get reference to var-builder placeholder.
    pub fn var_builder(&self) -> &VarBuilder<'static> {
        &self.var_builder
    }

    /// Get reference to the device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get reference to the tokenizer.
    pub fn tokenizer(&self) -> Option<&Arc<Tokenizer>> {
        self.tokenizer.as_ref()
    }

    /// Get model metadata.
    pub fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    /// Check if model has a tokenizer.
    pub fn has_tokenizer(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// Get weights path.
    pub fn weights_path(&self) -> &Path {
        &self.weights_path
    }
}

/// Model registry for managing multiple loaded models.
pub struct ModelRegistry {
    models: HashMap<String, Arc<LoadedModel>>,
}

impl ModelRegistry {
    /// Create a new model registry.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model with a name.
    pub fn register(&mut self, name: impl Into<String>, model: LoadedModel) {
        self.models.insert(name.into(), Arc::new(model));
    }

    /// Get a model by name.
    pub fn get(&self, name: &str) -> Option<Arc<LoadedModel>> {
        self.models.get(name).cloned()
    }

    /// Load and register a model from configuration.
    pub fn load_and_register(
        &mut self,
        name: impl Into<String>,
        config: ModelConfig,
    ) -> Result<()> {
        let model = LoadedModel::load(config)?;
        self.register(name, model);
        Ok(())
    }

    /// Check if a model is registered.
    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    /// Get list of registered model names.
    pub fn model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// Clear all models from registry.
    pub fn clear(&mut self) {
        self.models.clear();
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to discover model files in a directory.
pub fn discover_models(models_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut model_paths = Vec::new();

    let entries = std::fs::read_dir(models_dir.as_ref()).map_err(|e| {
        checkstream_core::Error::config(format!("Failed to read models directory: {}", e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            checkstream_core::Error::config(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();

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

        if let ModelSource::HuggingFace {
            repo_id,
            revision,
            filename,
        } = &config.source
        {
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
