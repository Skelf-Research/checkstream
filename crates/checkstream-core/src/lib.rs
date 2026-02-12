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

pub mod adapters;
pub mod error;
pub mod stream;
pub mod stream_adapter;
pub mod types;

pub use adapters::{anthropic_adapter, AdapterConfig, ConfigurableAdapter, OpenAiAdapter};
pub use error::{Error, Result};
pub use stream_adapter::{AdapterRegistry, ChunkMetadata, ParsedChunk, StreamAdapter};
pub use types::{ChatMessage, Message, StreamChunk, Token};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::adapters::{ConfigurableAdapter, OpenAiAdapter};
    pub use crate::error::{Error, Result};
    pub use crate::stream_adapter::{ChunkMetadata, ParsedChunk, StreamAdapter};
    pub use crate::types::{ChatMessage, Message, StreamChunk, Token};
}
