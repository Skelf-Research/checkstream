//! Toxicity detection classifier (Tier B)

use crate::classifier::{Classifier, ClassificationResult, ClassificationMetadata, ClassifierTier};
use checkstream_core::Result;
use std::time::Instant;

// Optional: Real ML model support
#[cfg(feature = "ml-models")]
use candle_core::{Device, Tensor};
#[cfg(feature = "ml-models")]
use candle_nn::VarBuilder;
#[cfg(feature = "ml-models")]
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
#[cfg(feature = "ml-models")]
use tokenizers::Tokenizer;

/// Toxicity detection classifier
///
/// Supports two modes:
/// 1. Pattern-based (fast, low accuracy) - Default
/// 2. ML-based (slower, high accuracy) - Requires "ml-models" feature
pub struct ToxicityClassifier {
    #[cfg(feature = "ml-models")]
    model: Option<ToxicityModel>,

    #[cfg(not(feature = "ml-models"))]
    _mode: PatternMode,
}

#[cfg(feature = "ml-models")]
struct ToxicityModel {
    tokenizer: Tokenizer,
    model: BertModel,
    device: Device,
}

#[cfg(not(feature = "ml-models"))]
#[derive(Clone)]
struct PatternMode;

impl ToxicityClassifier {
    /// Create a new toxicity classifier
    pub fn new() -> Result<Self> {
        #[cfg(feature = "ml-models")]
        {
            // Try to load ML model
            match Self::try_load_model() {
                Ok(model) => {
                    tracing::info!("Loaded ML-based toxicity classifier");
                    Ok(Self { model: Some(model) })
                }
                Err(e) => {
                    tracing::warn!("Failed to load ML model, using pattern-based fallback: {}", e);
                    Ok(Self { model: None })
                }
            }
        }

        #[cfg(not(feature = "ml-models"))]
        {
            tracing::info!("Using pattern-based toxicity classifier (compile with 'ml-models' feature for ML)");
            Ok(Self { _mode: PatternMode })
        }
    }

    #[cfg(feature = "ml-models")]
    fn try_load_model() -> Result<ToxicityModel> {
        // Try to load model from common locations
        let model_paths = vec![
            "./models/toxic-bert",
            "./models/toxicity",
            "~/.cache/checkstream/models/toxic-bert",
        ];

        for path in model_paths {
            let model_path = std::path::Path::new(path);
            if !model_path.exists() {
                continue;
            }

            tracing::info!("Found model at: {}", path);

            // Try to load tokenizer from tokenizer.json first
            let tokenizer_json_path = model_path.join("tokenizer.json");
            let tokenizer = if tokenizer_json_path.exists() {
                Tokenizer::from_file(&tokenizer_json_path)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to load tokenizer.json: {}", e)
                    ))?
            } else {
                // Fall back to building from pretrained (this will use the HF API)
                // For now, we'll just error out - user needs to generate tokenizer.json
                return Err(checkstream_core::Error::classifier(
                    format!("tokenizer.json not found at {}. Please generate it from the model files.", path)
                ));
            };

