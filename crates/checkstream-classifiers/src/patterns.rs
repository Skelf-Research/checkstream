//! Pattern-based classifier (Tier A)

use crate::classifier::{Classifier, ClassificationResult, ClassificationMetadata, ClassifierTier};
use aho_corasick::AhoCorasick;
use checkstream_core::Result;
use std::time::Instant;

/// Fast pattern-based classifier using Aho-Corasick algorithm
pub struct PatternClassifier {
    name: String,
    patterns: AhoCorasick,
    pattern_labels: Vec<String>,
}

impl PatternClassifier {
    /// Create a new pattern classifier
    pub fn new(name: impl Into<String>, patterns: Vec<(String, String)>) -> Result<Self> {
        let (labels, pattern_strs): (Vec<_>, Vec<_>) = patterns.into_iter().unzip();

        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&pattern_strs)
            .map_err(|e| checkstream_core::Error::classifier(format!("Failed to build pattern matcher: {}", e)))?;

        Ok(Self {
            name: name.into(),
            patterns: ac,
            pattern_labels: labels,
        })
    }
}

#[async_trait::async_trait]
impl Classifier for PatternClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let matches: Vec<_> = self.patterns.find_iter(text).collect();

        let result = if matches.is_empty() {
            ClassificationResult {
                label: "clean".to_string(),
                score: 0.0,
                metadata: ClassificationMetadata::default(),
                latency_us: start.elapsed().as_micros() as u64,
            }
        } else {
            let match_info = &matches[0];
            let label = self.pattern_labels[match_info.pattern().as_usize()].clone();

            let mut metadata = ClassificationMetadata::default();
            metadata.spans = matches.iter()
                .map(|m| (m.start(), m.end()))
                .collect();

            ClassificationResult {
                label,
                score: 1.0, // Pattern matches are binary
                metadata,
                latency_us: start.elapsed().as_micros() as u64,
            }
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::A
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pattern_classifier() {
        let patterns = vec![
            ("profanity".to_string(), "badword".to_string()),
            ("spam".to_string(), "click here".to_string()),
        ];

        let classifier = PatternClassifier::new("test", patterns).unwrap();

        let result = classifier.classify("this is clean text").await.unwrap();
        assert_eq!(result.label, "clean");
        assert_eq!(result.score, 0.0);

        let result = classifier.classify("click here for free stuff").await.unwrap();
        assert_eq!(result.label, "spam");
        assert_eq!(result.score, 1.0);
        assert!(!result.metadata.spans.is_empty());
    }
}
