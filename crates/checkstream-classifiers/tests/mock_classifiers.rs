//! Mock classifiers for testing
//!
//! Provides configurable mock implementations of the Classifier trait
//! for testing pipelines, aggregation strategies, and error handling.

use async_trait::async_trait;
use checkstream_classifiers::classifier::ClassificationMetadata;
use checkstream_classifiers::{ClassificationResult, Classifier, ClassifierTier};
use checkstream_core::Result;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// A configurable mock classifier for testing
pub struct MockClassifier {
    name: String,
    score: f32,
    label: String,
    tier: ClassifierTier,
    simulated_latency: Option<Duration>,
    call_count: AtomicU32,
}

impl MockClassifier {
    /// Create a new mock classifier with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            score: 0.5,
            label: "neutral".to_string(),
            tier: ClassifierTier::A,
            simulated_latency: None,
            call_count: AtomicU32::new(0),
        }
    }

    /// Set the score this classifier will return
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Set the label this classifier will return
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the tier for this classifier
    pub fn with_tier(mut self, tier: ClassifierTier) -> Self {
        self.tier = tier;
        self
    }

    /// Set simulated latency for this classifier
    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.simulated_latency = Some(latency);
        self
    }

    /// Get the number of times classify was called
    pub fn call_count(&self) -> u32 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Reset the call counter
    pub fn reset_call_count(&self) {
        self.call_count.store(0, Ordering::Relaxed);
    }
}

#[async_trait]
impl Classifier for MockClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        // Simulate latency if configured
        if let Some(latency) = self.simulated_latency {
            tokio::time::sleep(latency).await;
        }

        // Dynamic scoring based on text content (for testing)
        let score = if text.contains("UNSAFE") {
            0.95
        } else if text.contains("SUSPICIOUS") {
            0.7
        } else if text.contains("SAFE") {
            0.1
        } else {
            self.score
        };

        let label = if score >= 0.5 {
            "positive".to_string()
        } else {
            "negative".to_string()
        };

        Ok(ClassificationResult {
            label,
            score,
            metadata: ClassificationMetadata::default(),
            latency_us: self
                .simulated_latency
                .map(|d| d.as_micros() as u64)
                .unwrap_or(100),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        self.tier
    }
}

/// A classifier that always fails - for testing error paths
pub struct FailingClassifier {
    name: String,
    error_message: String,
}

impl FailingClassifier {
    /// Create a new failing classifier
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            error_message: "Simulated classifier failure".to_string(),
        }
    }

    /// Set a custom error message
    pub fn with_error(mut self, message: &str) -> Self {
        self.error_message = message.to_string();
        self
    }
}

#[async_trait]
impl Classifier for FailingClassifier {
    async fn classify(&self, _text: &str) -> Result<ClassificationResult> {
        Err(checkstream_core::Error::classifier(&self.error_message))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::A
    }
}

/// A classifier with variable latency for performance testing
pub struct VariableLatencyClassifier {
    name: String,
    base_latency_us: u64,
    variance_us: u64,
}

impl VariableLatencyClassifier {
    /// Create a new variable latency classifier
    pub fn new(name: &str, base_latency_us: u64, variance_us: u64) -> Self {
        Self {
            name: name.to_string(),
            base_latency_us,
            variance_us,
        }
    }
}

#[async_trait]
impl Classifier for VariableLatencyClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        // Simple pseudo-random variance based on text length
        let variance = text.len() as u64 % (self.variance_us + 1);
        let latency = Duration::from_micros(self.base_latency_us + variance);

        tokio::time::sleep(latency).await;

        Ok(ClassificationResult {
            label: "processed".to_string(),
            score: 0.5,
            metadata: ClassificationMetadata::default(),
            latency_us: latency.as_micros() as u64,
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
    async fn test_mock_classifier_basic() {
        let classifier = MockClassifier::new("test")
            .with_score(0.8)
            .with_label("positive");

        let result = classifier.classify("hello").await.unwrap();
        assert_eq!(result.score, 0.8);
        assert_eq!(classifier.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_classifier_dynamic_scoring() {
        let classifier = MockClassifier::new("test");

        let result = classifier.classify("This is UNSAFE content").await.unwrap();
        assert!(result.score > 0.9);

        let result = classifier.classify("This is SAFE content").await.unwrap();
        assert!(result.score < 0.2);
    }

    #[tokio::test]
    async fn test_failing_classifier() {
        let classifier = FailingClassifier::new("fail-test").with_error("Custom error");

        let result = classifier.classify("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_classifier_latency() {
        let classifier = MockClassifier::new("slow").with_latency(Duration::from_millis(10));

        let start = std::time::Instant::now();
        let _ = classifier.classify("test").await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
    }
}
