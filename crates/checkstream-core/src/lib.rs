//! CheckStream Core
//!
//! Core types, traits, and utilities shared across CheckStream components.
//!
//! This crate provides:
//! - Common types for tokens, messages, and streaming events
//! - Error types and result handling
//! - Shared traits for policies, classifiers, and actions
//! - Performance-critical utilities (buffer management, zero-copy operations)

pub mod error;
pub mod stream;
pub mod types;

pub use error::{Error, Result};
pub use types::{Token, Message, ChatMessage, StreamChunk};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::types::{Token, Message, ChatMessage, StreamChunk};
}
