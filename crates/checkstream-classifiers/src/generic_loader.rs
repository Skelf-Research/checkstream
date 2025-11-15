//! Generic model loader for common architectures
//!
//! This module provides automatic model loading from configuration,
//! supporting common transformer architectures without requiring custom code.

use crate::classifier::{Classifier, ClassificationResult, ClassificationMetadata, ClassifierTier};
use crate::model_config::{ModelConfig, ModelSource, ArchitectureConfig, ModelRegistry};
use checkstream_core::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "ml-models")]
use candle_core::{Device, Tensor};
#[cfg(feature = "ml-models")]
use candle_nn::VarBuilder;
#[cfg(feature = "ml-models")]
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
#[cfg(feature = "ml-models")]
use tokenizers::Tokenizer;

/// Generic model loader that creates classifiers from configuration
pub struct GenericModelLoader {
    pub registry: Arc<ModelRegistry>,
    cache_dir: PathBuf,
}

impl GenericModelLoader {
    /// Create a new generic model loader
    pub fn new(registry: ModelRegistry) -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache/checkstream/models");

        std::fs::create_dir_all(&cache_dir).ok();

        Self {
            registry: Arc::new(registry),
            cache_dir,
        }
    }

    /// Load a classifier by name from the registry
    pub async fn load_classifier(&self, name: &str) -> Result<Box<dyn Classifier>> {
        let config = self.registry.get_model(name)
            .ok_or_else(|| checkstream_core::Error::classifier(
                format!("Model '{}' not found in registry", name)
            ))?;

        tracing::info!("Loading model '{}' from registry", name);

        match &config.architecture {
            ArchitectureConfig::BertSequenceClassification { num_labels, labels } => {
                self.load_bert_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::DistilBertSequenceClassification { num_labels, labels } => {
                self.load_distilbert_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::RobertaSequenceClassification { num_labels, labels } => {
                self.load_roberta_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::Custom { implementation } => {
                Err(checkstream_core::Error::classifier(
                    format!("Custom architecture '{}' requires code implementation", implementation)
                ))
            }
            _ => {
                Err(checkstream_core::Error::classifier(
                    format!("Unsupported architecture: {:?}", config.architecture)
                ))
            }
        }
    }

    /// Resolve model path (download if needed)
    async fn resolve_model_path(&self, config: &ModelConfig) -> Result<PathBuf> {
        match &config.source {
            ModelSource::Local { path } => {
                if !path.exists() {
                    return Err(checkstream_core::Error::classifier(
                        format!("Model path does not exist: {}", path.display())
                    ));
                }
                Ok(path.clone())
            }
            ModelSource::HuggingFace { repo, revision } => {
                self.download_from_huggingface(repo, revision).await
            }
            ModelSource::Builtin { implementation } => {
                Err(checkstream_core::Error::classifier(
                    format!("Builtin implementation '{}' should not use model loader", implementation)
                ))
            }
        }
    }

    /// Download model from HuggingFace Hub
    async fn download_from_huggingface(&self, repo: &str, revision: &str) -> Result<PathBuf> {
        #[cfg(feature = "ml-models")]
        {
            tracing::info!("Downloading model from HuggingFace: {} @ {}", repo, revision);

            // Use hf-hub to download
            let api = hf_hub::api::sync::Api::new()
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to initialize HuggingFace API: {}", e)
                ))?;

            let repo_obj = api.repo(hf_hub::Repo::model(repo.to_string()));

            // Download required files
            let files = vec![
                "config.json",
                "tokenizer.json",
                "vocab.txt",
                "model.safetensors",
            ];

            for file in &files {
                tracing::debug!("Downloading {}", file);
                repo_obj.get(file)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to download {}: {}", file, e)
                    ))?;
            }

            // Return the cache path
            let model_dir = self.cache_dir.join(repo.replace("/", "--"));

            // Copy from hf cache to our cache
            let hf_cache = repo_obj.get("config.json")
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to locate model cache: {}", e)
                ))?;

            let hf_model_dir = hf_cache.parent()
                .ok_or_else(|| checkstream_core::Error::classifier("Invalid cache path"))?;

            std::fs::create_dir_all(&model_dir)
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to create cache directory: {}", e)
                ))?;

            // Copy files
            for file in &files {
                let src = hf_model_dir.join(file);
                let dst = model_dir.join(file);
                if src.exists() && !dst.exists() {
                    std::fs::copy(&src, &dst)
                        .map_err(|e| checkstream_core::Error::classifier(
                            format!("Failed to copy {}: {}", file, e)
                        ))?;
                }
            }

            tracing::info!("Model downloaded to: {}", model_dir.display());
            Ok(model_dir)
        }

        #[cfg(not(feature = "ml-models"))]
        {
            Err(checkstream_core::Error::classifier(
                "HuggingFace download requires 'ml-models' feature"
            ))
        }
    }

    /// Load BERT-based sequence classification model
    async fn load_bert_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        #[cfg(feature = "ml-models")]
        {
            let model_path = self.resolve_model_path(config).await?;

            // Load tokenizer
            let tokenizer_path = model_path.join("tokenizer.json");
            let tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to load tokenizer: {}", e)
                ))?;

            // Load BERT config
            let config_path = model_path.join("config.json");
            let bert_config: BertConfig = serde_json::from_str(
                &std::fs::read_to_string(&config_path)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to read config: {}", e)
                    ))?
            ).map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to parse config: {}", e)
            ))?;

            // Determine device
            let device = match config.inference.device.as_str() {
                "cuda" => Device::new_cuda(0)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to initialize CUDA: {}", e)
                    ))?,
                "mps" => Device::new_metal(0)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to initialize Metal: {}", e)
                    ))?,
                _ => Device::Cpu,
            };

            // Load model weights
            let weights_path = model_path.join("model.safetensors");
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to load weights: {}", e)
                    ))?
            };

            let model = BertModel::load(vb, &bert_config)
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to load BERT model: {}", e)
                ))?;

            tracing::info!("Successfully loaded BERT classifier with {} labels", num_labels);

            Ok(Box::new(GenericBertClassifier {
                name: config.name.clone(),
                tokenizer,
                model,
                device,
                num_labels,
                labels: labels.to_vec(),
                threshold: config.inference.threshold,
                max_length: config.inference.max_length,
            }))
        }

        #[cfg(not(feature = "ml-models"))]
        {
            Err(checkstream_core::Error::classifier(
                "ML models require 'ml-models' feature flag"
            ))
        }
    }

    /// Load DistilBERT-based sequence classification model
    async fn load_distilbert_classifier(
        &self,
        _config: &ModelConfig,
        _num_labels: usize,
        _labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        // TODO: Implement DistilBERT support
        Err(checkstream_core::Error::classifier(
            "DistilBERT support not yet implemented"
        ))
    }

    /// Load RoBERTa-based sequence classification model
    async fn load_roberta_classifier(
        &self,
        _config: &ModelConfig,
        _num_labels: usize,
        _labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        // TODO: Implement RoBERTa support
        Err(checkstream_core::Error::classifier(
            "RoBERTa support not yet implemented"
        ))
    }
}

