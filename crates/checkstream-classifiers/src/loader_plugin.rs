//! Extension points for model-backed classifier loading.

use crate::classifier::Classifier;
use checkstream_core::Result;

/// Pluggable backend for dynamic model classifier loading.
///
/// Implement this trait in external crates to provide runtime model inference
/// backends (for example, Candle/ONNX/TensorRT) without coupling the core
/// classifiers crate to heavyweight dependencies.
#[async_trait::async_trait]
pub trait ModelLoaderPlugin: Send + Sync {
    /// Load a classifier instance by model name.
    async fn load_classifier(&self, name: &str) -> Result<Box<dyn Classifier>>;

    /// List model names available to this loader.
    fn available_models(&self) -> Vec<String>;
}
