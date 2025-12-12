//! Stream Adapters
//!
//! Pluggable adapters for parsing various streaming formats from LLM backends.

mod openai;
pub mod configurable;

pub use openai::OpenAiAdapter;
pub use configurable::{ConfigurableAdapter, AdapterConfig, anthropic_adapter};

use crate::stream_adapter::{AdapterRegistry, StreamAdapter};

/// Create a default adapter registry with built-in adapters
pub fn default_registry() -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();

    registry.register("openai", Box::new(OpenAiAdapter::new()));
    registry.register("anthropic", Box::new(configurable::anthropic_adapter()));

    registry
}

/// Get an adapter by name, falling back to OpenAI
pub fn get_adapter(name: &str) -> Box<dyn StreamAdapter> {
    match name.to_lowercase().as_str() {
        "openai" | "openai_sse" => Box::new(OpenAiAdapter::new()),
        "anthropic" => Box::new(configurable::anthropic_adapter()),
        _ => Box::new(OpenAiAdapter::new()), // Default fallback
    }
}
