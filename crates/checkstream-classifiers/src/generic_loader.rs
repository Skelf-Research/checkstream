//! Generic model loader for common architectures
//!
//! This module provides automatic model loading from configuration,
//! supporting common transformer architectures without requiring custom code.

use crate::classifier::{ClassificationMetadata, ClassificationResult, Classifier, ClassifierTier};
use crate::model_config::{ArchitectureConfig, ModelConfig, ModelRegistry, ModelSource};
use async_trait::async_trait;
use checkstream_core::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "ml-models")]
use candle_core::{DType, Device, IndexOp, Tensor, D};
#[cfg(feature = "ml-models")]
use candle_nn::{Linear, Module, VarBuilder};
#[cfg(feature = "ml-models")]
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
#[cfg(feature = "ml-models")]
use candle_transformers::models::distilbert::{DistilBertModel, Config as DistilBertConfig};
#[cfg(feature = "ml-models")]
use tokenizers::{Tokenizer, TruncationDirection};

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
        let config = self
            .registry
            .get_model(name)
            .ok_or_else(|| {
                checkstream_core::Error::classifier(format!(
                    "Model '{}' not found in registry",
                    name
                ))
            })?;

        tracing::info!("Loading model '{}' from registry", name);

        match &config.architecture {
            ArchitectureConfig::BertSequenceClassification { num_labels, labels } => {
                self.load_bert_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::DistilBertSequenceClassification { num_labels, labels } => {
                self.load_distilbert_classifier(config, *num_labels, labels)
                    .await
            }
            ArchitectureConfig::RobertaSequenceClassification { num_labels, labels } => {
                // RoBERTa uses same architecture as BERT, just different pretraining
                self.load_bert_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::Custom { implementation } => Err(checkstream_core::Error::classifier(
                format!(
                    "Custom architecture '{}' requires code implementation",
                    implementation
                ),
            )),
            _ => Err(checkstream_core::Error::classifier(format!(
                "Unsupported architecture: {:?}",
                config.architecture
            ))),
        }
    }

    /// Resolve model path (download if needed)
    #[cfg(feature = "ml-models")]
    async fn resolve_model_path(&self, config: &ModelConfig) -> Result<PathBuf> {
        match &config.source {
            ModelSource::Local { path } => {
                if !path.exists() {
                    return Err(checkstream_core::Error::classifier(format!(
                        "Model path does not exist: {}",
                        path.display()
                    )));
                }
                Ok(path.clone())
            }
            ModelSource::HuggingFace { repo, revision } => {
                self.download_from_huggingface(repo, revision).await
            }
            ModelSource::Builtin { implementation } => Err(checkstream_core::Error::classifier(
                format!(
                    "Builtin implementation '{}' should not use model loader",
                    implementation
                ),
            )),
        }
    }

    /// Download model from HuggingFace Hub
    #[cfg(feature = "ml-models")]
    async fn download_from_huggingface(&self, repo: &str, _revision: &str) -> Result<PathBuf> {
        tracing::info!("Downloading model from HuggingFace: {}", repo);

        // Use hf-hub to download
        let api = hf_hub::api::sync::Api::new().map_err(|e| {
            checkstream_core::Error::classifier(format!(
                "Failed to initialize HuggingFace API: {}",
                e
            ))
        })?;

        let repo_obj = api.repo(hf_hub::Repo::model(repo.to_string()));

        // Download required files - try different weight formats
        let weight_files = vec!["model.safetensors", "pytorch_model.bin"];
        let mut found_weights = false;

        for weight_file in &weight_files {
            match repo_obj.get(weight_file) {
                Ok(_) => {
                    tracing::debug!("Found weight file: {}", weight_file);
                    found_weights = true;
                    break;
                }
                Err(_) => continue,
            }
        }

        if !found_weights {
            return Err(checkstream_core::Error::classifier(
                "No model weights found (tried model.safetensors, pytorch_model.bin)",
            ));
        }

        // Download config.json (required)
        tracing::debug!("Downloading config.json");
        repo_obj.get("config.json").map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to download config.json: {}", e))
        })?;

        // Try to download tokenizer files - tokenizer.json is preferred but vocab.txt works too
        let tokenizer_files = vec![
            "tokenizer.json",
            "vocab.txt",
            "tokenizer_config.json",
            "special_tokens_map.json",
        ];

        let mut found_tokenizer = false;
        for file in &tokenizer_files {
            match repo_obj.get(file) {
                Ok(_) => {
                    tracing::debug!("Found tokenizer file: {}", file);
                    if file == &"tokenizer.json" || file == &"vocab.txt" {
                        found_tokenizer = true;
                    }
                }
                Err(_) => {
                    tracing::debug!("File not found: {}", file);
                }
            }
        }

        if !found_tokenizer {
            return Err(checkstream_core::Error::classifier(
                "No tokenizer found (tried tokenizer.json, vocab.txt)",
            ));
        }

        // Get the cache path from hf-hub
        let config_path = repo_obj.get("config.json").map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to locate model cache: {}", e))
        })?;

        let model_dir = config_path
            .parent()
            .ok_or_else(|| checkstream_core::Error::classifier("Invalid cache path"))?;

        tracing::info!("Model available at: {}", model_dir.display());
        Ok(model_dir.to_path_buf())
    }

    /// Load BERT-based sequence classification model
    #[cfg(feature = "ml-models")]
    async fn load_bert_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;

        // Load tokenizer - try tokenizer.json first, then fall back to vocab.txt
        let tokenizer = load_tokenizer(&model_path)?;

        // Load BERT config
        let config_path = model_path.join("config.json");
        let bert_config: BertConfig = serde_json::from_str(
            &std::fs::read_to_string(&config_path)
                .map_err(|e| {
                    checkstream_core::Error::classifier(format!("Failed to read config: {}", e))
                })?,
        )
        .map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to parse config: {}", e))
        })?;

        // Determine device
        let device = get_device(&config.inference.device)?;

        // Load model weights
        let weights_path = model_path.join("model.safetensors");
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device).map_err(
                |e| checkstream_core::Error::classifier(format!("Failed to load weights: {}", e)),
            )?
        };

        // Load BERT encoder
        let model = BertModel::load(vb.pp("bert"), &bert_config).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to load BERT model: {}", e))
        })?;

        // Load classification head
        let classifier = load_classification_head(&vb, bert_config.hidden_size, num_labels)?;

        tracing::info!(
            "Successfully loaded BERT classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(BertSequenceClassifier {
            name: config.name.clone(),
            tokenizer,
            model,
            classifier,
            device,
            num_labels,
            labels: labels.to_vec(),
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    /// Load DistilBERT-based sequence classification model
    #[cfg(feature = "ml-models")]
    async fn load_distilbert_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;

        // Load tokenizer - try tokenizer.json first, then fall back to vocab.txt
        let tokenizer = load_tokenizer(&model_path)?;

        // Load DistilBERT config - parse as JSON Value first to get hidden size
        let config_path = model_path.join("config.json");
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to read config: {}", e))
            })?;

        // Get hidden_size from raw JSON (works around private field)
        let config_json: serde_json::Value = serde_json::from_str(&config_str)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to parse config JSON: {}", e))
            })?;

        let hidden_size = config_json.get("dim")
            .or_else(|| config_json.get("hidden_dim"))
            .or_else(|| config_json.get("hidden_size"))
            .and_then(|v| v.as_u64())
            .unwrap_or(768) as usize;

        let distilbert_config: DistilBertConfig = serde_json::from_str(&config_str)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to parse config: {}", e))
            })?;

        // Determine device
        let device = get_device(&config.inference.device)?;

        // Load model weights
        let weights_path = model_path.join("model.safetensors");
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device).map_err(
                |e| checkstream_core::Error::classifier(format!("Failed to load weights: {}", e)),
            )?
        };

        // Load DistilBERT encoder
        let model = DistilBertModel::load(vb.pp("distilbert"), &distilbert_config).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to load DistilBERT model: {}", e))
        })?;

        // Try to load pre_classifier (HuggingFace DistilBertForSequenceClassification has this)
        let pre_classifier = candle_nn::linear(hidden_size, hidden_size, vb.pp("pre_classifier")).ok();
        if pre_classifier.is_some() {
            tracing::info!("Loaded pre_classifier layer (hidden_size={})", hidden_size);
        }

        // Load classification head
        let classifier = load_classification_head(&vb, hidden_size, num_labels)?;

        tracing::info!(
            "Successfully loaded DistilBERT classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(DistilBertSequenceClassifier {
            name: config.name.clone(),
            tokenizer,
            model,
            pre_classifier,
            classifier,
            device,
            num_labels,
            labels: labels.to_vec(),
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    /// Stub for non-ml-models feature
    #[cfg(not(feature = "ml-models"))]
    async fn load_bert_classifier(
        &self,
        _config: &ModelConfig,
        _num_labels: usize,
        _labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        Err(checkstream_core::Error::classifier(
            "ML models require 'ml-models' feature flag",
        ))
    }

    /// Stub for non-ml-models feature
    #[cfg(not(feature = "ml-models"))]
    async fn load_distilbert_classifier(
        &self,
        _config: &ModelConfig,
        _num_labels: usize,
        _labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        Err(checkstream_core::Error::classifier(
            "ML models require 'ml-models' feature flag",
        ))
    }
}

