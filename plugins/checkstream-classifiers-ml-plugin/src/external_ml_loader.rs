use async_trait::async_trait;
use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::{Linear, Module, VarBuilder};
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use candle_transformers::models::debertav2::{
    Config as DebertaV2Config, DebertaV2SeqClassificationModel, Id2Label as DebertaId2Label,
};
use candle_transformers::models::distilbert::{Config as DistilBertConfig, DistilBertModel};
use candle_transformers::models::xlm_roberta::{
    Config as XlmRobertaConfig, XLMRobertaForSequenceClassification,
};
use checkstream_classifiers::classifier::{ClassificationMetadata, Classifier};
use checkstream_classifiers::loader_plugin::ModelLoaderPlugin;
use checkstream_classifiers::model_config::{
    ArchitectureConfig, ModelConfig, ModelRegistry, ModelSource,
};
use checkstream_classifiers::{ClassificationResult, ClassifierTier};
use checkstream_core::Result;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokenizers::{Tokenizer, TruncationDirection};

/// Candle/HuggingFace-backed external model loader plugin.
pub struct ExternalMlModelLoader {
    registry: Arc<ModelRegistry>,
    _cache_dir: PathBuf,
}

impl ExternalMlModelLoader {
    /// Create plugin from a parsed model registry.
    pub fn from_registry(registry: ModelRegistry) -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache/checkstream/models");

        std::fs::create_dir_all(&cache_dir).ok();

