//! Toxicity detection classifier (Tier B)

use crate::classifier::{ClassificationMetadata, ClassificationResult, Classifier, ClassifierTier};
use checkstream_core::Result;
use std::time::Instant;

/// Toxicity detection classifier.
///
/// This implementation is intentionally dependency-light for production
/// hardening and deterministic operation.
pub struct ToxicityClassifier {
    name: String,
}

impl ToxicityClassifier {
    /// Create a new toxicity classifier.
    pub fn new() -> Result<Self> {
        Ok(Self {
            name: "toxicity".to_string(),
        })
    }

    fn classify_with_patterns(&self, text: &str) -> f32 {
        let text_lower = text.to_lowercase();

        let toxic_patterns = [
            "hate", "stupid", "idiot", "dumb", "kill", "die", "worst", "terrible", "awful",
            "sucks", "garbage", "trash", "shit", "fuck", "damn", "hell", "asshole", "bastard",
            "bitch",
        ];

        let matches = toxic_patterns
            .iter()
            .filter(|pattern| text_lower.contains(**pattern))
            .count() as f32;

        // Keep confidence bounded for lexicon-only approach.
        (matches * 0.35).clamp(0.0, 0.95)
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

        let score = self.classify_with_patterns(text);
        let label = if score > 0.5 { "toxic" } else { "safe" };

        Ok(ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata {
                model: Some("toxicity-lexicon".to_string()),
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
    async fn test_toxicity_classifier_safe() {
        let classifier = ToxicityClassifier::new().unwrap();

        let result = classifier.classify("This is a nice message").await.unwrap();
        assert_eq!(result.label, "safe");
        assert!(result.score < 0.5);
    }

    #[tokio::test]
    async fn test_toxicity_classifier_toxic() {
        let classifier = ToxicityClassifier::new().unwrap();

        let result = classifier
            .classify("I hate you, you stupid idiot!")
            .await
            .unwrap();
        assert_eq!(result.label, "toxic");
        assert!(result.score > 0.5);
    }

    #[tokio::test]
    async fn test_toxicity_classifier_latency() {
        let classifier = ToxicityClassifier::new().unwrap();

        let result = classifier.classify("Test message").await.unwrap();

        assert!(
            result.latency_us < 10_000,
            "Latency too high: {}us",
            result.latency_us
        );
    }
}
