//! Error types for CheckStream

/// Result type alias using CheckStream's Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Core error type for CheckStream operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Policy evaluation errors
    #[error("policy error: {0}")]
    Policy(String),

    /// Classifier execution errors
    #[error("classifier error: {0}")]
    Classifier(String),

    /// Stream processing errors
    #[error("stream error: {0}")]
    Stream(String),

    /// Configuration errors
    #[error("configuration error: {0}")]
    Config(String),

    /// Network/IO errors
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Timeout errors
    #[error("operation timed out")]
    Timeout,

    /// Generic internal errors
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a new policy error
    pub fn policy(msg: impl Into<String>) -> Self {
        Self::Policy(msg.into())
    }

    /// Create a new classifier error
    pub fn classifier(msg: impl Into<String>) -> Self {
        Self::Classifier(msg.into())
    }

    /// Create a new stream error
    pub fn stream(msg: impl Into<String>) -> Self {
        Self::Stream(msg.into())
    }

    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
