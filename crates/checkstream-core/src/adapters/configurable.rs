//! Configurable Stream Adapter
//!
//! A generic adapter that can be configured to parse various streaming formats
//! using JSONPath-like content extraction.

use crate::stream_adapter::{ChunkMetadata, ParsedChunk, StreamAdapter};
use serde::{Deserialize, Serialize};

/// Configuration for a custom streaming format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Adapter name
    pub name: String,

    /// Wire format: "sse" or "ndjson"
    #[serde(default = "default_format")]
    pub format: String,

    /// Path to content field (dot-notation or simple JSONPath)
    /// Examples: "delta.text", "choices[0].delta.content", "data.message"
    pub content_path: String,

    /// Marker indicating stream end
    /// Examples: "[DONE]", "event: message_stop"
    #[serde(default)]
    pub done_marker: Option<String>,

    /// For SSE: event types that contain content (e.g., ["content_block_delta"])
    #[serde(default)]
    pub content_events: Vec<String>,

    /// Path to finish_reason field
    #[serde(default)]
    pub finish_reason_path: Option<String>,
}

fn default_format() -> String {
    "sse".to_string()
}

/// A configurable stream adapter
#[derive(Debug, Clone)]
pub struct ConfigurableAdapter {
    config: AdapterConfig,
}

impl ConfigurableAdapter {
    /// Create a new configurable adapter
    pub fn new(config: AdapterConfig) -> Self {
        Self { config }
    }

