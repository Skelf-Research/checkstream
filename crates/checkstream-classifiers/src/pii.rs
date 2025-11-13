//! PII detection classifier (Tier A)

use crate::classifier::{Classifier, ClassificationResult, ClassificationMetadata, ClassifierTier};
use checkstream_core::Result;
use regex::Regex;
use std::time::Instant;

/// PII detection classifier using regex patterns
pub struct PiiClassifier {
    email_regex: Regex,
    phone_regex: Regex,
    ssn_regex: Regex,
    credit_card_regex: Regex,
}

impl PiiClassifier {
    /// Create a new PII classifier
    pub fn new() -> Result<Self> {
        Ok(Self {
            email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
                .map_err(|e| checkstream_core::Error::classifier(format!("Failed to compile email regex: {}", e)))?,
            phone_regex: Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b")
                .map_err(|e| checkstream_core::Error::classifier(format!("Failed to compile phone regex: {}", e)))?,
            ssn_regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")
                .map_err(|e| checkstream_core::Error::classifier(format!("Failed to compile SSN regex: {}", e)))?,
            credit_card_regex: Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")
                .map_err(|e| checkstream_core::Error::classifier(format!("Failed to compile credit card regex: {}", e)))?,
        })
    }
}

impl Default for PiiClassifier {
    fn default() -> Self {
        Self::new().expect("Failed to create PII classifier")
    }
}

#[async_trait::async_trait]
impl Classifier for PiiClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let mut pii_types = Vec::new();
        let mut spans = Vec::new();

        // Check for emails
        if let Some(mat) = self.email_regex.find(text) {
            pii_types.push("email");
            spans.push((mat.start(), mat.end()));
        }

        // Check for phone numbers
        if let Some(mat) = self.phone_regex.find(text) {
            pii_types.push("phone");
            spans.push((mat.start(), mat.end()));
        }

        // Check for SSN
        if let Some(mat) = self.ssn_regex.find(text) {
            pii_types.push("ssn");
            spans.push((mat.start(), mat.end()));
        }

        // Check for credit cards
        if let Some(mat) = self.credit_card_regex.find(text) {
            pii_types.push("credit_card");
            spans.push((mat.start(), mat.end()));
        }

        let result = if pii_types.is_empty() {
            ClassificationResult {
                label: "no_pii".to_string(),
                score: 0.0,
                metadata: ClassificationMetadata::default(),
                latency_us: start.elapsed().as_micros() as u64,
            }
        } else {
            let mut metadata = ClassificationMetadata::default();
            metadata.spans = spans;
            metadata.extra = pii_types.iter()
                .map(|t| ("pii_type".to_string(), t.to_string()))
                .collect();

            ClassificationResult {
                label: "pii_detected".to_string(),
                score: 1.0,
                metadata,
                latency_us: start.elapsed().as_micros() as u64,
            }
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "pii_detector"
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::A
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_detection() {
        let classifier = PiiClassifier::new().unwrap();

        let result = classifier.classify("Contact me at john@example.com").await.unwrap();
        assert_eq!(result.label, "pii_detected");
        assert_eq!(result.score, 1.0);
    }

    #[tokio::test]
    async fn test_no_pii() {
        let classifier = PiiClassifier::new().unwrap();

        let result = classifier.classify("This is clean text").await.unwrap();
        assert_eq!(result.label, "no_pii");
        assert_eq!(result.score, 0.0);
    }

    #[tokio::test]
    async fn test_phone_detection() {
        let classifier = PiiClassifier::new().unwrap();

        let result = classifier.classify("Call me at 555-123-4567").await.unwrap();
        assert_eq!(result.label, "pii_detected");
    }
}