        Self {
            registry: Arc::new(registry),
            _cache_dir: cache_dir,
        }
    }

    /// Create plugin by loading a model-registry file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let registry = ModelRegistry::from_file(path).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to load model registry: {}", e))
        })?;
        Ok(Self::from_registry(registry))
    }

    /// Access the underlying model registry.
    pub fn registry(&self) -> &Arc<ModelRegistry> {
        &self.registry
    }

    async fn load_classifier_internal(&self, name: &str) -> Result<Box<dyn Classifier>> {
        let config = self.registry.get_model(name).ok_or_else(|| {
            checkstream_core::Error::classifier(format!("Model '{}' not found in registry", name))
        })?;

        tracing::info!("Loading external ML model '{}'", name);

        match &config.architecture {
            ArchitectureConfig::BertSequenceClassification { num_labels, labels }
            | ArchitectureConfig::RobertaSequenceClassification { num_labels, labels }
            | ArchitectureConfig::MiniLmSequenceClassification { num_labels, labels } => {
                self.load_bert_classifier(config, *num_labels, labels).await
            }
            ArchitectureConfig::DistilBertSequenceClassification { num_labels, labels } => {
                self.load_distilbert_classifier(config, *num_labels, labels)
                    .await
            }
            ArchitectureConfig::DebertaSequenceClassification { num_labels, labels } => {
                self.load_deberta_classifier(config, *num_labels, labels)
                    .await
            }
            ArchitectureConfig::XlmRobertaSequenceClassification { num_labels, labels } => {
                self.load_xlm_roberta_classifier(config, *num_labels, labels)
                    .await
            }
            ArchitectureConfig::SentenceTransformer { pooling } => {
                self.load_sentence_transformer_classifier(config, pooling)
                    .await
            }
            ArchitectureConfig::Custom { implementation } => {
                Err(checkstream_core::Error::classifier(format!(
                    "Custom architecture '{}' requires a custom plugin implementation",
                    implementation
                )))
            }
        }
    }

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
            ModelSource::Builtin { implementation } => {
                Err(checkstream_core::Error::classifier(format!(
                    "Builtin implementation '{}' should not use external ML loader",
                    implementation
                )))
            }
        }
    }

    async fn download_from_huggingface(&self, repo: &str, _revision: &str) -> Result<PathBuf> {
        tracing::info!("Downloading model from HuggingFace: {}", repo);

        let api = hf_hub::api::sync::Api::new().map_err(|e| {
            checkstream_core::Error::classifier(format!(
                "Failed to initialize HuggingFace API: {}",
                e
            ))
        })?;

        let repo_obj = api.repo(hf_hub::Repo::model(repo.to_string()));

        let weight_files = ["model.safetensors", "pytorch_model.bin"];
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

        tracing::debug!("Downloading config.json");
        repo_obj.get("config.json").map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to download config.json: {}", e))
        })?;

        let tokenizer_files = [
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

        let config_path = repo_obj.get("config.json").map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to locate model cache: {}", e))
        })?;

        let model_dir = config_path
            .parent()
            .ok_or_else(|| checkstream_core::Error::classifier("Invalid cache path"))?;

        tracing::info!("Model available at: {}", model_dir.display());
        Ok(model_dir.to_path_buf())
    }

    async fn load_bert_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;
        let tokenizer = load_tokenizer(&model_path)?;
        let bert_config: BertConfig = parse_json_config(&model_path.join("config.json"))?;

        let device = get_device(&config.inference.device)?;
        let vb = load_var_builder(&model_path, &device)?;

        let model = load_bert_backbone(&vb, &bert_config, &["bert", "roberta", ""])?;
        let classifier = load_classification_head(&vb, bert_config.hidden_size, num_labels)?;
        let labels = normalized_labels(num_labels, labels);

        tracing::info!(
            "Successfully loaded BERT-family classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(BertSequenceClassifier {
            name: resolved_name(config),
            tokenizer,
            model,
            classifier,
            device,
            num_labels,
            labels,
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    async fn load_distilbert_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;
        let tokenizer = load_tokenizer(&model_path)?;

        let config_path = model_path.join("config.json");
        let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to read config: {}", e))
        })?;

        let config_json: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to parse config JSON: {}", e))
        })?;

        let hidden_size = config_json
            .get("dim")
            .or_else(|| config_json.get("hidden_dim"))
            .or_else(|| config_json.get("hidden_size"))
            .and_then(|v| v.as_u64())
            .unwrap_or(768) as usize;

        let distilbert_config: DistilBertConfig =
            serde_json::from_str(&config_str).map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to parse config: {}", e))
            })?;

        let device = get_device(&config.inference.device)?;
        let vb = load_var_builder(&model_path, &device)?;

        let model =
            DistilBertModel::load(vb.pp("distilbert"), &distilbert_config).map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to load DistilBERT model: {}",
                    e
                ))
            })?;

        let pre_classifier =
            candle_nn::linear(hidden_size, hidden_size, vb.pp("pre_classifier")).ok();
        if pre_classifier.is_some() {
            tracing::info!("Loaded pre_classifier layer (hidden_size={})", hidden_size);
        }

        let classifier = load_classification_head(&vb, hidden_size, num_labels)?;
        let labels = normalized_labels(num_labels, labels);

        tracing::info!(
            "Successfully loaded DistilBERT classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(DistilBertSequenceClassifier {
            name: resolved_name(config),
            tokenizer,
            model,
            pre_classifier,
            classifier,
            device,
            num_labels,
            labels,
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    async fn load_deberta_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;
        let tokenizer = load_tokenizer(&model_path)?;
        let deberta_config: DebertaV2Config = parse_json_config(&model_path.join("config.json"))?;

        let device = get_device(&config.inference.device)?;
        let vb = load_var_builder(&model_path, &device)?;

        let labels = normalized_labels(num_labels, labels);
        let id2label: DebertaId2Label = labels
            .iter()
            .enumerate()
            .map(|(idx, label)| (idx as u32, label.clone()))
            .collect();

        let model = load_deberta_sequence_model(&vb, &deberta_config, id2label)?;

        tracing::info!(
            "Successfully loaded DeBERTa classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(DebertaSequenceClassifier {
            name: resolved_name(config),
            tokenizer,
            model,
            device,
            num_labels,
            labels,
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    async fn load_xlm_roberta_classifier(
        &self,
        config: &ModelConfig,
        num_labels: usize,
        labels: &[String],
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;
        let tokenizer = load_tokenizer(&model_path)?;
        let xlm_config: XlmRobertaConfig = parse_json_config(&model_path.join("config.json"))?;

        let device = get_device(&config.inference.device)?;
        let vb = load_var_builder(&model_path, &device)?;
        let model = load_xlm_roberta_sequence_model(&vb, num_labels, &xlm_config)?;
        let labels = normalized_labels(num_labels, labels);

        tracing::info!(
            "Successfully loaded XLM-RoBERTa classifier with {} labels: {:?}",
            num_labels,
            labels
        );

        Ok(Box::new(XlmRobertaSequenceClassifier {
            name: resolved_name(config),
            tokenizer,
            model,
            device,
            num_labels,
            labels,
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }

    async fn load_sentence_transformer_classifier(
        &self,
        config: &ModelConfig,
        pooling: &str,
    ) -> Result<Box<dyn Classifier>> {
        let model_path = self.resolve_model_path(config).await?;
        let tokenizer = load_tokenizer(&model_path)?;
        let bert_config: BertConfig = parse_json_config(&model_path.join("config.json"))?;

        let device = get_device(&config.inference.device)?;
        let vb = load_var_builder(&model_path, &device)?;
        let model = load_bert_backbone(&vb, &bert_config, &["bert", "roberta", ""])?;

        let pooling = PoolingStrategy::from_config(pooling);

        tracing::info!(
            "Successfully loaded sentence-transformer model with pooling='{}'",
            pooling.as_str()
        );

        Ok(Box::new(SentenceTransformerClassifier {
            name: resolved_name(config),
            tokenizer,
            model,
            device,
            pooling,
            threshold: config.inference.threshold,
            max_length: config.inference.max_length,
        }))
    }
}

#[async_trait]
impl ModelLoaderPlugin for ExternalMlModelLoader {
    async fn load_classifier(&self, name: &str) -> Result<Box<dyn Classifier>> {
        self.load_classifier_internal(name).await
    }

    fn available_models(&self) -> Vec<String> {
        self.registry.models.keys().cloned().collect()
    }
}

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

fn resolved_name(config: &ModelConfig) -> String {
    if config.name.is_empty() {
        "external-ml-model".to_string()
    } else {
        config.name.clone()
    }
}

fn parse_json_config<T: DeserializeOwned>(config_path: &Path) -> Result<T> {
    let config_str = std::fs::read_to_string(config_path).map_err(|e| {
        checkstream_core::Error::classifier(format!(
            "Failed to read config {}: {}",
            config_path.display(),
            e
        ))
    })?;

    serde_json::from_str(&config_str).map_err(|e| {
        checkstream_core::Error::classifier(format!(
            "Failed to parse config {}: {}",
            config_path.display(),
            e
        ))
    })
}

fn load_var_builder(model_path: &Path, device: &Device) -> Result<VarBuilder<'static>> {
    let weights_path = model_path.join("model.safetensors");
    if !weights_path.exists() {
        return Err(checkstream_core::Error::classifier(format!(
            "model.safetensors not found in {}",
            model_path.display()
        )));
    }

    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, device).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to load weights: {}", e))
        })?
    };

    Ok(vb)
}

