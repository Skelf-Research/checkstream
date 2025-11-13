//! Core types for CheckStream

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents a single token in the LLM output stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// The token text content
    pub text: String,

    /// Token ID from the model's vocabulary (if available)
    pub token_id: Option<u32>,

    /// Log probability of this token (if available)
    pub logprob: Option<f32>,

    /// Timestamp when this token was generated
    pub timestamp: SystemTime,
}

impl Token {
    /// Create a new token with text content
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            token_id: None,
            logprob: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a token with additional metadata
    pub fn with_metadata(text: impl Into<String>, token_id: u32, logprob: f32) -> Self {
        Self {
            text: text.into(),
            token_id: Some(token_id),
            logprob: Some(logprob),
            timestamp: SystemTime::now(),
        }
    }
}

/// A chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender (system, user, assistant)
    pub role: String,

    /// Content of the message
    pub content: String,

    /// Optional name of the sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            name: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }
}

/// Represents a chunk in the streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique identifier for this chunk
    pub id: String,

    /// Model that generated this chunk
    pub model: String,

    /// Tokens in this chunk
    pub tokens: Vec<Token>,

    /// Whether this is the final chunk
    pub finish_reason: Option<String>,

    /// Timestamp for this chunk
    pub created: SystemTime,
}

impl StreamChunk {
    /// Create a new stream chunk
    pub fn new(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            tokens: Vec::new(),
            finish_reason: None,
            created: SystemTime::now(),
        }
    }

    /// Add a token to this chunk
    pub fn add_token(&mut self, token: Token) {
        self.tokens.push(token);
    }

    /// Mark this chunk as the final one
    pub fn finish(mut self, reason: impl Into<String>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }
}

/// Complete message containing multiple chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Conversation messages (for context)
    pub messages: Vec<ChatMessage>,

    /// Generated tokens
    pub tokens: Vec<Token>,

    /// Metadata about the generation
    pub metadata: MessageMetadata,
}

/// Metadata about a generated message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Model used for generation
    pub model: String,

    /// Total tokens generated
    pub token_count: usize,

    /// Time taken for generation (milliseconds)
    pub latency_ms: u64,

    /// CheckStream processing overhead (milliseconds)
    pub overhead_ms: u64,

    /// Policies that were triggered
    pub triggered_policies: Vec<String>,
}

impl MessageMetadata {
    /// Create new metadata
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            token_count: 0,
            latency_ms: 0,
            overhead_ms: 0,
            triggered_policies: Vec::new(),
        }
    }
}
