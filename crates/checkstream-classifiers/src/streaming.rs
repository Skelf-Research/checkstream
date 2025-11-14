//! Streaming classification with configurable context windows
//!
//! This module provides utilities for running classifiers on streaming text
//! with the ability to see previous chunks for context.

use crate::{Classifier, ClassificationResult, ClassifierPipeline, PipelineExecutionResult};
use checkstream_core::Result;
use std::collections::VecDeque;
use std::sync::Arc;

/// Configuration for streaming classification behavior
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Number of previous chunks to include as context
    /// - 0 = See entire buffer (all previous chunks)
    /// - N = See last N chunks
    pub context_chunks: usize,

    /// Maximum buffer size (to prevent unbounded memory growth)
    pub max_buffer_size: usize,

    /// Delimiter to join chunks (typically " " or "")
    pub chunk_delimiter: String,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            context_chunks: 3,       // Last 3 chunks by default
            max_buffer_size: 100,    // Max 100 chunks
            chunk_delimiter: " ".to_string(),
        }
    }
}

impl StreamingConfig {
    /// Create config that sees entire buffer
    pub fn entire_buffer() -> Self {
        Self {
            context_chunks: 0,  // 0 = entire buffer
            max_buffer_size: 1000,
            chunk_delimiter: " ".to_string(),
        }
    }

    /// Create config that only sees current chunk (no context)
    pub fn no_context() -> Self {
        Self {
            context_chunks: 1,  // Only current chunk
            max_buffer_size: 10,
            chunk_delimiter: " ".to_string(),
        }
    }

    /// Create config with specific window size
    pub fn with_window(chunks: usize) -> Self {
        Self {
            context_chunks: chunks,
            max_buffer_size: chunks.max(100),
            chunk_delimiter: " ".to_string(),
        }
    }
}

/// Buffer for streaming chunks with context awareness
#[derive(Debug)]
pub struct StreamingBuffer {
    /// Buffered chunks
    chunks: VecDeque<String>,

    /// Configuration
    config: StreamingConfig,
}

impl StreamingBuffer {
    /// Create a new streaming buffer
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            chunks: VecDeque::with_capacity(config.max_buffer_size),
            config,
        }
    }

    /// Add a new chunk to the buffer
    pub fn push(&mut self, chunk: String) -> Result<()> {
        // Prevent unbounded growth
        if self.chunks.len() >= self.config.max_buffer_size {
            // Remove oldest chunk
            self.chunks.pop_front();
        }

        self.chunks.push_back(chunk);
        Ok(())
    }

    /// Get text for classification (current chunk + context)
    ///
    /// # Returns
    /// Concatenated string with context based on config:
    /// - If context_chunks = 0: entire buffer
    /// - If context_chunks = N: last N chunks
    pub fn get_context_text(&self) -> String {
        if self.chunks.is_empty() {
            return String::new();
        }

        let chunks_vec: Vec<&String> = if self.config.context_chunks == 0 {
            // Entire buffer
            self.chunks.iter().collect()
        } else {
            // Last N chunks
            let start = self.chunks.len().saturating_sub(self.config.context_chunks);
            self.chunks.iter().skip(start).collect()
        };

        chunks_vec
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(&self.config.chunk_delimiter)
    }

    /// Get just the current (most recent) chunk
    pub fn current_chunk(&self) -> Option<&str> {
        self.chunks.back().map(|s| s.as_str())
    }

    /// Get the number of chunks in buffer
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Clear all chunks
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Get configuration
    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }
}

/// Streaming classifier executor with context awareness
pub struct StreamingClassifier {
    /// The classifier or pipeline to use
    classifier: Arc<dyn Classifier>,

    /// Streaming buffer
    buffer: StreamingBuffer,
}

impl StreamingClassifier {
    /// Create a new streaming classifier
    pub fn new(classifier: Arc<dyn Classifier>, config: StreamingConfig) -> Self {
        Self {
            classifier,
            buffer: StreamingBuffer::new(config),
        }
    }