fn load_bert_backbone(
    vb: &VarBuilder,
    config: &BertConfig,
    prefixes: &[&str],
) -> Result<BertModel> {
    let mut errors = Vec::new();

    for prefix in prefixes {
        let vb_prefix = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };

        match BertModel::load(vb_prefix, config) {
            Ok(model) => {
                let effective_prefix = if prefix.is_empty() { "<root>" } else { prefix };
                tracing::info!("Loaded BERT backbone from '{}'", effective_prefix);
                return Ok(model);
            }
            Err(e) => {
                errors.push(format!(
                    "{}: {}",
                    if prefix.is_empty() { "<root>" } else { prefix },
                    e
                ));
            }
        }
    }

    Err(checkstream_core::Error::classifier(format!(
        "Failed to load BERT backbone with tried prefixes [{}]",
        errors.join(" | ")
    )))
}

fn load_deberta_sequence_model(
    vb: &VarBuilder,
    config: &DebertaV2Config,
    id2label: DebertaId2Label,
) -> Result<DebertaV2SeqClassificationModel> {
    let mut errors = Vec::new();

    for prefix in ["deberta", ""] {
        let vb_prefix = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };

        match DebertaV2SeqClassificationModel::load(vb_prefix, config, Some(id2label.clone())) {
            Ok(model) => {
                let effective_prefix = if prefix.is_empty() { "<root>" } else { prefix };
                tracing::info!("Loaded DeBERTa backbone from '{}'", effective_prefix);
                return Ok(model);
            }
            Err(e) => {
                errors.push(format!(
                    "{}: {}",
                    if prefix.is_empty() { "<root>" } else { prefix },
                    e
                ));
            }
        }
    }

    Err(checkstream_core::Error::classifier(format!(
        "Failed to load DeBERTa sequence model with tried prefixes [{}]",
        errors.join(" | ")
    )))
}

