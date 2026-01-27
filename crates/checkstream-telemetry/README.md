# checkstream-telemetry

Hash-chained audit trail and Prometheus metrics for CheckStream.

[![Crates.io](https://img.shields.io/crates/v/checkstream-telemetry.svg)](https://crates.io/crates/checkstream-telemetry)
[![Documentation](https://docs.rs/checkstream-telemetry/badge.svg)](https://docs.rs/checkstream-telemetry)
[![License](https://img.shields.io/crates/l/checkstream-telemetry.svg)](https://github.com/skelf-research/checkstream/blob/main/LICENSE)

## Overview

`checkstream-telemetry` provides tamper-proof audit logging and observability for the CheckStream guardrail platform. It implements a hash-chained audit trail for regulatory compliance and Prometheus metrics for monitoring.

## Features

- **Hash-Chained Audit Trail** - Tamper-evident logging with SHA-256 chains
- **Prometheus Metrics** - Built-in metrics exporter
- **Structured Logging** - JSON-formatted audit events
- **Compliance Ready** - Designed for regulatory requirements (FCA, SOC2, etc.)
- **Query API** - Search and filter audit events

## Installation

```toml
[dependencies]
checkstream-telemetry = "0.1"
```

## Usage

### Audit Trail

```rust
use checkstream_telemetry::{AuditTrail, AuditEvent, Severity};

// Create audit trail
let audit = AuditTrail::new("./audit.log")?;

// Record an event
let event = AuditEvent {
    timestamp: chrono::Utc::now(),
    event_type: "policy_violation".to_string(),
    severity: Severity::High,
    rule_name: "block-toxic-content".to_string(),
    content_hash: "sha256:abc123...".to_string(),
    action_taken: "stop".to_string(),
    metadata: serde_json::json!({
        "classifier": "toxicity",
        "score": 0.92,
        "request_id": "req-12345"
    }),
};

audit.record(event).await?;
```

### Hash Chain Verification

Each audit entry is linked to the previous entry via SHA-256 hash, creating a tamper-evident chain:

```rust
// Verify audit trail integrity
let is_valid = audit.verify_chain().await?;

if !is_valid {
    eprintln!("Audit trail has been tampered with!");
}
```

### Query Audit Events

```rust
use checkstream_telemetry::{AuditQuery, TimeRange};

let query = AuditQuery {
    time_range: TimeRange::last_hours(24),
    severity: Some(Severity::High),
    event_type: Some("policy_violation".to_string()),
    limit: 100,
};

let events = audit.query(query).await?;

for event in events {
    println!("{}: {} - {}", event.timestamp, event.event_type, event.action_taken);
}
```

### Prometheus Metrics

```rust
use checkstream_telemetry::MetricsRegistry;

let metrics = MetricsRegistry::new();

// Record classifier latency
metrics.record_classifier_latency("toxicity", 45.2);

// Record policy evaluation
metrics.record_policy_evaluation("block-toxic", true);

// Increment request counter
metrics.increment_requests("chat_completions");
```

## Metrics Exported

| Metric | Type | Description |
|--------|------|-------------|
| `checkstream_requests_total` | Counter | Total requests processed |
| `checkstream_classifier_latency_ms` | Histogram | Classifier execution time |
| `checkstream_policy_evaluations_total` | Counter | Policy evaluations by rule |
| `checkstream_policy_violations_total` | Counter | Policy violations by rule |
| `checkstream_actions_total` | Counter | Actions taken by type |
| `checkstream_audit_events_total` | Counter | Audit events by severity |

## Audit Event Schema

```json
{
  "id": "evt_abc123",
  "timestamp": "2024-01-15T10:30:00Z",
  "previous_hash": "sha256:def456...",
  "event_type": "policy_violation",
  "severity": "high",
  "rule_name": "block-toxic-content",
  "content_hash": "sha256:789xyz...",
  "action_taken": "stop",
  "metadata": {
    "classifier": "toxicity",
    "score": 0.92,
    "request_id": "req-12345",
    "user_id": "user-67890"
  },
  "hash": "sha256:current..."
}
```

## Configuration

```yaml
telemetry:
  audit:
    enabled: true
    path: "./audit/checkstream.log"
    rotation: daily
    retention_days: 90

  metrics:
    enabled: true
    endpoint: "/metrics"

  logging:
    level: info
    format: json
```

## Documentation

- [Full Documentation](https://docs.skelfresearch.com/checkstream)
- [Compliance Guide](https://docs.skelfresearch.com/checkstream/compliance)
- [API Reference](https://docs.rs/checkstream-telemetry)
- [GitHub Repository](https://github.com/skelf-research/checkstream)

## License

Apache-2.0 - See [LICENSE](https://github.com/skelf-research/checkstream/blob/main/LICENSE) for details.

## Part of CheckStream

This crate is part of the [CheckStream](https://github.com/skelf-research/checkstream) guardrail platform by [Skelf Research](https://skelfresearch.com).