    /// Extract value from JSON using a simple path notation
    fn extract_path<'a>(
        &self,
        value: &'a serde_json::Value,
        path: &str,
    ) -> Option<&'a serde_json::Value> {
        let mut current = value;

        for part in path.split('.') {
            // Handle array index notation: "choices[0]"
            if let Some(bracket_pos) = part.find('[') {
                let key = &part[..bracket_pos];
                let index_str = &part[bracket_pos + 1..part.len() - 1];

                if !key.is_empty() {
                    current = current.get(key)?;
                }

                let index: usize = index_str.parse().ok()?;
                current = current.get(index)?;
            } else {
                current = current.get(part)?;
            }
        }

        Some(current)
    }

    /// Parse SSE format data
    fn parse_sse(&self, data: &str) -> Vec<ParsedChunk> {
        let mut chunks = Vec::new();
        let mut current_event: Option<String> = None;

        for line in data.lines() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            // Check done marker
            if let Some(ref marker) = self.config.done_marker {
                if line.contains(marker) {
                    chunks.push(ParsedChunk::done(None));
                    continue;
                }
            }

            // Parse event type
            if let Some(event) = line.strip_prefix("event:") {
                current_event = Some(event.trim().to_string());
                continue;
            }

            // Parse data line
            if let Some(json_str) = line.strip_prefix("data:") {
                let json_str = json_str.trim();

                if json_str.is_empty() {
                    continue;
                }

                // Check if this event type should be processed
                if !self.config.content_events.is_empty() {
                    if let Some(ref event) = current_event {
                        if !self.config.content_events.contains(event) {
                            // Pass through non-content events
                            chunks.push(ParsedChunk::PassThrough(json_str.to_string()));
                            continue;
                        }
                    }
                }

                // Parse JSON and extract content
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // Check for finish reason
                    if let Some(ref fr_path) = self.config.finish_reason_path {
                        if let Some(reason) = self.extract_path(&json, fr_path) {
                            if let Some(reason_str) = reason.as_str() {
                                if !reason_str.is_empty() {
                                    chunks.push(ParsedChunk::done(Some(reason_str.to_string())));
                                    continue;
                                }
                            }
                        }
                    }

                    // Extract content
                    if let Some(content) = self.extract_path(&json, &self.config.content_path) {
                        if let Some(text) = content.as_str() {
                            if !text.is_empty() {
                                chunks.push(ParsedChunk::Content {
                                    text: text.to_string(),
                                    metadata: ChunkMetadata {
                                        event_type: current_event.clone(),
                                        ..Default::default()
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }

        chunks
    }

    /// Parse NDJSON format data
    fn parse_ndjson(&self, data: &str) -> Vec<ParsedChunk> {
        let mut chunks = Vec::new();

        for line in data.lines() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            // Check done marker
            if let Some(ref marker) = self.config.done_marker {
                if line.contains(marker) {
                    chunks.push(ParsedChunk::done(None));
                    continue;
                }
            }

            // Parse JSON line
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                // Check for finish reason
                if let Some(ref fr_path) = self.config.finish_reason_path {
                    if let Some(reason) = self.extract_path(&json, fr_path) {
                        if let Some(reason_str) = reason.as_str() {
                            if !reason_str.is_empty() {
                                chunks.push(ParsedChunk::done(Some(reason_str.to_string())));
                                continue;
                            }
                        }
                    }
                }

                // Extract content
                if let Some(content) = self.extract_path(&json, &self.config.content_path) {
                    if let Some(text) = content.as_str() {
                        if !text.is_empty() {
                            chunks.push(ParsedChunk::content(text));
                        }
                    }
                }
            }
        }

        chunks
    }
}

impl StreamAdapter for ConfigurableAdapter {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn parse(&self, data: &str) -> Vec<ParsedChunk> {
        match self.config.format.as_str() {
            "ndjson" | "jsonl" => self.parse_ndjson(data),
            _ => self.parse_sse(data),
        }
    }

    fn is_done_marker(&self, data: &str) -> bool {
        if let Some(ref marker) = self.config.done_marker {
            data.contains(marker)
        } else {
            false
        }
    }

    fn content_type(&self) -> &str {
        match self.config.format.as_str() {
            "ndjson" | "jsonl" => "application/x-ndjson",
            _ => "text/event-stream",
        }
    }
}

// =============================================================================
// Pre-built Adapters
// =============================================================================

/// Create an Anthropic Messages API adapter
///
/// Anthropic format:
/// ```text
/// event: content_block_delta
/// data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}
///
/// event: message_stop
/// data: {"type": "message_stop"}
/// ```
pub fn anthropic_adapter() -> ConfigurableAdapter {
    ConfigurableAdapter::new(AdapterConfig {
        name: "anthropic".to_string(),
        format: "sse".to_string(),
        content_path: "delta.text".to_string(),
        done_marker: Some("message_stop".to_string()),
        content_events: vec!["content_block_delta".to_string()],
        finish_reason_path: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_adapter() {
        let adapter = anthropic_adapter();

        let data = r#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

event: message_stop
data: {"type":"message_stop"}"#;

        let chunks = adapter.parse(data);

        // Should have: Hello, world, done
        let content_chunks: Vec<_> = chunks.iter().filter(|c| c.is_content()).collect();
        assert_eq!(content_chunks.len(), 2);
        assert_eq!(content_chunks[0].text(), Some("Hello"));
        assert_eq!(content_chunks[1].text(), Some(" world"));

        // Should detect done
        assert!(chunks.iter().any(|c| c.is_done()));
    }

    #[test]
    fn test_ndjson_adapter() {
        let adapter = ConfigurableAdapter::new(AdapterConfig {
            name: "custom".to_string(),
            format: "ndjson".to_string(),
            content_path: "message.content".to_string(),
            done_marker: Some("\"done\":true".to_string()),
            content_events: vec![],
            finish_reason_path: None,
        });

        let data = r#"{"message":{"content":"Hello"}}
{"message":{"content":" world"}}
{"done":true}"#;

        let chunks = adapter.parse(data);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text(), Some("Hello"));
        assert_eq!(chunks[1].text(), Some(" world"));
        assert!(chunks[2].is_done());
    }

    #[test]
    fn test_path_extraction() {
        let adapter = ConfigurableAdapter::new(AdapterConfig {
            name: "test".to_string(),
            format: "ndjson".to_string(),
            content_path: "choices[0].delta.content".to_string(),
            done_marker: None,
            content_events: vec![],
            finish_reason_path: None,
        });

        let json: serde_json::Value =
            serde_json::from_str(r#"{"choices":[{"delta":{"content":"test"}}]}"#).unwrap();

        let value = adapter.extract_path(&json, "choices[0].delta.content");
        assert_eq!(value.unwrap().as_str(), Some("test"));
    }
}