fn load_xlm_roberta_sequence_model(
    vb: &VarBuilder,
    num_labels: usize,
    config: &XlmRobertaConfig,
) -> Result<XLMRobertaForSequenceClassification> {
    let mut errors = Vec::new();

    for prefix in ["", "model"] {
        let vb_prefix = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };

        match XLMRobertaForSequenceClassification::new(num_labels, config, vb_prefix) {
            Ok(model) => {
                let effective_prefix = if prefix.is_empty() { "<root>" } else { prefix };
                tracing::info!("Loaded XLM-RoBERTa backbone from '{}'", effective_prefix);
                return Ok(model);
            }
            Err(e) => {
                errors.push(format!(
                    "{}: {}",
                    if prefix.is_empty() { "<root>" } else { prefix },
                    e
                ));
            }
        }
    }

    Err(checkstream_core::Error::classifier(format!(
        "Failed to load XLM-RoBERTa sequence model with tried prefixes [{}]",
        errors.join(" | ")
    )))
}

fn load_tokenizer(model_path: &Path) -> Result<Tokenizer> {
    let tokenizer_json_path = model_path.join("tokenizer.json");
    if tokenizer_json_path.exists() {
        tracing::debug!("Loading tokenizer from tokenizer.json");
        return Tokenizer::from_file(&tokenizer_json_path).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to load tokenizer.json: {}", e))
        });
    }

    let vocab_path = model_path.join("vocab.txt");
    if vocab_path.exists() {
        tracing::debug!("Building tokenizer from vocab.txt");

        use tokenizers::models::wordpiece::WordPiece;
        use tokenizers::normalizers::BertNormalizer;
        use tokenizers::pre_tokenizers::bert::BertPreTokenizer;
        use tokenizers::processors::bert::BertProcessing;

        let wordpiece = WordPiece::from_file(vocab_path.to_string_lossy().as_ref())
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

fn load_classification_head(
    vb: &VarBuilder,
    hidden_size: usize,
    num_labels: usize,
) -> Result<Linear> {
    let naming_options = ["", "classifier", "cls.predictions", "score"];

    for prefix in &naming_options {
        let vb_cls = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };

        let vb_for_linear = if prefix.is_empty() {
            vb.pp("classifier")
        } else {
            vb_cls
        };

        if let Ok(linear) = candle_nn::linear(hidden_size, num_labels, vb_for_linear.clone()) {
            tracing::info!(
                "Loaded classification head from '{}' (hidden_size={}, num_labels={})",
                if prefix.is_empty() {
                    "classifier"
                } else {
                    prefix
                },
                hidden_size,
                num_labels
            );
            return Ok(linear);
        }
    }

    tracing::warn!(
        "No pre-trained classification head found, initializing random weights. \
         Model should be fine-tuned before use."
    );

    let weight =
        Tensor::randn(0f32, 0.02, (num_labels, hidden_size), vb.device()).map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to init weights: {}", e))
        })?;
    let bias = Tensor::zeros((num_labels,), DType::F32, vb.device())
        .map_err(|e| checkstream_core::Error::classifier(format!("Failed to init bias: {}", e)))?;

    Ok(Linear::new(weight, Some(bias)))
}

