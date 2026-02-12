//! Stream Adapters
//!
//! Pluggable adapters for parsing various streaming formats from LLM backends.

pub mod configurable;
mod openai;

pub use configurable::{anthropic_adapter, AdapterConfig, ConfigurableAdapter};
pub use openai::OpenAiAdapter;

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