/// Get device from string specification
#[cfg(feature = "ml-models")]
fn get_device(device_str: &str) -> Result<Device> {
    match device_str.to_lowercase().as_str() {
        "cuda" | "cuda:0" => Device::new_cuda(0).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to initialize CUDA: {}", e))
        }),
        "mps" | "metal" => Device::new_metal(0).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to initialize Metal: {}", e))
        }),
        _ => Ok(Device::Cpu),
    }
}

/// Load tokenizer from model directory
/// Tries tokenizer.json first, then falls back to building from vocab.txt
#[cfg(feature = "ml-models")]
fn load_tokenizer(model_path: &std::path::Path) -> Result<Tokenizer> {
    // Try tokenizer.json first (modern HuggingFace format)
    let tokenizer_json_path = model_path.join("tokenizer.json");
    if tokenizer_json_path.exists() {
        tracing::debug!("Loading tokenizer from tokenizer.json");
        return Tokenizer::from_file(&tokenizer_json_path).map_err(|e| {
            checkstream_core::Error::classifier(format!(
                "Failed to load tokenizer.json: {}",
                e
            ))
        });
    }

    // Fall back to vocab.txt (older format)
    let vocab_path = model_path.join("vocab.txt");
    if vocab_path.exists() {
        tracing::debug!("Building tokenizer from vocab.txt");

        // Build WordPiece tokenizer from vocab.txt
        use tokenizers::models::wordpiece::WordPiece;
        use tokenizers::normalizers::BertNormalizer;
        use tokenizers::pre_tokenizers::bert::BertPreTokenizer;
        use tokenizers::processors::bert::BertProcessing;

        let wordpiece = WordPiece::from_file(&vocab_path.to_string_lossy())
            .unk_token("[UNK]".to_string())
            .build()
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to build WordPiece model: {}",
                    e
                ))
            })?;

        let mut tokenizer = Tokenizer::new(wordpiece);
        tokenizer.with_normalizer(Some(BertNormalizer::default()));
        tokenizer.with_pre_tokenizer(Some(BertPreTokenizer));

        // Add BERT post-processor for [CLS] and [SEP] tokens
        let sep = ("[SEP]".to_string(), 102);
        let cls = ("[CLS]".to_string(), 101);
        tokenizer.with_post_processor(Some(BertProcessing::new(sep, cls)));

        return Ok(tokenizer);
    }

    Err(checkstream_core::Error::classifier(format!(
        "No tokenizer found in {:?} (tried tokenizer.json, vocab.txt)",
        model_path
    )))
}