fn softmax(logits: &Tensor) -> Result<Tensor> {
    candle_nn::ops::softmax(logits, D::Minus1)
        .map_err(|e| checkstream_core::Error::classifier(format!("Softmax failed: {}", e)))
}

fn normalized_labels(num_labels: usize, labels: &[String]) -> Vec<String> {
    if labels.is_empty() {
        return match num_labels {
            0 | 2 => vec!["negative".to_string(), "positive".to_string()],
            1 => vec!["positive".to_string()],
            n => (0..n).map(|idx| format!("label_{}", idx)).collect(),
        };
    }

    let mut resolved = labels.to_vec();
    if resolved.len() < num_labels {
        for idx in resolved.len()..num_labels {
            resolved.push(format!("label_{}", idx));
        }
    }
    resolved
}

fn to_probabilities(logits: &Tensor) -> Result<Vec<f32>> {
    softmax(logits)?
        .squeeze(0)
        .map_err(|e| checkstream_core::Error::classifier(format!("Squeeze failed: {}", e)))?
        .to_vec1()
        .map_err(|e| {
            checkstream_core::Error::classifier(format!("Failed to convert to vec: {}", e))
        })
}

fn build_sequence_result(
    name: &str,
    labels: &[String],
    num_labels: usize,
    threshold: f32,
    probs_vec: Vec<f32>,
    start: Instant,
) -> ClassificationResult {
    let (max_idx, max_prob) = probs_vec
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));

    let predicted_label = labels
        .get(max_idx)
        .cloned()
        .unwrap_or_else(|| format!("label_{}", max_idx));

    let score = if num_labels == 2 {
        probs_vec.get(1).copied().unwrap_or(*max_prob)
    } else {
        *max_prob
    };

    let final_label = if num_labels == 2 && score <= threshold {
        labels
            .first()
            .cloned()
            .unwrap_or_else(|| "negative".to_string())
    } else {
        predicted_label
    };

    let all_scores = labels
        .iter()
        .enumerate()
        .map(|(idx, label)| (label.clone(), probs_vec.get(idx).copied().unwrap_or(0.0)))
        .collect();

    ClassificationResult {
        label: final_label,
        score,
        metadata: ClassificationMetadata {
            model: Some(name.to_string()),
            all_scores: Some(all_scores),
            ..Default::default()
        },
        latency_us: start.elapsed().as_micros() as u64,
    }
}

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

#[async_trait]
impl Classifier for BertSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;

        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create input tensor: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        let token_type_ids = Tensor::new(encoding.get_type_ids(), &self.device)
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

        let hidden_states = self
            .model
            .forward(&input_ids, &token_type_ids, None)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        let cls_embedding = hidden_states
            .i((0, 0, ..))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to get CLS token: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze CLS: {}", e))
            })?;

        let logits = self.classifier.forward(&cls_embedding).map_err(|e| {
            checkstream_core::Error::classifier(format!("Classification head failed: {}", e))
        })?;

        let probs_vec = to_probabilities(&logits)?;

        Ok(build_sequence_result(
            &self.name,
            &self.labels,
            self.num_labels,
            self.threshold,
            probs_vec,
            start,
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}

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

#[async_trait]
impl Classifier for DistilBertSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;

        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids_i64: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let input_ids = Tensor::new(input_ids_i64.as_slice(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create input tensor: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        let attention_mask_inverted: Vec<u8> = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| if x == 0 { 1u8 } else { 0u8 })
            .collect();
        let attention_mask = Tensor::new(attention_mask_inverted.as_slice(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create attention mask: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze: {}", e))
            })?;

        let hidden_states = self
            .model
            .forward(&input_ids, &attention_mask)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        let cls_embedding = hidden_states
            .i((0, 0, ..))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to get CLS token: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze CLS: {}", e))
            })?;

        let pooled_output = if let Some(ref pre_classifier) = self.pre_classifier {
            let pre_out = pre_classifier.forward(&cls_embedding).map_err(|e| {
                checkstream_core::Error::classifier(format!("Pre-classifier failed: {}", e))
            })?;
            pre_out
                .relu()
                .map_err(|e| checkstream_core::Error::classifier(format!("ReLU failed: {}", e)))?
        } else {
            cls_embedding
        };

        let logits = self.classifier.forward(&pooled_output).map_err(|e| {
            checkstream_core::Error::classifier(format!("Classification head failed: {}", e))
        })?;

        let probs_vec = to_probabilities(&logits)?;

        Ok(build_sequence_result(
            &self.name,
            &self.labels,
            self.num_labels,
            self.threshold,
            probs_vec,
            start,
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}

struct DebertaSequenceClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: DebertaV2SeqClassificationModel,
    device: Device,
    num_labels: usize,
    labels: Vec<String>,
    threshold: f32,
    max_length: usize,
}

