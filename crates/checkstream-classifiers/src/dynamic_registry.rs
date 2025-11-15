//! Dynamic classifier registry that loads models from configuration

use crate::classifier::Classifier;
use crate::generic_loader::GenericModelLoader;
use crate::model_config::ModelRegistry;
use crate::pii::PiiClassifier;
use crate::patterns::PatternClassifier;
use checkstream_core::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Dynamic classifier registry that loads classifiers from model registry
pub struct DynamicClassifierRegistry {
    model_loader: Arc<GenericModelLoader>,
    classifiers: Arc<RwLock<HashMap<String, Arc<dyn Classifier>>>>,
}

impl DynamicClassifierRegistry {
    /// Create a new dynamic registry from a model registry file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let model_registry = ModelRegistry::from_file(path)
            .map_err(|e| checkstream_core::Error::classifier(
                format!("Failed to load model registry: {}", e)
            ))?;

        let model_loader = Arc::new(GenericModelLoader::new(model_registry));
        let classifiers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            model_loader,
            classifiers,
        })
    }

    /// Get a classifier by name (loads on first access)
    pub async fn get_classifier(&self, name: &str) -> Result<Arc<dyn Classifier>> {
        // Check if already loaded
        {
            let classifiers = self.classifiers.read().await;
            if let Some(classifier) = classifiers.get(name) {
                return Ok(Arc::clone(classifier));
            }
        }

        // Load the classifier
        tracing::info!("Loading classifier '{}' from registry", name);
        let classifier = self.model_loader.load_classifier(name).await?;
        let classifier = Arc::from(classifier);

        // Cache it
        {
            let mut classifiers = self.classifiers.write().await;
            classifiers.insert(name.to_string(), Arc::clone(&classifier));
        }

        Ok(classifier)
    }

    /// Register a built-in classifier (pattern-based, etc.)
    pub async fn register_builtin(&self, name: String, classifier: Arc<dyn Classifier>) {
        let mut classifiers = self.classifiers.write().await;
        classifiers.insert(name, classifier);
    }

    /// Preload specified classifiers
    pub async fn preload(&self, names: &[String]) -> Result<()> {
        for name in names {
            self.get_classifier(name).await?;
        }
        Ok(())
    }

    /// List all available model names
    pub fn available_models(&self) -> Vec<String> {
        self.model_loader.registry.models.keys().cloned().collect()
    }
}

/// Builder for dynamic classifier registry with convenience methods
pub struct DynamicRegistryBuilder {
    model_registry_path: Option<String>,
    builtin_classifiers: Vec<(String, Arc<dyn Classifier>)>,
    preload: Vec<String>,
}

impl DynamicRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            model_registry_path: None,
            builtin_classifiers: Vec::new(),
            preload: Vec::new(),
        }
    }

    /// Set the model registry file path
    pub fn with_model_registry(mut self, path: impl Into<String>) -> Self {
        self.model_registry_path = Some(path.into());
        self
    }

    /// Register a built-in classifier
    pub fn with_builtin(mut self, name: impl Into<String>, classifier: Arc<dyn Classifier>) -> Self {
        self.builtin_classifiers.push((name.into(), classifier));
        self
    }

    /// Add a classifier to preload
    pub fn preload(mut self, name: impl Into<String>) -> Self {
        self.preload.push(name.into());
        self
    }

    /// Build the registry
    pub async fn build(self) -> Result<DynamicClassifierRegistry> {
        let registry_path = self.model_registry_path
            .unwrap_or_else(|| "models/registry.yaml".to_string());

        let registry = DynamicClassifierRegistry::from_file(&registry_path).await?;

        // Register built-ins
        for (name, classifier) in self.builtin_classifiers {
            registry.register_builtin(name, classifier).await;
        }

        // Preload if requested
        if !self.preload.is_empty() {
            registry.preload(&self.preload).await?;
        }

        Ok(registry)
    }
}

impl Default for DynamicRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dynamic_registry_builder() {
        let builder = DynamicRegistryBuilder::new()
            .with_model_registry("models/registry.yaml");

        // This will fail if the file doesn't exist, which is fine for testing
        let _ = builder.build().await;
    }
}
