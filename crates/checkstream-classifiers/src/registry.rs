//! Classifier registry initialization and management

use crate::{
    Classifier, ClassifierConfig, ClassifierPipeline, ModelRegistry,
    PipelineConfigSpec, StageConfigSpec,
    pii::PiiClassifier,
    toxicity::ToxicityClassifier,
    patterns::PatternClassifier,
};
use checkstream_core::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Registry for managing classifiers and pipelines
pub struct ClassifierRegistry {
    /// Loaded classifier configuration
    config: ClassifierConfig,

    /// Model registry for ML models
    model_registry: ModelRegistry,

    /// Instantiated classifiers by name
    classifiers: HashMap<String, Arc<dyn Classifier>>,
}

impl ClassifierRegistry {
    /// Create a new registry from configuration
    pub fn new(config: ClassifierConfig, model_registry: ModelRegistry) -> Self {
        Self {
            config,
            model_registry,
            classifiers: HashMap::new(),
        }
    }

    /// Load registry from configuration file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let config = load_config(path)?;
        let model_registry = init_registry_from_config(&config)?;

        let mut registry = Self::new(config, model_registry);
        registry.initialize_classifiers().await?;

        Ok(registry)
    }

    /// Initialize all classifiers from configuration
    async fn initialize_classifiers(&mut self) -> Result<()> {
        info!("Initializing classifiers");

        // For now, initialize placeholder classifiers
        // TODO: Load actual classifiers based on configuration

        // Add basic classifiers
        self.classifiers.insert(
            "pii".to_string(),
            Arc::new(PiiClassifier::new()?),
        );

        self.classifiers.insert(
            "toxicity".to_string(),
            Arc::new(ToxicityClassifier::new()?),
        );

        self.classifiers.insert(
            "profanity".to_string(),
            Arc::new(PatternClassifier::new(
                "profanity".to_string(),
                vec![], // Empty patterns for now
            )?),
        );

        info!("Initialized {} classifiers", self.classifiers.len());

        Ok(())
    }

    /// Get the number of loaded classifiers
    pub fn count(&self) -> usize {
        self.classifiers.len()
    }

    /// Build a pipeline from configuration by name
    pub fn build_pipeline(&self, pipeline_name: &str) -> Result<ClassifierPipeline> {
        let pipeline_config = self.config.pipelines.get(pipeline_name)
            .ok_or_else(|| checkstream_core::Error::config(
                format!("Pipeline '{}' not found", pipeline_name)
            ))?;

        build_pipeline_from_config(pipeline_config, &self.classifiers)
    }
}

/// Initialize model registry from classifier configuration
pub fn init_registry_from_config(config: &ClassifierConfig) -> Result<ModelRegistry> {
    let mut registry = ModelRegistry::new();

    info!("Initializing model registry with {} models", config.models.len());

    for model_name in config.model_names() {
        info!("Loading model: {}", model_name);

        let model_config = config.to_model_config(&model_name)
            .ok_or_else(|| checkstream_core::Error::config(format!("Model {} not found in config", model_name)))?;

        match registry.load_and_register(&model_name, model_config) {
            Ok(_) => {
                info!("✓ Loaded model: {}", model_name);
            }
            Err(e) => {
                warn!("✗ Failed to load model {}: {}", model_name, e);
                // Continue with other models rather than failing completely
            }
        }
    }

    let loaded_count = registry.model_names().len();
    info!("Model registry initialized with {}/{} models", loaded_count, config.models.len());

    Ok(registry)
}

/// Load classifier configuration from file
pub fn load_config(path: impl AsRef<Path>) -> Result<ClassifierConfig> {
    ClassifierConfig::from_file(path.as_ref())
        .map_err(|e| checkstream_core::Error::config(format!("Failed to load classifiers config: {}", e)))
}

/// Initialize registry from config file
pub fn init_registry_from_file(path: impl AsRef<Path>) -> Result<ModelRegistry> {
    let config = load_config(path)?;
    init_registry_from_config(&config)
}

