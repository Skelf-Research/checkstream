# CheckStream Architecture

## Overview

CheckStream is a high-performance streaming guardrail system built in Rust, designed to enforce safety, security, and regulatory compliance on LLM outputs with sub-10ms latency overhead.

## Design Principles

1. **Performance First**: Every component designed for <10ms total latency
2. **Zero-Copy Where Possible**: Minimize allocations in hot paths
3. **Async Throughout**: Tokio runtime for maximum concurrency
4. **Type Safety**: Leverage Rust's type system for correctness
5. **Modular Architecture**: Clean separation of concerns across crates

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Client                               │
└───────────────────────────┬─────────────────────────────────┘
                            │ HTTP/SSE Request
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  CheckStream Proxy                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Ingress Stage (Pre-Generation)         [2-8ms]      │  │
│  │  - Prompt validation                                  │  │
│  │  - PII detection in user input                        │  │
│  │  - Policy pre-checks                                  │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│                            ▼                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Proxy to Backend LLM                                 │  │
│  │  - Forward request to OpenAI/Anthropic/etc.           │  │
│  │  - Establish SSE stream                               │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│                            ▼                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Midstream Stage (During Generation)    [3-6ms/chunk] │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │ Token Buffer (Holdback Queue)                   │  │  │
│  │  │ - Sliding window of N tokens                    │  │  │
│  │  │ - Release tokens beyond holdback                │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │                      │                                  │  │
│  │                      ▼                                  │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │ Classifier Pipeline (Parallel)                  │  │  │
│  │  │ ┌────────────┐ ┌────────────┐ ┌─────────────┐  │  │  │
│  │  │ │ Tier A     │ │ Tier A     │ │ Tier B      │  │  │  │
│  │  │ │ PII        │ │ Patterns   │ │ Toxicity    │  │  │  │
│  │  │ │ <2ms       │ │ <2ms       │ │ <5ms        │  │  │  │
│  │  │ └────────────┘ └────────────┘ └─────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │                      │                                  │  │
│  │                      ▼                                  │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │ Policy Engine                                   │  │  │
│  │  │ - Evaluate triggers against classifier results  │  │  │
│  │  │ - Execute actions (redact/stop/inject/log)      │  │  │
│  │  │ - Record audit events                           │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│                            ▼                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Egress Stage (Finalization)                          │  │
│  │  - Flush remaining buffer                             │  │
│  │  - Inject compliance footers                          │  │
│  │  - Finalize audit trail                               │  │
│  │  - Generate metrics                                   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────┬───────────────────────────────┘
                              │ Modified SSE Stream
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         Client                               │
└─────────────────────────────────────────────────────────────┘
```

## Crate Architecture

### checkstream-core

**Purpose**: Foundation types and utilities

**Key Components**:
- `types.rs`: Token, ChatMessage, StreamChunk, Message
- `stream.rs`: TokenBuffer with holdback mechanism
- `error.rs`: Unified error handling

**Dependencies**: Minimal (tokio, serde, bytes)

**Performance**: Zero-cost abstractions, inline-friendly

### checkstream-proxy

**Purpose**: HTTP/SSE proxy server

**Key Components**:
- `main.rs`: Server initialization, CLI parsing
- `config.rs`: Configuration management
- `routes.rs`: HTTP endpoints (health, metrics, chat completions)
- `proxy.rs`: Core streaming logic (TODO: implementation)

**Technology Stack**:
- Async runtime: Tokio
- HTTP server: Axum (built on Hyper)
- HTTP client: Reqwest
- Metrics: Prometheus exporter

**Flow**:
1. Accept HTTP request on `/v1/chat/completions`
2. Validate and parse request
3. Run ingress stage checks
4. Proxy request to backend LLM
5. Stream response through midstream processing
6. Apply egress finalization
7. Return to client

### checkstream-policy

**Purpose**: Declarative policy evaluation engine

**Key Components**:
- `rule.rs`: Policy and Rule definitions
- `trigger.rs`: Trigger types (Pattern, Classifier, Context, Composite)
- `action.rs`: Action types (Log, Stop, Redact, Inject, Adapt, Audit)
- `engine.rs`: Policy evaluation logic

**Policy Format** (YAML):
```yaml
policies:
  - name: policy-name
    rules:
      - name: rule-name
        trigger: { type: classifier, classifier: pii, threshold: 0.9 }
        actions:
          - { type: redact }
          - { type: audit, category: pii, severity: high }
