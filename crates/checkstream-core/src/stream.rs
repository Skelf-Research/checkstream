//! Stream processing utilities

use crate::{Error, Result, Token};
use std::collections::VecDeque;

/// Buffer for holding tokens during streaming with configurable holdback
#[derive(Debug)]
pub struct TokenBuffer {
    /// Buffered tokens
    tokens: VecDeque<Token>,

    /// Number of tokens to hold back for safety checks
    holdback_size: usize,

    /// Maximum buffer capacity
    max_capacity: usize,
}

impl TokenBuffer {
    /// Create a new token buffer
    ///
    /// # Arguments
    /// * `holdback_size` - Number of tokens to retain for lookahead analysis
    /// * `max_capacity` - Maximum buffer size before forcing flush
    pub fn new(holdback_size: usize, max_capacity: usize) -> Self {
        Self {
            tokens: VecDeque::with_capacity(max_capacity),
            holdback_size,
            max_capacity,
        }
    }

    /// Add a token to the buffer
    pub fn push(&mut self, token: Token) -> Result<()> {
        if self.tokens.len() >= self.max_capacity {
            return Err(Error::stream("buffer capacity exceeded"));
        }
        self.tokens.push_back(token);
        Ok(())
    }

    /// Get tokens that are ready to be released (beyond holdback)
    pub fn releasable(&self) -> impl Iterator<Item = &Token> {
        let releasable_count = self.tokens.len().saturating_sub(self.holdback_size);
        self.tokens.iter().take(releasable_count)
    }

    /// Drain and return tokens that are ready to be released
    pub fn drain_releasable(&mut self) -> Vec<Token> {
        let releasable_count = self.tokens.len().saturating_sub(self.holdback_size);
        self.tokens.drain(..releasable_count).collect()
    }

    /// Get all tokens in the buffer (for safety checks)
    pub fn all_tokens(&self) -> &VecDeque<Token> {
        &self.tokens
    }

    /// Get the current buffer length
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Flush all remaining tokens (e.g., at end of stream)
    pub fn flush(&mut self) -> Vec<Token> {
        self.tokens.drain(..).collect()
    }

    /// Get a window of the most recent N tokens
    pub fn window(&self, size: usize) -> impl Iterator<Item = &Token> {
        let start = self.tokens.len().saturating_sub(size);
        self.tokens.iter().skip(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_buffer_holdback() {
        let mut buffer = TokenBuffer::new(3, 100);

        // Add tokens
        for i in 0..5 {
            buffer.push(Token::new(format!("token{}", i))).unwrap();
        }

        // Should have 2 releasable (5 - 3 holdback)
        assert_eq!(buffer.releasable().count(), 2);

        // Drain releasable
        let released = buffer.drain_releasable();
        assert_eq!(released.len(), 2);
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_token_buffer_flush() {
        let mut buffer = TokenBuffer::new(3, 100);

        for i in 0..5 {
            buffer.push(Token::new(format!("token{}", i))).unwrap();
        }

        let all = buffer.flush();
        assert_eq!(all.len(), 5);
        assert!(buffer.is_empty());
    }
}