/// Build a classifier pipeline from configuration
///
/// Takes a pipeline configuration and a map of available classifiers,
/// and constructs a complete `ClassifierPipeline` with all stages configured.
///
/// # Arguments
///
/// * `config` - The pipeline configuration specification
/// * `classifiers` - Map of classifier names to classifier implementations
///
/// # Returns
///
/// A configured `ClassifierPipeline` ready for execution
///
/// # Errors
///
/// Returns an error if:
/// - A referenced classifier is not found in the map
/// - Stage configuration is invalid
pub fn build_pipeline_from_config(
    config: &PipelineConfigSpec,
    classifiers: &HashMap<String, Arc<dyn Classifier>>,
) -> Result<ClassifierPipeline> {
    let mut pipeline = ClassifierPipeline::new();

    for stage_spec in &config.stages {
        match stage_spec {
            StageConfigSpec::Single { name, classifier } => {
                let classifier_impl = classifiers
                    .get(classifier)
                    .ok_or_else(|| {
                        checkstream_core::Error::config(format!(
                            "Classifier '{}' not found for stage '{}'",
                            classifier, name
                        ))
                    })?
                    .clone();

                pipeline = pipeline.add_single(name.clone(), classifier_impl);
            }

            StageConfigSpec::Parallel {
                name,
                classifiers: classifier_names,
                aggregation,
            } => {
                let mut stage_classifiers = Vec::new();

                for classifier_name in classifier_names {
                    let classifier_impl = classifiers
                        .get(classifier_name)
                        .ok_or_else(|| {
                            checkstream_core::Error::config(format!(
                                "Classifier '{}' not found for parallel stage '{}'",
                                classifier_name, name
                            ))
                        })?
                        .clone();

                    stage_classifiers.push((classifier_name.clone(), classifier_impl));
                }

                pipeline = pipeline.add_parallel(
                    name.clone(),
                    stage_classifiers,
                    aggregation.to_aggregation_strategy(),
                );
            }

            StageConfigSpec::Sequential {
                name,
                classifiers: classifier_names,
            } => {
                let mut stage_classifiers = Vec::new();

                for classifier_name in classifier_names {
                    let classifier_impl = classifiers
                        .get(classifier_name)
                        .ok_or_else(|| {
                            checkstream_core::Error::config(format!(
                                "Classifier '{}' not found for sequential stage '{}'",
                                classifier_name, name
                            ))
                        })?
                        .clone();

                    stage_classifiers.push((classifier_name.clone(), classifier_impl));
                }

                pipeline = pipeline.add_sequential(name.clone(), stage_classifiers);
            }

            StageConfigSpec::Conditional {
                name,
                classifier,
                condition,
            } => {
                let classifier_impl = classifiers
                    .get(classifier)
                    .ok_or_else(|| {
                        checkstream_core::Error::config(format!(
                            "Classifier '{}' not found for conditional stage '{}'",
                            classifier, name
                        ))
                    })?
                    .clone();

                let condition_fn = condition.to_condition_fn();

                pipeline = pipeline.add_conditional(name.clone(), move |results| condition_fn(results), classifier_impl);
            }
        }
    }

    Ok(pipeline)
}

/// Shared model registry for application-wide use
pub struct SharedRegistry {
    registry: Arc<ModelRegistry>,
}

impl SharedRegistry {
    /// Create a new shared registry
    pub fn new(registry: ModelRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Get reference to the registry
    pub fn registry(&self) -> &Arc<ModelRegistry> {
        &self.registry
    }

    /// Clone the Arc for sharing across threads
    pub fn clone_arc(&self) -> Arc<ModelRegistry> {
        Arc::clone(&self.registry)
    }
}

impl Clone for SharedRegistry {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let yaml = r#"
models:
  test:
    repo_id: test/model
    filename: model.safetensors

default_device: cpu
"#;

        // Write to temp file
        let temp_file = std::env::temp_dir().join("test_classifiers.yaml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = load_config(&temp_file).unwrap();
        assert_eq!(config.models.len(), 1);

        std::fs::remove_file(&temp_file).ok();
    }
}
