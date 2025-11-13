//! CheckStream Classifiers
//!
//! High-performance classifiers for real-time safety and compliance checks.
//!
//! Classifiers are organized into tiers based on latency budgets:
//! - Tier A (<2ms): Pattern matching, PII detection, basic toxicity
//! - Tier B (<5ms): Quantized neural classifiers for prompt injection, advice detection
//! - Tier C (<10ms): Larger models for nuanced classification
//!
//! All classifiers are designed to run on CPU with minimal overhead.

pub mod classifier;
pub mod config;
pub mod model_loader;
pub mod pipeline;
pub mod registry;
pub mod pii;
pub mod toxicity;
pub mod patterns;

pub use classifier::{Classifier, ClassificationResult, ClassifierTier};
pub use config::{
    ClassifierConfig, ModelConfigSpec, ModelSourceSpec, DeviceSpec,
    PipelineConfigSpec, StageConfigSpec, AggregationStrategySpec, ConditionSpec,
};
pub use model_loader::{
    LoadedModel, ModelConfig, ModelRegistry, ModelSource, DeviceType, ModelFormat,
};
pub use pipeline::{
    ClassifierPipeline, PipelineBuilder, PipelineStage, PipelineResult,
    PipelineExecutionResult, AggregationStrategy,
};
pub use registry::{
    build_pipeline_from_config, init_registry_from_config, init_registry_from_file,
    load_config, SharedRegistry,
};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::classifier::{Classifier, ClassificationResult};
    pub use crate::model_loader::{LoadedModel, ModelConfig, ModelRegistry, ModelSource, DeviceType, ModelFormat};
    pub use crate::pii::PiiClassifier;
    pub use crate::toxicity::ToxicityClassifier;
    pub use crate::patterns::PatternClassifier;
}
