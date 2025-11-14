# Streaming Classification with Context Windows

CheckStream's streaming classifier system supports configurable context windows, allowing classifiers to see previous chunks for better context-aware detection.

## The Problem

When classifying streaming text chunk-by-chunk, you face a trade-off:

```
❌ No Context (each chunk alone):
   "I recommend" → ✓ OK (innocent)
   "putting all" → ✓ OK (innocent)
   "your money" → ✓ OK (innocent)
   "into Bitcoin" → ✓ OK (innocent)

   Result: Miss the problematic advice! ❌

✅ With Context (sees previous chunks):
   "I recommend putting all your money into Bitcoin"
   → ⚠️ FLAGGED (personalized investment advice)

   Result: Correctly detected! ✅
```

## Solution: Configurable Context Windows

CheckStream provides three context strategies:

### 1. No Context (Fastest)
```rust
let config = StreamingConfig::no_context();
// Only sees current chunk
```

**Best for:**
- Token-level detection (PII, profanity)
- Ultra-low latency (<2ms)
- Independent checks

### 2. Sliding Window (Balanced)
```rust
let config = StreamingConfig::with_window(5);
// Sees last 5 chunks
```

**Best for:**
- Sentence-level analysis
- Short-term context
- Good balance of speed and accuracy

### 3. Entire Buffer (Most Context)
```rust
let config = StreamingConfig::entire_buffer();
// Sees all previous chunks (context_chunks = 0)
```

**Best for:**
- Full conversation analysis
- Compliance checks
- Multi-turn attack detection

## API Usage

### Basic Usage

```rust
use checkstream_classifiers::{StreamingClassifier, StreamingConfig};

// Create streaming classifier with 3-chunk window
let config = StreamingConfig::with_window(3);
let mut streaming = StreamingClassifier::new(my_classifier, config);

// Process chunks as they arrive
for chunk in stream {
    let result = streaming.classify_chunk(chunk).await?;

    if result.score > threshold {
        // Block or redact
    }
}
```

### With Pipelines

```rust
use checkstream_classifiers::{StreamingPipeline, StreamingConfig};

// Use entire buffer for compliance pipeline
let config = StreamingConfig::entire_buffer();
let mut streaming = StreamingPipeline::new(my_pipeline, config);

for chunk in stream {
    let result = streaming.execute_chunk(chunk).await?;

    // Check pipeline results
    if let Some(decision) = result.final_decision {
        if decision.score > 0.7 {
            redact_and_stop();
        }
    }
}
```

### Configuration Options

```rust
pub struct StreamingConfig {
    /// Number of previous chunks to include
    /// - 0 = entire buffer (all chunks)
    /// - N = last N chunks
    pub context_chunks: usize,

    /// Maximum buffer size (prevents unbounded growth)
    pub max_buffer_size: usize,

    /// Delimiter to join chunks (usually " ")
    pub chunk_delimiter: String,
}

// Custom configuration
let config = StreamingConfig {
    context_chunks: 10,           // Last 10 chunks
    max_buffer_size: 100,         // Max 100 chunks total
    chunk_delimiter: " ".to_string(),
};
```

## Phase-Specific Recommendations

### Phase 1: Ingress (Pre-Generation)

**Not Applicable** - Ingress validates the entire prompt at once, no streaming yet.

### Phase 2: Midstream (Streaming Checks)

#### Fast Path (Ultra-Low Latency)
```rust
// For per-chunk checks that must be <3ms
let config = StreamingConfig::no_context(); // or with_window(1)

// Use cases:
// - PII detection in current chunk
// - Single-word profanity
// - Token-level toxicity
```

#### Balanced Path (Good Detection)
```rust
// For checks that can afford 5-8ms
let config = StreamingConfig::with_window(5);

// Use cases:
// - Sentence-level toxicity
// - Short-term topic tracking
// - Recent context matters
```

#### Thorough Path (Best Detection)
```rust
// For important checks, can afford 10-15ms
let config = StreamingConfig::with_window(10);

// Use cases:
// - Advice vs. information detection
// - Compliance monitoring
// - Multi-sentence analysis
```

### Phase 3: Egress (Post-Generation)

```rust
// Use entire buffer for comprehensive analysis
let config = StreamingConfig::entire_buffer();

// Use cases:
// - Full conversation compliance check
// - Complete audit trail
// - Regulatory review
// - No latency pressure (async)
```

## Real-World Example: FCA Compliance

### Fast Midstream Checks (Phase 2)
```rust
// Fast path: Check each chunk with minimal context
let fast_config = StreamingConfig::with_window(3);
let mut fast_check = StreamingPipeline::new(
    fast_pipeline,  // Ultra-fast classifiers only
    fast_config
);

for chunk in llm_stream {
    let result = fast_check.execute_chunk(chunk.clone()).await?;

    if result.final_decision.map_or(false, |d| d.score > 0.8) {
        // High confidence issue, redact immediately
        send_to_user("[REDACTED]");
        break;
    }

    send_to_user(chunk);
}
```

### Comprehensive Egress Analysis (Phase 3)
```rust
// Egress: Analyze entire conversation
let full_config = StreamingConfig::entire_buffer();
let mut full_check = StreamingPipeline::new(
    compliance_pipeline,  // All tiers, thorough
    full_config
);

// Replay all chunks for full analysis
for chunk in all_chunks {
    full_check.execute_chunk(chunk).await?;
}

// Final result sees entire conversation
let result = full_check.buffer().get_context_text();

// Generate audit trail with full context
create_audit_record(&result)?;
```

