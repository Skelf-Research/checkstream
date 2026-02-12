# checkstream-classifiers

Classifier and pipeline components for toxicity, PII, prompt injection, and sentiment detection.

[![Crates.io](https://img.shields.io/crates/v/checkstream-classifiers.svg)](https://crates.io/crates/checkstream-classifiers)
[![Documentation](https://docs.rs/checkstream-classifiers/badge.svg)](https://docs.rs/checkstream-classifiers)
[![License](https://img.shields.io/crates/l/checkstream-classifiers.svg)](https://github.com/Skelf-Research/checkstream/blob/main/LICENSE)

## Overview

`checkstream-classifiers` provides high-performance content classification for LLM guardrails. It includes built-in dependency-light classifiers and registry-driven sequence classifiers for deterministic real-time content analysis.

## Features

- **Registry-Driven Sequence Classifiers** - Configurable BERT/DistilBERT/RoBERTa/XLM-R/MiniLM/DeBERTa interfaces
- **Pattern Classifiers** - Regex and Aho-Corasick based detection
- **PII Detection** - SSN, credit cards, emails, phone numbers
- **Toxicity Detection** - Multi-label toxicity classification
- **Prompt Injection** - Detect and block injection attempts
- **Sub-millisecond Patterns** - ~0.5ms for pattern classifiers
- **Deterministic Runtime** - No heavyweight runtime ML dependencies required

## Installation

```toml
[dependencies]
checkstream-classifiers = "0.1"

# Enable extended model-registry paths
checkstream-classifiers = { version = "0.1", features = ["ml-models"] }

# Hardened default profile
checkstream-classifiers = { version = "0.1", default-features = false }

# Optional: true external model inference backend
checkstream-classifiers-ml-plugin = { path = "../../plugins/checkstream-classifiers-ml-plugin" }
```

## Usage

### Pattern Classifier

```rust
use checkstream_classifiers::{PatternClassifier, PatternConfig};

// Create a PII detector
let config = PatternConfig {
    name: "ssn-detector".to_string(),
    patterns: vec![
        r"\b\d{3}-\d{2}-\d{4}\b".to_string(),  // SSN format
    ],
    label: "pii-ssn".to_string(),
};

let classifier = PatternClassifier::new(config)?;
let result = classifier.classify("My SSN is 123-45-6789").await?;

println!("Detected: {} (score: {})", result.label, result.score);
```

### Sequence Classifier

```rust
use checkstream_classifiers::{GenericModelLoader, ModelRegistry};

// Load from YAML configuration
let registry: ModelRegistry = serde_yaml::from_str(r#"
version: "1.0"
models:
  sentiment:
    name: "distilbert-sst2"
    source:
      type: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
    architecture:
      type: distil-bert-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"
      max_length: 512
"#)?;

let loader = GenericModelLoader::new(registry);
let classifier = loader.load_classifier("sentiment").await?;

let result = classifier.classify("I love this product!").await?;
println!("{}: {} ({:.2})", result.label, result.score, result.latency_us);
```

### External ML Plugin (True Inference)

```rust
use checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder;
use checkstream_classifiers_ml_plugin::ExternalMlModelLoader;
use std::sync::Arc;

let loader = ExternalMlModelLoader::from_file("models/registry.yaml")?;

let registry = DynamicRegistryBuilder::new()
    .with_loader(Arc::new(loader))
    .preload("sentiment")
    .build()
    .await?;
```

### Classifier Pipeline

```rust
use checkstream_classifiers::{ClassifierPipeline, Classifier};

let mut pipeline = ClassifierPipeline::new();

// Add classifiers in order
pipeline.add(Box::new(pii_classifier));
pipeline.add(Box::new(toxicity_classifier));
pipeline.add(Box::new(injection_classifier));

// Run all classifiers
let results = pipeline.classify_all("User input text").await?;

for result in results {
    println!("{}: {}", result.label, result.score);
}
```

## Built-in Classifiers

### Pattern-Based (Tier A: <2ms)

| Classifier | Description |
|------------|-------------|
| `pii-ssn` | US Social Security Numbers |
| `pii-credit-card` | Credit card numbers (Luhn validated) |
| `pii-email` | Email addresses |
| `pii-phone` | Phone numbers |
| `prompt-injection` | Common injection patterns |

### Sequence Classifiers (Tier B)

| Model | Description |
|-------|-------------|
| `toxicity` | Multi-label toxicity (lexicon-based) |
| `sentiment` | Positive/negative sentiment |
| `custom` | Custom registry-driven sequence classifier |

## Configuration

### classifiers.yaml

```yaml
version: "1.0"

patterns:
  - name: "ssn-detector"
    patterns:
      - '\b\d{3}-\d{2}-\d{4}\b'
    label: "pii-ssn"

  - name: "credit-card"
    patterns:
      - '\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b'
    label: "pii-credit-card"

models:
  toxicity:
    source:
      type: huggingface
      repo: "unitary/toxic-bert"
    architecture:
      type: bert-sequence-classification
      num_labels: 6
```

## Performance

| Classifier Type | Latency |
|-----------------|---------|
| Pattern (regex) | ~0.5ms |
| Pattern (Aho-Corasick) | ~0.2ms |
| Sequence (Tier B) | ~0.5-2ms |

## Documentation

- [Full Documentation](https://docs.skelfresearch.com/checkstream)
- [Model Loading Guide](https://docs.skelfresearch.com/checkstream/models)
- [API Reference](https://docs.rs/checkstream-classifiers)
- [GitHub Repository](https://github.com/Skelf-Research/checkstream)

## License

Apache-2.0 - See [LICENSE](https://github.com/Skelf-Research/checkstream/blob/main/LICENSE) for details.

## Part of CheckStream

This crate is part of the [CheckStream](https://github.com/Skelf-Research/checkstream) guardrail platform by [Skelf Research](https://skelfresearch.com).
