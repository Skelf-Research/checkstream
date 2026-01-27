# checkstream-policy

YAML-based policy engine for LLM safety rules, triggers, and actions.

[![Crates.io](https://img.shields.io/crates/v/checkstream-policy.svg)](https://crates.io/crates/checkstream-policy)
[![Documentation](https://docs.rs/checkstream-policy/badge.svg)](https://docs.rs/checkstream-policy)
[![License](https://img.shields.io/crates/l/checkstream-policy.svg)](https://github.com/Skelf-Research/checkstream/blob/main/LICENSE)

## Overview

`checkstream-policy` provides a flexible policy engine for defining and enforcing safety rules on LLM outputs. Policies are defined in YAML and support complex trigger conditions with configurable actions.

## Features

- **YAML Configuration** - Human-readable policy definitions
- **Flexible Triggers** - Combine classifiers with AND/OR logic
- **Multiple Actions** - Stop, Redact, Log, Audit, and custom actions
- **Pipeline Stages** - Ingress, Midstream, and Egress evaluation points
- **Hot Reload** - Update policies without restarting

## Installation

```toml
[dependencies]
checkstream-policy = "0.1"
```

## Usage

### Define a Policy

```yaml
# policies/default.yaml
version: "1.0"
name: "default-safety"
description: "Default safety policy"

rules:
  - name: "block-toxic-content"
    description: "Block toxic or harmful content"
    stage: egress
    trigger:
      classifier: "toxicity"
      threshold: 0.8
    actions:
      - type: stop
        message: "Content blocked due to toxicity"
      - type: audit
        severity: high

  - name: "redact-pii"
    description: "Redact personally identifiable information"
    stage: egress
    trigger:
      classifier: "pii-detector"
      threshold: 0.5
    actions:
      - type: redact
        replacement: "[REDACTED]"
      - type: log
        level: warn
```

### Load and Evaluate

```rust
use checkstream_policy::{PolicyEngine, PolicyConfig};

// Load policy from file
let config = PolicyConfig::from_file("policies/default.yaml")?;
let engine = PolicyEngine::new(config);

// Evaluate content against policy
let results = engine.evaluate(&classification_results, Stage::Egress).await?;

for action in results.actions {
    match action {
        Action::Stop { message } => println!("Blocked: {}", message),
        Action::Redact { pattern, replacement } => println!("Redacting..."),
        Action::Log { level, message } => println!("[{}] {}", level, message),
        Action::Audit { event } => println!("Audit: {:?}", event),
    }
}
```

## Policy Structure

### Rules

Each rule consists of:

- **name** - Unique identifier
- **description** - Human-readable description
- **stage** - When to evaluate (ingress, midstream, egress)
- **trigger** - Conditions that activate the rule
- **actions** - What to do when triggered

### Triggers

Simple trigger:
```yaml
trigger:
  classifier: "toxicity"
  threshold: 0.8
```

Compound trigger (AND):
```yaml
trigger:
  all:
    - classifier: "toxicity"
      threshold: 0.7
    - classifier: "sentiment"
      label: "negative"
```

Compound trigger (OR):
```yaml
trigger:
  any:
    - classifier: "pii-ssn"
      threshold: 0.5
    - classifier: "pii-credit-card"
      threshold: 0.5
```

### Actions

| Action | Description |
|--------|-------------|
| `stop` | Halt stream with message |
| `redact` | Replace matched content |
| `log` | Write to structured log |
| `audit` | Record in audit trail |
| `webhook` | Call external endpoint |

## Built-in Policy Packs

CheckStream includes pre-built policy packs:

- **default.yaml** - Basic safety (PII, toxicity, prompt injection)
- **fca-consumer-duty.yaml** - UK FCA financial compliance

## Documentation

- [Full Documentation](https://docs.skelfresearch.com/checkstream)
- [Policy Guide](https://docs.skelfresearch.com/checkstream/policies)
- [API Reference](https://docs.rs/checkstream-policy)
- [GitHub Repository](https://github.com/Skelf-Research/checkstream)

## License

Apache-2.0 - See [LICENSE](https://github.com/Skelf-Research/checkstream/blob/main/LICENSE) for details.

## Part of CheckStream

This crate is part of the [CheckStream](https://github.com/Skelf-Research/checkstream) guardrail platform by [Skelf Research](https://skelfresearch.com).