## Performance Considerations

### Latency vs. Context Trade-off

```
Context Window Size → Latency Impact

No Context (1 chunk):
  Text Length: ~10 chars
  Latency: ~1-2ms

Small Window (3 chunks):
  Text Length: ~30 chars
  Latency: ~2-4ms

Medium Window (10 chunks):
  Text Length: ~100 chars
  Latency: ~4-8ms

Entire Buffer (unlimited):
  Text Length: ~1000+ chars
  Latency: ~10-50ms (grows with conversation)
```

### Memory Considerations

```rust
// Prevent unbounded memory growth
let config = StreamingConfig {
    context_chunks: 0,      // Entire buffer
    max_buffer_size: 100,   // But cap at 100 chunks
    ..Default::default()
};

// When buffer is full, oldest chunks are removed
// Maintains sliding window of last 100 chunks
```

## Advanced Patterns

### Adaptive Context

```rust
// Start with small window, expand if needed
let mut streaming = StreamingClassifier::new(
    classifier,
    StreamingConfig::with_window(3)
);

for chunk in stream {
    let result = streaming.classify_chunk(chunk).await?;

    if result.score > 0.6 {
        // Suspicious, expand context for better analysis
        streaming = StreamingClassifier::new(
            classifier,
            StreamingConfig::entire_buffer()
        );

        // Re-analyze with full context
        // ...
    }
}
```

### Multi-Tier Strategy

```rust
// Tier 1: Fast, no context
let tier1 = StreamingClassifier::new(
    fast_classifier,
    StreamingConfig::no_context()
);

// Tier 2: Slower, with context
let tier2 = StreamingClassifier::new(
    thorough_classifier,
    StreamingConfig::entire_buffer()
);

for chunk in stream {
    // Always run fast check
    let fast_result = tier1.classify_chunk(chunk.clone()).await?;

    if fast_result.score > 0.5 {
        // Run thorough check with full context
        let thorough_result = tier2.classify_chunk(chunk).await?;

        if thorough_result.score > 0.8 {
            block();
        }
    }
}
```

### Buffer Reset Strategy

```rust
let mut streaming = StreamingClassifier::new(
    classifier,
    StreamingConfig::entire_buffer()
);

for message in conversation {
    if message.role == "user" {
        // Reset context at conversation boundaries
        streaming.reset();
    }

    for chunk in message.chunks {
        streaming.classify_chunk(chunk).await?;
    }
}
```

## Complete Example

See [`examples/streaming_context.rs`](../examples/streaming_context.rs) for a complete working example demonstrating:

- All three context strategies
- Side-by-side comparison
- Why context matters for detection
- Phase-specific recommendations
- Use case guidelines

Run it:
```bash
cargo run --example streaming_context
```

## Configuration Reference

### Pre-defined Configs

```rust
// No context (fastest)
StreamingConfig::no_context()
// → context_chunks: 1
// → max_buffer_size: 10

// Small window
StreamingConfig::with_window(5)
// → context_chunks: 5
// → max_buffer_size: 100

// Entire buffer
StreamingConfig::entire_buffer()
// → context_chunks: 0
// → max_buffer_size: 1000
```

### Custom Config

```rust
let config = StreamingConfig {
    context_chunks: 10,           // Last 10 chunks
    max_buffer_size: 50,          // Cap at 50 total
    chunk_delimiter: "\n".to_string(),  // Join with newlines
};
```

## Best Practices

### 1. Choose Based on What You're Detecting

```rust
// Token-level → No context
let pii_config = StreamingConfig::no_context();

// Sentence-level → Small window
let toxicity_config = StreamingConfig::with_window(5);

// Conversation-level → Entire buffer
let compliance_config = StreamingConfig::entire_buffer();
```

### 2. Consider Your Latency Budget

```rust
// Critical path (<3ms) → No context or tiny window
let critical = StreamingConfig::with_window(2);

// Standard path (<10ms) → Medium window
let standard = StreamingConfig::with_window(8);

// Async path → Entire buffer
let async_check = StreamingConfig::entire_buffer();
```

### 3. Reset at Boundaries

```rust
// Reset between different conversations
if new_conversation {
    streaming_classifier.reset();
}

// Or reset between user messages
if message.role == "user" {
    streaming_classifier.reset();
}
```

### 4. Monitor Buffer Size

```rust
// Log when buffer grows large
let buffer_size = streaming.buffer().len();
if buffer_size > 50 {
    warn!("Large streaming buffer: {} chunks", buffer_size);
}
```

## Integration with Configuration

### YAML Configuration (Future Enhancement)

```yaml
# Future: Configure streaming behavior per pipeline
pipelines:
  midstream-fast:
    streaming:
      context_chunks: 1          # No context
      max_buffer_size: 10

    stages:
      - type: parallel
        classifiers: [pii, profanity]

  midstream-thorough:
    streaming:
      context_chunks: 10         # Last 10 chunks
      max_buffer_size: 100

    stages:
      - type: parallel
        classifiers: [advice-detector, risk-disclosure]

  egress-compliance:
    streaming:
      context_chunks: 0          # Entire buffer
      max_buffer_size: 1000

    stages:
      - type: sequential
        classifiers: [full-compliance-check]
```

## See Also

- [Pipeline Configuration](pipeline-configuration.md) - General pipeline system
- [Integration Guide](INTEGRATION_GUIDE.md) - Integrating into proxy
- [FCA Example](FCA_EXAMPLE.md) - Real-world regulatory example
- [Example Code](../examples/streaming_context.rs) - Complete working example