#[async_trait]
impl Classifier for DebertaSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;
        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to create input tensor: {}", e))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze input ids: {}", e))
            })?;

        let token_type_ids = Tensor::new(encoding.get_type_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create token type tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze token types: {}",
                    e
                ))
            })?;

        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create attention mask tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze attention mask: {}",
                    e
                ))
            })?;

        let logits = self
            .model
            .forward(&input_ids, Some(token_type_ids), Some(attention_mask))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        let probs_vec = to_probabilities(&logits)?;

        Ok(build_sequence_result(
            &self.name,
            &self.labels,
            self.num_labels,
            self.threshold,
            probs_vec,
            start,
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::C
    }
}

struct XlmRobertaSequenceClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: XLMRobertaForSequenceClassification,
    device: Device,
    num_labels: usize,
    labels: Vec<String>,
    threshold: f32,
    max_length: usize,
}

#[async_trait]
impl Classifier for XlmRobertaSequenceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;
        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create input ids tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze input ids: {}", e))
            })?;

        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create attention mask tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze attention mask: {}",
                    e
                ))
            })?;

        let token_type_ids = Tensor::new(encoding.get_type_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create token type ids tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze token type ids: {}",
                    e
                ))
            })?;

        let logits = self
            .model
            .forward(&input_ids, &attention_mask, &token_type_ids)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        let probs_vec = to_probabilities(&logits)?;

        Ok(build_sequence_result(
            &self.name,
            &self.labels,
            self.num_labels,
            self.threshold,
            probs_vec,
            start,
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::C
    }
}

#[derive(Clone, Copy)]
enum PoolingStrategy {
    Mean,
    Cls,
}

impl PoolingStrategy {
    fn from_config(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "cls" => Self::Cls,
            _ => Self::Mean,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Mean => "mean",
            Self::Cls => "cls",
        }
    }
}

struct SentenceTransformerClassifier {
    name: String,
    tokenizer: Tokenizer,
    model: BertModel,
    device: Device,
    pooling: PoolingStrategy,
    threshold: f32,
    max_length: usize,
}

