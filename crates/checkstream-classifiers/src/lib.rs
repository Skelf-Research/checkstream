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
pub mod dynamic_registry;
pub mod financial_advice;
pub mod generic_loader;
pub mod loader_plugin;
pub mod model_config;
pub mod model_loader;
pub mod patterns;
pub mod pii;
pub mod pipeline;
pub mod prompt_injection;
pub mod registry;
pub mod sentiment;
pub mod streaming;
pub mod toxicity;

pub use classifier::{ClassificationResult, Classifier, ClassifierTier};
pub use config::{
    AggregationStrategySpec, ClassifierConfig, ConditionSpec, DeviceSpec, ModelConfigSpec,
    ModelSourceSpec, PipelineConfigSpec, StageConfigSpec,
};
pub use loader_plugin::ModelLoaderPlugin;
pub use model_loader::{
    DeviceType, LoadedModel, ModelConfig, ModelFormat, ModelRegistry, ModelSource,
};
pub use pipeline::{
    AggregationStrategy, ClassifierPipeline, PipelineBuilder, PipelineExecutionResult,
    PipelineResult, PipelineStage,
};
pub use registry::{
    build_pipeline_from_config, init_registry_from_config, init_registry_from_file, load_config,
    ClassifierRegistry, SharedRegistry,
};
pub use sentiment::SentimentClassifier;
pub use streaming::{StreamingBuffer, StreamingClassifier, StreamingConfig, StreamingPipeline};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::classifier::{ClassificationResult, Classifier};
    pub use crate::financial_advice::FinancialAdviceClassifier;
    pub use crate::model_loader::{
        DeviceType, LoadedModel, ModelConfig, ModelFormat, ModelRegistry, ModelSource,
    };
    pub use crate::patterns::PatternClassifier;
    pub use crate::pii::PiiClassifier;
    pub use crate::prompt_injection::PromptInjectionClassifier;
    pub use crate::sentiment::SentimentClassifier;
    pub use crate::toxicity::ToxicityClassifier;
}
