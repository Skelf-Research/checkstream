//! OpenAI SSE Stream Adapter
//!
//! Parses the OpenAI chat completions streaming format:
//! ```text
//! data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"token"},"finish_reason":null}]}
//!
//! data: [DONE]
//! ```

use crate::stream_adapter::{ChunkMetadata, ParsedChunk, StreamAdapter};
use serde::Deserialize;

/// OpenAI SSE stream adapter
#[derive(Debug, Clone)]
pub struct OpenAiAdapter {
    /// Whether to preserve tool call events (pass-through)
    preserve_tool_calls: bool,
}

impl OpenAiAdapter {
    /// Create a new OpenAI adapter with default settings
    pub fn new() -> Self {
        Self {
            preserve_tool_calls: true,
        }
    }

    /// Create with custom settings
    pub fn with_settings(preserve_tool_calls: bool) -> Self {
        Self {
            preserve_tool_calls,
        }
    }

    /// Parse a single SSE data line
    fn parse_data_line(&self, json_str: &str) -> ParsedChunk {
        // Try to parse as OpenAI chunk
        match serde_json::from_str::<OpenAiChunk>(json_str) {
            Ok(chunk) => {
                // Extract content from first choice
                if let Some(choice) = chunk.choices.first() {
                    // Check for finish reason first
                    if let Some(ref reason) = choice.finish_reason {
                        return ParsedChunk::Done {
                            finish_reason: Some(reason.clone()),
                            metadata: Some(serde_json::json!({
                                "id": chunk.id,
                                "model": chunk.model,
                            })),
                        };
                    }

                    // Check for content
                    if let Some(ref content) = choice.delta.content {
                        if !content.is_empty() {
                            return ParsedChunk::Content {
                                text: content.clone(),
                                metadata: ChunkMetadata {
                                    id: chunk.id.clone(),
                                    model: chunk.model.clone(),
                                    index: choice.index,
                                    created: chunk.created,
                                    event_type: None,
                                },
                            };
                        }
                    }

                    // Check for tool calls (pass-through)
                    if self.preserve_tool_calls && choice.delta.tool_calls.is_some() {
                        return ParsedChunk::PassThrough(json_str.to_string());
                    }
                }

                ParsedChunk::Empty
            }
            Err(e) => ParsedChunk::Error(format!("Failed to parse OpenAI chunk: {}", e)),
        }
    }
}

impl Default for OpenAiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamAdapter for OpenAiAdapter {
    fn name(&self) -> &str {
        "openai"
    }

    fn parse(&self, data: &str) -> Vec<ParsedChunk> {
        let mut chunks = Vec::new();

        for line in data.lines() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for done marker
            if self.is_done_marker(line) {
                chunks.push(ParsedChunk::done(None));
                continue;
            }

            // Parse SSE data lines
            if let Some(json_str) = line.strip_prefix("data:") {
                let json_str = json_str.trim();

                // Skip [DONE] marker (already handled above but could appear after data:)
                if json_str == "[DONE]" {
                    chunks.push(ParsedChunk::done(None));
                    continue;
                }

                // Skip empty data
                if json_str.is_empty() {
                    continue;
                }

                chunks.push(self.parse_data_line(json_str));
            }
        }

        chunks
    }

    fn is_done_marker(&self, data: &str) -> bool {
        let trimmed = data.trim();
        trimmed == "data: [DONE]" || trimmed == "[DONE]"
    }

    fn content_type(&self) -> &str {
        "text/event-stream"
    }
}

// =============================================================================
// OpenAI Response Structures
// =============================================================================

#[derive(Debug, Deserialize)]
struct OpenAiChunk {
    id: Option<String>,
    // object: String, // "chat.completion.chunk"
    created: Option<u64>,
    model: Option<String>,
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    #[serde(default)]
    index: usize,
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAiDelta {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<serde_json::Value>,
    // role: Option<String>, // Usually only in first chunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_chunk() {
        let adapter = OpenAiAdapter::new();

        let data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;

        let chunks = adapter.parse(data);
        assert_eq!(chunks.len(), 1);

        match &chunks[0] {
            ParsedChunk::Content { text, metadata } => {
                assert_eq!(text, "Hello");
                assert_eq!(metadata.model.as_deref(), Some("gpt-4"));
            }
            _ => panic!("Expected Content chunk"),
        }
    }

    #[test]
    fn test_parse_finish_chunk() {
        let adapter = OpenAiAdapter::new();

        let data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;

        let chunks = adapter.parse(data);
        assert_eq!(chunks.len(), 1);

        match &chunks[0] {
            ParsedChunk::Done { finish_reason, .. } => {
                assert_eq!(finish_reason.as_deref(), Some("stop"));
            }
            _ => panic!("Expected Done chunk"),
        }
    }

    #[test]
    fn test_parse_done_marker() {
        let adapter = OpenAiAdapter::new();

        let data = "data: [DONE]";
        let chunks = adapter.parse(data);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_done());
    }

    #[test]
    fn test_is_done_marker() {
        let adapter = OpenAiAdapter::new();

        assert!(adapter.is_done_marker("data: [DONE]"));
        assert!(adapter.is_done_marker("[DONE]"));
        assert!(!adapter.is_done_marker("data: {\"test\": true}"));
    }

    #[test]
    fn test_parse_multiple_events() {
        let adapter = OpenAiAdapter::new();

        let data = r#"data: {"id":"1","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"1","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: [DONE]"#;

        let chunks = adapter.parse(data);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text(), Some("Hello"));
        assert_eq!(chunks[1].text(), Some(" world"));
        assert!(chunks[2].is_done());
    }
}
