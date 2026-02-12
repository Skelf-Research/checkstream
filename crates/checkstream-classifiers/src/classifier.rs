//! Classifier trait and common types

use async_trait::async_trait;
use checkstream_core::Result;

/// Trait for all classifiers
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Classify the given text
    async fn classify(&self, text: &str) -> Result<ClassificationResult>;

    /// Get the classifier name
    fn name(&self) -> &str;

    /// Get the tier (performance category)
    fn tier(&self) -> ClassifierTier;
}

/// Result of classification
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Classification label
    pub label: String,

    /// Confidence score (0.0-1.0)
    pub score: f32,

    /// Additional metadata
    pub metadata: ClassificationMetadata,

    /// Latency in microseconds
    pub latency_us: u64,
}

impl ClassificationResult {
    /// Create a new classification result
    pub fn new(label: impl Into<String>, score: f32) -> Self {
        Self {
            label: label.into(),
            score,
            metadata: ClassificationMetadata::default(),
            latency_us: 0,
        }
    }

    /// Check if score exceeds threshold
    pub fn exceeds_threshold(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
}

/// Metadata about classification
#[derive(Debug, Clone, Default)]
pub struct ClassificationMetadata {
    /// Matched spans (for pattern-based classifiers)
    pub spans: Vec<(usize, usize)>,

    /// Model name or version
    pub model: Option<String>,

    /// All class scores (for multi-class classifiers)
    pub all_scores: Option<Vec<(String, f32)>>,

    /// Additional key-value pairs
    pub extra: Vec<(String, String)>,
}

/// Classifier performance tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierTier {
    /// Ultra-fast (<2ms) - Pattern matching, simple rules
    A,
    /// Fast (<5ms) - Quantized models, lightweight ML
    B,
    /// Moderate (<10ms) - Full models for nuanced classification
    C,
}

impl ClassifierTier {
    /// Get the latency budget for this tier in microseconds
    pub fn latency_budget_us(&self) -> u64 {
        match self {
            Self::A => 2_000,
            Self::B => 5_000,
            Self::C => 10_000,
        }
    }
}
