//! Lightweight sentiment classifier (Tier A fallback)
//!
//! This is a lexicon-based classifier used when no external model is loaded.

use crate::classifier::{ClassificationMetadata, ClassificationResult, Classifier, ClassifierTier};
use aho_corasick::AhoCorasick;
use checkstream_core::Result;
use std::time::Instant;

pub struct SentimentClassifier {
    name: String,
    positive: AhoCorasick,
    negative: AhoCorasick,
}

impl SentimentClassifier {
    pub fn new() -> Result<Self> {
        Self::with_name("sentiment")
    }

    pub fn with_name(name: impl Into<String>) -> Result<Self> {
        let positive = vec![
            "good",
            "great",
            "excellent",
            "love",
            "amazing",
            "wonderful",
            "happy",
            "fantastic",
            "awesome",
            "best",
        ];
        let negative = vec![
            "bad",
            "terrible",
            "awful",
            "hate",
            "horrible",
            "worst",
            "sad",
            "angry",
            "disappointed",
            "poor",
        ];

        let positive = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(positive)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to build positive sentiment matcher: {e}"
                ))
            })?;

        let negative = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(negative)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to build negative sentiment matcher: {e}"
                ))
            })?;

        Ok(Self {
            name: name.into(),
            positive,
            negative,
        })
    }
}

#[async_trait::async_trait]
impl Classifier for SentimentClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let positive_hits = self.positive.find_iter(text).count() as f32;
        let negative_hits = self.negative.find_iter(text).count() as f32;
        let total = positive_hits + negative_hits;

        let score = if total == 0.0 {
            0.5
        } else {
            positive_hits / total
        };
        let label = if score >= 0.5 { "positive" } else { "negative" };

        Ok(ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata {
                model: Some("sentiment-lexicon".to_string()),
                all_scores: Some(vec![
                    ("negative".to_string(), 1.0 - score),
                    ("positive".to_string(), score),
                ]),
                ..Default::default()
            },
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::A
    }
}