/// Load the classification head (linear layer) from weights
#[cfg(feature = "ml-models")]
fn load_classification_head(
    vb: &VarBuilder,
    hidden_size: usize,
    num_labels: usize,
) -> Result<Linear> {
    // Try different naming conventions used by different models
    // Many HuggingFace models have the classifier at root level
    let naming_options = vec![
        "",                     // Root level (common for fine-tuned models)
        "classifier",           // Standard HuggingFace naming
        "cls.predictions",      // Some BERT variants
        "score",                // Some models use this
    ];

    for prefix in &naming_options {
        let vb_cls = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };

        // For root level, try "classifier" sub-prefix
        let vb_for_linear = if prefix.is_empty() {
            vb.pp("classifier")
        } else {
            vb_cls
        };

        // Try to load the linear layer
        if let Ok(linear) = candle_nn::linear(hidden_size, num_labels, vb_for_linear.clone()) {
            tracing::info!(
                "Loaded classification head from '{}' (hidden_size={}, num_labels={})",
                if prefix.is_empty() { "classifier" } else { prefix },
                hidden_size,
                num_labels
            );
            return Ok(linear);
        }
    }

    // If no pre-trained classifier head found, create a new one (for fine-tuning)
    tracing::warn!(
        "No pre-trained classification head found, initializing random weights. \
         Model should be fine-tuned before use."
    );

    // Create with random initialization
    let weight = Tensor::randn(0f32, 0.02, (num_labels, hidden_size), vb.device())
        .map_err(|e| checkstream_core::Error::classifier(format!("Failed to init weights: {}", e)))?;
    let bias = Tensor::zeros((num_labels,), DType::F32, vb.device())
        .map_err(|e| checkstream_core::Error::classifier(format!("Failed to init bias: {}", e)))?;

    Ok(Linear::new(weight, Some(bias)))
}