#[async_trait]
impl Classifier for SentenceTransformerClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut encoding = self.tokenizer.encode(text, true).map_err(|e| {
            checkstream_core::Error::classifier(format!("Tokenization failed: {}", e))
        })?;
        encoding.truncate(self.max_length, 0, TruncationDirection::Right);

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create input ids tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Failed to unsqueeze input ids: {}", e))
            })?;

        let token_type_ids = Tensor::new(encoding.get_type_ids(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create token type ids tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze token type ids: {}",
                    e
                ))
            })?;

        let attention_mask_vec: Vec<u32> = encoding.get_attention_mask().to_vec();
        let attention_mask = Tensor::new(attention_mask_vec.as_slice(), &self.device)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to create attention mask tensor: {}",
                    e
                ))
            })?
            .unsqueeze(0)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to unsqueeze attention mask: {}",
                    e
                ))
            })?;

        let hidden_states = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| {
                checkstream_core::Error::classifier(format!("Model forward pass failed: {}", e))
            })?;

        let pooled_embedding = match self.pooling {
            PoolingStrategy::Cls => hidden_states
                .i((0, 0, ..))
                .map_err(|e| {
                    checkstream_core::Error::classifier(format!(
                        "Failed to extract CLS token: {}",
                        e
                    ))
                })?
                .to_vec1()
                .map_err(|e| {
                    checkstream_core::Error::classifier(format!(
                        "Failed to convert CLS embedding to vector: {}",
                        e
                    ))
                })?,
            PoolingStrategy::Mean => {
                let sequence_embeddings: Vec<Vec<f32>> = hidden_states
                    .squeeze(0)
                    .map_err(|e| {
                        checkstream_core::Error::classifier(format!(
                            "Failed to squeeze embedding tensor: {}",
                            e
                        ))
                    })?
                    .to_vec2()
                    .map_err(|e| {
                        checkstream_core::Error::classifier(format!(
                            "Failed to convert embeddings to matrix: {}",
                            e
                        ))
                    })?;

                mean_pool_embeddings(&sequence_embeddings, &attention_mask_vec)
            }
        };

        let raw_score = pooled_embedding
            .first()
            .copied()
            .unwrap_or(0.0)
            .clamp(-8.0, 8.0);
        let score = 1.0f32 / (1.0f32 + (-raw_score).exp());
        let label = if score > self.threshold {
            "positive"
        } else {
            "negative"
        };

        let norm = pooled_embedding.iter().map(|v| v * v).sum::<f32>().sqrt();

        Ok(ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata {
                model: Some(self.name.clone()),
                all_scores: Some(vec![
                    ("negative".to_string(), 1.0 - score),
                    ("positive".to_string(), score),
                ]),
                extra: vec![
                    ("pooling".to_string(), self.pooling.as_str().to_string()),
                    (
                        "embedding_dim".to_string(),
                        pooled_embedding.len().to_string(),
                    ),
                    ("embedding_norm".to_string(), format!("{:.6}", norm)),
                ],
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::C
    }
}

fn mean_pool_embeddings(sequence_embeddings: &[Vec<f32>], attention_mask: &[u32]) -> Vec<f32> {
    if sequence_embeddings.is_empty() {
        return Vec::new();
    }

    let hidden_dim = sequence_embeddings[0].len();
    let mut pooled = vec![0.0f32; hidden_dim];
    let mut token_count = 0.0f32;

    for (idx, embedding) in sequence_embeddings.iter().enumerate() {
        let include_token = attention_mask.get(idx).copied().unwrap_or(0) > 0;
        if !include_token {
            continue;
        }

        token_count += 1.0;
        for (j, value) in embedding.iter().enumerate() {
            pooled[j] += value;
        }
    }

    if token_count == 0.0 {
        token_count = sequence_embeddings.len() as f32;
        for embedding in sequence_embeddings {
            for (j, value) in embedding.iter().enumerate() {
                pooled[j] += value;
            }
        }
    }

    for value in &mut pooled {
        *value /= token_count;
    }

    pooled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loader_registry_parsing() {
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
        let loader = ExternalMlModelLoader::from_registry(registry);

        let _ = loader.load_classifier("test-model").await;
    }

    #[test]
    fn test_mean_pool_embeddings_uses_mask() {
        let sequence_embeddings = vec![vec![2.0, 2.0], vec![6.0, 10.0], vec![100.0, 100.0]];
        let attention_mask = vec![1, 1, 0];

        let pooled = mean_pool_embeddings(&sequence_embeddings, &attention_mask);

        assert_eq!(pooled.len(), 2);
        assert!((pooled[0] - 4.0).abs() < 0.0001);
        assert!((pooled[1] - 6.0).abs() < 0.0001);
    }
}
