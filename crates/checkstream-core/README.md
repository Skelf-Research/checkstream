# checkstream-core

Core types, token buffer, and error handling for the CheckStream guardrail platform.

[![Crates.io](https://img.shields.io/crates/v/checkstream-core.svg)](https://crates.io/crates/checkstream-core)
[![Documentation](https://docs.rs/checkstream-core/badge.svg)](https://docs.rs/checkstream-core)
[![License](https://img.shields.io/crates/l/checkstream-core.svg)](https://github.com/skelf-research/checkstream/blob/main/LICENSE)

## Overview

`checkstream-core` provides the foundational types and utilities used across all CheckStream crates:

- **Token Buffer** - Efficient streaming token accumulation with configurable holdback
- **Core Types** - Classification results, actions, and pipeline stage definitions
- **Error Handling** - Unified error types with rich context
- **Hashing Utilities** - SHA-256 based content hashing for audit trails

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
checkstream-core = "0.1"
```

## Usage

```rust
use checkstream_core::{TokenBuffer, ClassificationResult, Action};

// Create a token buffer with 10-token holdback
let mut buffer = TokenBuffer::new(10);

// Add streaming tokens
buffer.push("Hello");
buffer.push(" world");

// Get accumulated content for classification
let content = buffer.content();
```

## Features

### Token Buffer

The `TokenBuffer` provides efficient streaming token management:

- Configurable holdback window for streaming classification
- Memory-efficient token accumulation
- Thread-safe operations

### Classification Results

Standardized result types for all classifiers:

```rust
use checkstream_core::ClassificationResult;

let result = ClassificationResult {
    label: "positive".to_string(),
    score: 0.95,
    latency_us: 1200,
    metadata: Default::default(),
};
```

### Actions

Defines actions that can be taken based on policy evaluation:

- `Stop` - Halt the stream immediately
- `Redact` - Remove or mask sensitive content
- `Log` - Record the event for auditing
- `Audit` - Add to tamper-proof audit trail

## Documentation

- [Full Documentation](https://docs.skelfresearch.com/checkstream)
- [API Reference](https://docs.rs/checkstream-core)
- [GitHub Repository](https://github.com/skelf-research/checkstream)

## License

Apache-2.0 - See [LICENSE](https://github.com/skelf-research/checkstream/blob/main/LICENSE) for details.

## Part of CheckStream

This crate is part of the [CheckStream](https://github.com/skelf-research/checkstream) guardrail platform by [Skelf Research](https://skelfresearch.com).