/// Apply softmax to get probabilities
#[cfg(feature = "ml-models")]
fn softmax(logits: &Tensor) -> Result<Tensor> {
    // Use candle's softmax which handles broadcasting correctly
    candle_nn::ops::softmax(logits, D::Minus1)
        .map_err(|e| checkstream_core::Error::classifier(format!("Softmax failed: {}", e)))
}

/// BERT-based sequence classifier
#[cfg(feature = "ml-models")]
struct BertSequenceClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: BertModel,
    classifier: Linear,
    device: Device,
    num_labels: usize,
    labels: Vec<String>,
    threshold: f32,
    max_length: usize,
}

#[cfg(feature = "ml-models")]
#[async_trait]
impl Classifier for BertSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input with truncation
        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;

        // Truncate if needed
        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = encoding.get_ids();
        let token_type_ids = encoding.get_type_ids();

        // Convert to tensors with batch dimension
        let input_ids = Tensor::new(input_ids, &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create input tensor: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        let token_type_ids = Tensor::new(token_type_ids, &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create token type tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        // Run BERT forward pass to get hidden states
        let hidden_states = self
            .model
            .forward(&input_ids, &token_type_ids, None)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        // Get [CLS] token embedding (first token of sequence)
        // hidden_states shape: [batch_size, seq_len, hidden_size]
        let cls_embedding = hidden_states
            .i((0, 0, ..))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to get CLS token: {}", e))
            })?
            .unsqueeze(0)  // Make it [1, hidden_size] for linear layer
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze CLS: {}", e))
            })?;

        // Apply classification head: [1, hidden_size] -> [1, num_labels]
        let logits = self.classifier.forward(&cls_embedding).map_err(|e| {
            checkstream_core::Error::classifier(format!("Classification head failed: {}", e))
        })?;

        // Apply softmax to get probabilities (logits is already [1, num_labels])
        let probs = softmax(&logits)?;

        // Convert to vec for processing
        let probs_vec: Vec<f32> = probs
            .squeeze(0)
            .map_err(|e| checkstream_core::Error::classifier(format!("Squeeze failed: {}", e)))?
            .to_vec1()
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to convert to vec: {}", e))
            })?;

        // Find the label with highest probability
        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("label_{}", max_idx));

        // For binary classification, return probability of positive class
        // For multi-class, return max probability
        let score = if self.num_labels == 2 {
            // Binary: return probability of label index 1 (typically "positive" or "toxic")
            probs_vec.get(1).copied().unwrap_or(*max_prob)
        } else {
            *max_prob
        };

        // Determine if above threshold
        let is_positive = score > self.threshold;
        let final_label = if is_positive {
            label
        } else if self.num_labels == 2 {
            self.labels.get(0).cloned().unwrap_or_else(|| "negative".to_string())
        } else {
            label
        };

        Ok(ClassificationResult {
            label: final_label,
            score,
            metadata: ClassificationMetadata {
                model: Some(self.name.clone()),
                all_scores: Some(
                    self.labels
                        .iter()
                        .zip(probs_vec.iter())
                        .map(|(l, s)| (l.clone(), *s))
                        .collect(),
                ),
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B // ML models are Tier B (< 5ms target)
    }
}

/// DistilBERT-based sequence classifier
#[cfg(feature = "ml-models")]
struct DistilBertSequenceClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: DistilBertModel,
    pre_classifier: Option<Linear>,
    classifier: Linear,
    device: Device,
    num_labels: usize,
    labels: Vec<String>,
    threshold: f32,
    max_length: usize,
}