    /// Classify a new chunk with context
    ///
    /// This adds the chunk to the buffer and classifies using the configured
    /// context window (last N chunks or entire buffer).
    pub async fn classify_chunk(&mut self, chunk: String) -> Result<ClassificationResult> {
        // Add chunk to buffer
        self.buffer.push(chunk)?;

        // Get text with context
        let text = self.buffer.get_context_text();

        // Classify
        self.classifier.classify(&text).await
    }

    /// Get the current buffer
    pub fn buffer(&self) -> &StreamingBuffer {
        &self.buffer
    }

    /// Clear the buffer (e.g., start of new conversation)
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// Streaming pipeline executor with context awareness
pub struct StreamingPipeline {
    /// The pipeline to execute
    pipeline: ClassifierPipeline,

    /// Streaming buffer
    buffer: StreamingBuffer,
}

impl StreamingPipeline {
    /// Create a new streaming pipeline
    pub fn new(pipeline: ClassifierPipeline, config: StreamingConfig) -> Self {
        Self {
            pipeline,
            buffer: StreamingBuffer::new(config),
        }
    }

    /// Execute pipeline on new chunk with context
    pub async fn execute_chunk(&mut self, chunk: String) -> Result<PipelineExecutionResult> {
        // Add chunk to buffer
        self.buffer.push(chunk)?;

        // Get text with context
        let text = self.buffer.get_context_text();

        // Execute pipeline
        self.pipeline.execute(&text).await
    }

    /// Get the current buffer
    pub fn buffer(&self) -> &StreamingBuffer {
        &self.buffer
    }

    /// Clear the buffer (e.g., start of new conversation)
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_buffer_window() {
        let config = StreamingConfig::with_window(3);
        let mut buffer = StreamingBuffer::new(config);

        // Add chunks
        buffer.push("chunk1".to_string()).unwrap();
        buffer.push("chunk2".to_string()).unwrap();
        buffer.push("chunk3".to_string()).unwrap();
        buffer.push("chunk4".to_string()).unwrap();
        buffer.push("chunk5".to_string()).unwrap();

        // Should see last 3 chunks
        let context = buffer.get_context_text();
        assert_eq!(context, "chunk3 chunk4 chunk5");
    }

    #[test]
    fn test_streaming_buffer_entire() {
        let config = StreamingConfig::entire_buffer();
        let mut buffer = StreamingBuffer::new(config);

        buffer.push("chunk1".to_string()).unwrap();
        buffer.push("chunk2".to_string()).unwrap();
        buffer.push("chunk3".to_string()).unwrap();

        // Should see entire buffer
        let context = buffer.get_context_text();
        assert_eq!(context, "chunk1 chunk2 chunk3");
    }

    #[test]
    fn test_streaming_buffer_no_context() {
        let config = StreamingConfig::no_context();
        let mut buffer = StreamingBuffer::new(config);

        buffer.push("chunk1".to_string()).unwrap();
        buffer.push("chunk2".to_string()).unwrap();
        buffer.push("chunk3".to_string()).unwrap();

        // Should see only current chunk
        let context = buffer.get_context_text();
        assert_eq!(context, "chunk3");
    }

    #[test]
    fn test_streaming_buffer_max_size() {
        let config = StreamingConfig {
            context_chunks: 0,
            max_buffer_size: 3,
            chunk_delimiter: " ".to_string(),
        };
        let mut buffer = StreamingBuffer::new(config);

        // Add more than max
        for i in 1..=5 {
            buffer.push(format!("chunk{}", i)).unwrap();
        }

        // Should only have last 3
        assert_eq!(buffer.len(), 3);
        let context = buffer.get_context_text();
        assert_eq!(context, "chunk3 chunk4 chunk5");
    }

    #[test]
    fn test_current_chunk() {
        let mut buffer = StreamingBuffer::new(StreamingConfig::default());

        buffer.push("chunk1".to_string()).unwrap();
        buffer.push("chunk2".to_string()).unwrap();

        assert_eq!(buffer.current_chunk(), Some("chunk2"));
    }
}