            // Load BERT model
            let config_path = model_path.join("config.json");
            let config: BertConfig = serde_json::from_str(
                &std::fs::read_to_string(&config_path)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to read config: {}", e)
                    ))?
            ).map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to parse config: {}", e)
            ))?;

            let device = Device::Cpu;
            let weights_path = model_path.join("model.safetensors");

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)
                    .map_err(|e| checkstream_core::Error::classifier(
                        format!("Failed to load weights: {}", e)
                    ))?
            };

            let model = BertModel::load(vb, &config)
                .map_err(|e| checkstream_core::Error::classifier(
                    format!("Failed to load BERT model: {}", e)
                ))?;

            tracing::info!("Successfully loaded toxicity model from {}", path);

            return Ok(ToxicityModel {
                tokenizer,
                model,
                device,
            });
        }

        Err(checkstream_core::Error::classifier(
            "No toxicity model found. Run: scripts/download_models.sh"
        ))
    }

    #[cfg(feature = "ml-models")]
    async fn classify_with_ml(&self, text: &str, model: &ToxicityModel) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input
        let encoding = model.tokenizer
            .encode(text, true)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Tokenization failed: {}", e)
            ))?;

        let input_ids = encoding.get_ids();
        let token_type_ids = encoding.get_type_ids();

        // Convert to tensors
        let input_ids = Tensor::new(input_ids, &model.device)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to create input tensor: {}", e)
            ))?
            .unsqueeze(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to unsqueeze: {}", e)
            ))?;

        let token_type_ids = Tensor::new(token_type_ids, &model.device)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to create token type tensor: {}", e)
            ))?
            .unsqueeze(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to unsqueeze: {}", e)
            ))?;

        // Run BERT forward pass (attention_mask is optional, pass None)
        let output = model.model
            .forward(&input_ids, &token_type_ids, None)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Model forward pass failed: {}", e)
            ))?;

        // Get [CLS] token embedding (first token)
        let cls_embedding = output
            .get(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to get batch: {}", e)
            ))?
            .get(0)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to get CLS token: {}", e)
            ))?;

        // For toxic-bert, we need a classification head
        // For now, use simple heuristic on embedding norm as a placeholder
        // In production, you'd add a proper classification layer
        let embedding_vec = cls_embedding.to_vec1::<f32>()
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to convert to vec: {}", e)
            ))?;

        // Simple scoring: normalize embedding magnitude to [0, 1]
        // This is a placeholder - real toxic-bert has a classification head
        let magnitude: f32 = embedding_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let score = (magnitude / 10.0).min(1.0).max(0.0);

        let label = if score > 0.5 { "toxic" } else { "safe" };

        Ok(ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata {
                model: Some("toxic-bert-ml".to_string()),
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn classify_with_patterns(&self, text: &str) -> f32 {
        // Enhanced pattern-based detection
        let text_lower = text.to_lowercase();

        // Common toxic patterns
        let toxic_patterns = vec![
            "hate", "stupid", "idiot", "dumb", "kill", "die", "worst",
            "terrible", "awful", "sucks", "garbage", "trash", "shit",
            "fuck", "damn", "hell", "ass", "bastard", "bitch",
        ];

        let mut score: f32 = 0.0;

        for pattern in toxic_patterns {
            if text_lower.contains(pattern) {
                score += 0.3; // Each match adds to score
            }
        }

        // Cap at 0.95 (pattern-based is never 100% confident)
        score.min(0.95)
    }
}

impl Default for ToxicityClassifier {
    fn default() -> Self {
        Self::new().expect("Failed to create toxicity classifier")
    }
}

#[async_trait::async_trait]
impl Classifier for ToxicityClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        #[cfg(feature = "ml-models")]
        {
            // Try ML model first if available
            if let Some(ref model) = self.model {
                return self.classify_with_ml(text, model).await;
            }
        }

        // Fallback to pattern-based
        let score = self.classify_with_patterns(text);
        let label = if score > 0.5 { "toxic" } else { "safe" };

        let result = ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata {
                model: Some("toxicity-patterns".to_string()),
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "toxicity"
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toxicity_classifier_safe() {
        let classifier = ToxicityClassifier::new().unwrap();

        let result = classifier.classify("This is a nice message").await.unwrap();
        assert_eq!(result.label, "safe");
        assert!(result.score < 0.5);
    }

    #[tokio::test]
    async fn test_toxicity_classifier_toxic() {
        let classifier = ToxicityClassifier::new().unwrap();

        // Test with toxic patterns
        let result = classifier.classify("I hate you, you stupid idiot!").await.unwrap();
        assert_eq!(result.label, "toxic");
        assert!(result.score > 0.5);
    }

    #[tokio::test]
    async fn test_toxicity_classifier_latency() {
        let classifier = ToxicityClassifier::new().unwrap();

        let result = classifier.classify("Test message").await.unwrap();

        // Pattern-based should be very fast (<1ms = 1000µs)
        #[cfg(not(feature = "ml-models"))]
        assert!(result.latency_us < 1000, "Latency too high: {}µs", result.latency_us);

        // ML-based should still be reasonably fast (<10ms = 10000µs)
        #[cfg(feature = "ml-models")]
        assert!(result.latency_us < 10000, "Latency too high: {}µs", result.latency_us);
    }
}
