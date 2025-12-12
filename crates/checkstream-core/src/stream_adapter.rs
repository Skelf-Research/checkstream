//! Stream Adapter System
//!
//! Provides a pluggable system for parsing different streaming formats
//! (OpenAI SSE, Anthropic, custom formats, etc.)

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Metadata about a parsed chunk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Chunk ID from the upstream source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Index in multi-choice responses
    #[serde(default)]
    pub index: usize,

    /// Creation timestamp (unix epoch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Event type (for event-based streams like Anthropic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
}

/// Represents a parsed chunk from a streaming response
#[derive(Debug, Clone)]
pub enum ParsedChunk {
    /// Content token extracted from the stream
    Content {
        /// The text content
        text: String,
        /// Associated metadata
        metadata: ChunkMetadata,
    },

    /// Stream completion signal
    Done {
        /// Reason for completion (stop, content_filter, length, etc.)
        finish_reason: Option<String>,
        /// Optional metadata
        metadata: Option<serde_json::Value>,
    },

    /// Empty chunk (no content but not done)
    Empty,

    /// Pass-through data (not parsed, forward as-is)
    /// Used for non-content events like tool calls, usage stats, etc.
    PassThrough(String),

    /// Parse error
    Error(String),
}

impl ParsedChunk {
    /// Create a content chunk
    pub fn content(text: impl Into<String>) -> Self {
        Self::Content {
            text: text.into(),
            metadata: ChunkMetadata::default(),
        }
    }

    /// Create a content chunk with metadata
    pub fn content_with_metadata(text: impl Into<String>, metadata: ChunkMetadata) -> Self {
        Self::Content {
            text: text.into(),
            metadata,
        }
    }

    /// Create a done chunk
    pub fn done(finish_reason: Option<String>) -> Self {
        Self::Done {
            finish_reason,
            metadata: None,
        }
    }

    /// Check if this is a content chunk
    pub fn is_content(&self) -> bool {
        matches!(self, Self::Content { .. })
    }

    /// Check if this is a done signal
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. })
    }

    /// Get the text content if this is a content chunk
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Content { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// Trait for parsing different streaming formats
///
/// Implementations of this trait handle format-specific parsing logic
/// for SSE streams from various LLM providers.
pub trait StreamAdapter: Send + Sync + Debug {
    /// Name of this adapter (for logging/debugging)
    fn name(&self) -> &str;

    /// Parse raw bytes into chunks
    ///
    /// This method should handle the wire format (SSE, NDJSON, etc.)
    /// and extract content tokens.
    ///
    /// # Arguments
    /// * `data` - Raw string data from the stream (typically one SSE event)
    ///
    /// # Returns
    /// A vector of parsed chunks (may be empty, one, or multiple)
    fn parse(&self, data: &str) -> Vec<ParsedChunk>;

    /// Check if this data represents end of stream
    fn is_done_marker(&self, data: &str) -> bool;

    /// Get the expected content type for this format
    fn content_type(&self) -> &str {
        "text/event-stream"
    }
}

/// Registry of available stream adapters
#[derive(Debug, Default)]
pub struct AdapterRegistry {
    adapters: std::collections::HashMap<String, Box<dyn StreamAdapter>>,
}

impl AdapterRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an adapter
    pub fn register(&mut self, name: impl Into<String>, adapter: Box<dyn StreamAdapter>) {
        self.adapters.insert(name.into(), adapter);
    }

    /// Get an adapter by name
    pub fn get(&self, name: &str) -> Option<&dyn StreamAdapter> {
        self.adapters.get(name).map(|a| a.as_ref())
    }

    /// List all registered adapters
    pub fn list(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_chunk_creation() {
        let chunk = ParsedChunk::content("hello");
        assert!(chunk.is_content());
        assert_eq!(chunk.text(), Some("hello"));

        let done = ParsedChunk::done(Some("stop".to_string()));
        assert!(done.is_done());
        assert_eq!(done.text(), None);
    }
}
