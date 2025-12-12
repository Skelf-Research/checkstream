//! CheckStream Core
//!
//! Core types, traits, and utilities shared across CheckStream components.
//!
//! This crate provides:
//! - Common types for tokens, messages, and streaming events
//! - Error types and result handling
//! - Shared traits for policies, classifiers, and actions
//! - Performance-critical utilities (buffer management, zero-copy operations)
//! - Stream adapters for parsing various LLM streaming formats

pub mod error;
pub mod stream;
pub mod types;
pub mod stream_adapter;
pub mod adapters;

pub use error::{Error, Result};
pub use types::{Token, Message, ChatMessage, StreamChunk};
pub use stream_adapter::{StreamAdapter, ParsedChunk, ChunkMetadata, AdapterRegistry};
pub use adapters::{OpenAiAdapter, ConfigurableAdapter, AdapterConfig, anthropic_adapter};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::types::{Token, Message, ChatMessage, StreamChunk};
    pub use crate::stream_adapter::{StreamAdapter, ParsedChunk, ChunkMetadata};
    pub use crate::adapters::{OpenAiAdapter, ConfigurableAdapter};
}