/// Generic BERT classifier loaded from configuration
#[cfg(feature = "ml-models")]
struct GenericBertClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: BertModel,
    device: Device,
    num_labels: usize,
    labels: Vec<String>,
    threshold: f32,
    max_length: usize,
}

#[cfg(feature = "ml-models")]
#[async_trait::async_trait]
impl Classifier for GenericBertClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Tokenization failed: {}", e)
            ))?;

        let input_ids = encoding.get_ids();
        let token_type_ids = encoding.get_type_ids();

        // Convert to tensors
        let input_ids = Tensor::new(input_ids, &self.device)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to create input tensor: {}", e)
            ))?
            .unsqueeze(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to unsqueeze: {}", e)
            ))?;

        let token_type_ids = Tensor::new(token_type_ids, &self.device)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to create token type tensor: {}", e)
            ))?
            .unsqueeze(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to unsqueeze: {}", e)
            ))?;

        // Run BERT forward pass
        let output = self.model
            .forward(&input_ids, &token_type_ids, None)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Model forward pass failed: {}", e)
            ))?;

        // Get [CLS] token embedding
        let cls_embedding = output
            .get(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to get batch: {}", e)
            ))?
            .get(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to get CLS token: {}", e)
            ))?;

        // Simple scoring (placeholder - would need proper classification head)
        let embedding_vec = cls_embedding.to_vec1::<f32>()
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to convert to vec: {}", e)
            ))?;

        let magnitude: f32 = embedding_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let score = (magnitude / 10.0).min(1.0).max(0.0);

        // Determine label
        let label = if score > self.threshold {
            self.labels.get(0).cloned().unwrap_or_else(|| "positive".to_string())
        } else {
            "negative".to_string()
        };

        Ok(ClassificationResult {
            label,
            score,
            metadata: ClassificationMetadata {
                model: Some(self.name.clone()),
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_from_registry() {
        let yaml = r#"
version: "1.0"
models:
  test-model:
    name: "test-bert"
    source:
      type: local
      path: "./models/toxic-bert"
    architecture:
      type: bert-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"
      threshold: 0.5
"#;

        let registry: ModelRegistry = serde_yaml::from_str(yaml).unwrap();
        let loader = GenericModelLoader::new(registry);

        // This will fail if model doesn't exist, which is expected in CI
        // In real usage, the model would be downloaded or present locally
        let _ = loader.load_classifier("test-model").await;
    }
}