```

**Evaluation Model**:
- Load policies from YAML
- For each token chunk, evaluate all enabled rules
- Collect triggered actions
- Execute actions in priority order
- Record audit events

### checkstream-classifiers

**Purpose**: High-performance safety and compliance classifiers

**Tier System**:

**Tier A (<2ms)**:
- Pattern-based matching (Aho-Corasick algorithm)
- PII detection (regex-based)
- Simple rule-based checks

**Tier B (<5ms)**:
- Quantized neural networks
- Distilled BERT models
- Fast transformer variants

**Tier C (<10ms)**:
- Full-size models for nuanced detection
- Multi-stage cascades
- Ensemble methods

**Current Implementations**:
- `pii.rs`: Email, phone, SSN, credit card detection
- `patterns.rs`: Multi-pattern matching with Aho-Corasick
- `toxicity.rs`: Placeholder for ML model

**Classifier Trait**:
```rust
#[async_trait]
pub trait Classifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult>;
    fn name(&self) -> &str;
    fn tier(&self) -> ClassifierTier;
}
```

### checkstream-telemetry

**Purpose**: Observability and compliance audit trail

**Key Components**:

**Audit Trail** (`audit.rs`):
- Cryptographic hash chaining
- Tamper detection
- Regulatory event logging
- Immutable append-only structure

**Metrics** (`metrics.rs`):
- Request counters
- Token throughput
- Latency histograms
- Policy trigger rates
- Prometheus-compatible export

**Audit Event Structure**:
```rust
AuditEvent {
    event_type: String,
    data: Option<String>,
    timestamp: SystemTime,
    hash: Option<String>,           // SHA-256 hash
    previous_hash: Option<String>,   // Chain link
    regulation: Option<String>,      // e.g., "FCA PRIN 2A"
    severity: AuditSeverity,
}
```

## Data Flow

### Token Processing Pipeline

```
┌─────────────────────────────────────────────────────────┐
│ 1. Token arrives from LLM backend                       │
│    Input: { text: "invest", token_id: 1234, ... }      │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 2. Add to TokenBuffer                                   │
│    - Push to VecDeque                                   │
│    - Check if buffer exceeds holdback                   │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 3. Run classifiers on buffer window                     │
│    - Extract text from last N tokens                    │
│    - Parallel classifier execution                      │
│    - Collect scores                                     │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 4. Evaluate policies                                    │
│    - Match classifier results against triggers          │
│    - Execute actions if triggered                       │
│    - Record audit events                                │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 5. Release tokens beyond holdback                       │
│    - Drain releasable tokens from buffer                │
│    - Apply any modifications (redactions, injections)   │
│    - Stream to client                                   │
└─────────────────────────────────────────────────────────┘
```

## Performance Optimization Strategies

### 1. Zero-Copy Operations
- Use `Bytes` for buffer sharing without cloning
- Pin projections for safe streaming
- Avoid intermediate allocations

### 2. Async Concurrency
- Parallel classifier execution with `tokio::spawn`
- Non-blocking I/O throughout
- Bounded channels for backpressure

### 3. Efficient Data Structures
- `VecDeque` for O(1) push/pop on both ends
- `AhoCorasick` automaton for multi-pattern matching
- Pre-compiled regex patterns

### 4. Build Optimizations
```toml
[profile.release]
opt-level = 3           # Maximum optimizations
lto = "fat"             # Link-time optimization
codegen-units = 1       # Better optimization opportunity
strip = true            # Remove debug symbols
panic = "abort"         # Smaller binary, faster panics
```

### 5. Benchmarking
- Criterion.rs for statistical benchmarking
- Per-classifier latency tracking
- End-to-end throughput measurement

## Latency Budget Breakdown

Target: <10ms total overhead

| Stage | Budget | Components |
|-------|--------|------------|
| Ingress | 2-8ms | Prompt validation, PII scan |
| Classifier Tier A | <2ms | Pattern match, PII detection |
| Classifier Tier B | <5ms | Quantized ML models |
| Policy Evaluation | <1ms | Rule matching, action execution |
| Buffer Management | <1ms | Push/pop operations |
| Egress | 1-2ms | Footer injection, audit finalization |

**Total**: ~7-12ms (target met with optimizations)

## Scalability Considerations

### Horizontal Scaling
- Stateless proxy design
- Load balancer compatible
- Shared policy updates via control plane

### Vertical Scaling
- Multi-threaded Tokio runtime
- CPU-bound classifiers on thread pool
- Async I/O for network operations

### Resource Limits
- Bounded token buffer (prevent memory exhaustion)
- Rate limiting per client
- Circuit breakers for backend failures

## Security Architecture

### Data Privacy
- In-VPC deployment option
- No LLM traffic through control plane
- Configurable data residency

### Audit Integrity
- Cryptographic hash chaining
- Immutable audit log
- Tamper detection on read

### Access Control
- API key authentication (TODO)
- Role-based policy assignment
- TLS for all connections

## Future Architecture Enhancements

### Phase 2: vLLM Sidecar
- Direct integration with vLLM inference engine
- Logit masking for preventive safety
- KV-cache sharing for efficiency

### Phase 3: Control Plane
- Centralized policy distribution
- Fleet management and monitoring
- Multi-tenant SaaS architecture

### Phase 4: ML Pipeline
- Continuous model training
- Weak supervision labeling
- Canary rollouts with approval gates

## Technology Choices Rationale

### Why Rust?
- **Performance**: Zero-cost abstractions, no GC pauses
- **Safety**: Memory safety without runtime overhead
- **Concurrency**: Fearless async/await
- **Ecosystem**: Excellent async, HTTP, and ML libraries

### Why Tokio?
- Industry-standard async runtime
- Excellent performance and ergonomics
- Rich ecosystem (Axum, Hyper, etc.)

### Why Axum?
- Built on Hyper (battle-tested)
- Type-safe routing
- Minimal overhead
- Great middleware support

### Why Not Go?
- GC latency spikes unacceptable for <10ms target
- Weaker type system for safety-critical code

### Why Not Python?
- Too slow for real-time streaming
- GIL limits concurrency
- Higher memory overhead

## Monitoring and Observability

### Metrics (Prometheus)
- `checkstream_requests_total`: Request counter
- `checkstream_tokens_processed`: Token throughput
- `checkstream_latency_seconds`: Latency histogram
- `checkstream_policy_triggers_total`: Trigger counter
- `checkstream_classifier_latency_seconds`: Per-classifier timing

### Logs (Structured)
- Request/response tracing
- Policy trigger events
- Error conditions
- Performance warnings

### Distributed Tracing
- OpenTelemetry support (TODO)
- Request ID propagation
- Span hierarchy for debugging

## Deployment Patterns

### Development
```bash
cargo run --bin checkstream-proxy
```

### Docker
```bash
docker build -t checkstream:latest .
docker run -p 8080:8080 checkstream:latest
```

### Kubernetes
- Deployment with horizontal pod autoscaling
- ConfigMap for policies
- Service for load balancing
- Prometheus ServiceMonitor for metrics

## Testing Strategy

### Unit Tests
- Per-function correctness
- Edge case coverage
- Mock classifiers for determinism

### Integration Tests
- End-to-end request flow
- Policy evaluation scenarios
- Multi-classifier interaction

### Performance Tests
- Latency benchmarks (Criterion)
- Throughput stress tests
- Memory leak detection (Valgrind)

### Compliance Tests
- Audit trail verification
- Regulatory scenario coverage
- Hash chain integrity checks

---

This architecture is designed to scale from development to production while maintaining the sub-10ms latency requirement for real-time LLM safety and compliance.