#[cfg(feature = "ml-models")]
#[async_trait]
impl Classifier for DistilBertSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input with truncation
        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;

        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        // Convert input_ids to i64 tensor (transformer models expect Long type)
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let input_ids = Tensor::new(input_ids_i64.as_slice(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create input tensor: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        // Candle's DistilBERT uses INVERTED mask semantics:
        // - 0 = attend to this position
        // - 1 = mask out (fill with NEG_INFINITY)
        // HuggingFace's attention_mask uses: 1 = attend, 0 = mask
        // So we need to invert: new_mask = 1 - old_mask
        let attention_mask_inverted: Vec<u8> = attention_mask.iter().map(|&x| if x == 0 { 1u8 } else { 0u8 }).collect();
        let attention_mask = Tensor::new(attention_mask_inverted.as_slice(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create attention mask: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        // DistilBERT forward pass
        let hidden_states = self.model.forward(&input_ids, &attention_mask).map_err(|e| {
            checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
        })?;

        // Get [CLS] token embedding (first token)
        let cls_embedding = hidden_states
            .i((0, 0, ..))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to get CLS token: {}", e))
            })?
            .unsqueeze(0)  // Make it [1, hidden_size] for linear layer
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze CLS: {}", e))
            })?;

        // Apply pre_classifier + ReLU if present (HuggingFace DistilBertForSequenceClassification)
        let pooled_output = if let Some(ref pre_classifier) = self.pre_classifier {
            let pre_out = pre_classifier.forward(&cls_embedding).map_err(|e| {
                checkstream_core::Error::classifier(format!("Pre-classifier failed: {}", e))
            })?;
            // Apply ReLU activation
            pre_out.relu().map_err(|e| {
                checkstream_core::Error::classifier(format!("ReLU failed: {}", e))
            })?
        } else {
            cls_embedding
        };

        // Apply classification head: [1, hidden_size] -> [1, num_labels]
        let logits = self.classifier.forward(&pooled_output).map_err(|e| {
            checkstream_core::Error::classifier(format!("Classification head failed: {}", e))
        })?;

        // Apply softmax to get probabilities
        let probs = softmax(&logits)?;

        let probs_vec: Vec<f32> = probs
            .squeeze(0)
            .map_err(|e| checkstream_core::Error::classifier(format!("Squeeze failed: {}", e)))?
            .to_vec1()
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to convert to vec: {}", e))
            })?;

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("label_{}", max_idx));

        let score = if self.num_labels == 2 {
            probs_vec.get(1).copied().unwrap_or(*max_prob)
        } else {
            *max_prob
        };

        let is_positive = score > self.threshold;
        let final_label = if is_positive {
            label
        } else if self.num_labels == 2 {
            self.labels.get(0).cloned().unwrap_or_else(|| "negative".to_string())
        } else {
            label
        };

        Ok(ClassificationResult {
            label: final_label,
            score,
            metadata: ClassificationMetadata {
                model: Some(self.name.clone()),
                all_scores: Some(
                    self.labels
                        .iter()
                        .zip(probs_vec.iter())
                        .map(|(l, s)| (l.clone(), *s))
                        .collect(),
                ),
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
