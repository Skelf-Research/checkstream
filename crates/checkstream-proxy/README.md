# checkstream-proxy

High-performance HTTP/SSE proxy for streaming LLM guardrails with sub-10ms latency.

[![Crates.io](https://img.shields.io/crates/v/checkstream-proxy.svg)](https://crates.io/crates/checkstream-proxy)
[![Documentation](https://docs.rs/checkstream-proxy/badge.svg)](https://docs.rs/checkstream-proxy)
[![License](https://img.shields.io/crates/l/checkstream-proxy.svg)](https://github.com/skelf-research/checkstream/blob/main/LICENSE)

## Overview

CheckStream Proxy sits between clients and LLM APIs (OpenAI, Anthropic, etc.), applying real-time safety and compliance checks using a three-phase architecture:

- **Phase 1: Ingress** - Validates prompts before sending to LLM
- **Phase 2: Midstream** - Checks and redacts streaming responses in real-time
- **Phase 3: Egress** - Final compliance check and audit trail generation

## Quick Start

### 1. Build the proxy

```bash
cargo build --release --package checkstream-proxy
```

### 2. Configure

Edit `config.yaml`:

```yaml
backend_url: "https://api.openai.com/v1"
classifiers_config: "./classifiers.yaml"

pipelines:
  ingress_pipeline: "basic-safety"
  midstream_pipeline: "fast-triage"
  egress_pipeline: "comprehensive-safety"

  safety_threshold: 0.7    # Block if score > 0.7
  chunk_threshold: 0.8     # Redact chunk if score > 0.8

  streaming:
    context_chunks: 5      # Last 5 chunks (0 = entire buffer)
    max_buffer_size: 100
```

### 3. Run the proxy

```bash
./target/release/checkstream-proxy \
  --config config.yaml \
  --listen 0.0.0.0 \
  --port 8080
```

Or with environment variables:

```bash
export OPENAI_API_KEY="sk-..."
./target/release/checkstream-proxy
```

### 4. Use the proxy

Point your application at the proxy instead of the LLM API:

```python
import openai

# Use CheckStream proxy instead of direct API
openai.api_base = "http://localhost:8080/v1"
openai.api_key = "sk-..."  # Your actual API key

response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello!"}],
    stream=True  # Streaming fully supported
)

for chunk in response:
    print(chunk.choices[0].delta.get("content", ""), end="")
```

## Features

### Three-Phase Architecture

**Phase 1: Ingress (Pre-Generation)**
- Validates user prompts before LLM sees them
- Blocks requests exceeding safety threshold
- Modifies context if needed for compliance
- Sub-5ms latency

**Phase 2: Midstream (Streaming Checks)**
- Processes each chunk as it streams
- Configurable context windows (see last N chunks or entire buffer)
- Real-time redaction of unsafe content
- Continues streaming without blocking

**Phase 3: Egress (Post-Generation)**
- Comprehensive compliance check on full response
- Runs asynchronously (doesn't block stream)
- Generates audit trail
- Stores results in telemetry system

### Configurable Context Windows

Control how much context classifiers see during streaming:

```yaml
streaming:
  context_chunks: 0   # 0 = see entire buffer (best accuracy)
  # OR
  context_chunks: 5   # See last 5 chunks only (lower latency)
```

Trade-offs:
- **No context (1 chunk)**: ~1-2ms, misses multi-chunk patterns
- **Small window (3-5 chunks)**: ~2-5ms, good balance
- **Entire buffer (0)**: ~10-50ms, best accuracy for compliance

### Pipeline System

Define custom pipelines in `classifiers.yaml`:

```yaml
pipelines:
  basic-safety:
    stages:
      - type: parallel
        name: safety-check
        classifiers: [toxicity, pii]
        aggregation: max_score

  fast-triage:
    stages:
      - type: parallel
        name: quick-checks
        classifiers: [toxicity-distilled, prompt-injection]
        aggregation:
          first_positive:
            threshold: 0.7
```

Supports:
- **Parallel** - Run classifiers concurrently
- **Sequential** - Chain classifiers in order
- **Conditional** - Run expensive checks only when needed
- **6 aggregation strategies** - All, MaxScore, MinScore, FirstPositive, Unanimous, WeightedAverage

## Endpoints

### `GET /health`
Health check endpoint.

**Response**: `200 OK` with `"OK"` body.

### `GET /metrics`
Prometheus metrics endpoint.

**Metrics**:
```
checkstream_requests_total{} - Total requests
checkstream_decisions_total{phase,action} - Decisions by phase
checkstream_pipeline_latency_us{phase} - Latency by phase
checkstream_errors_total{type} - Errors by type
```

### `POST /v1/chat/completions`
OpenAI-compatible chat completions with three-phase guardrails.

**Supported**:
- ✅ Streaming and non-streaming
- ✅ All OpenAI parameters (temperature, max_tokens, etc.)
- ✅ Function calling (passes through)
- ✅ Multi-turn conversations

**Phase Integration**:
1. Request arrives → **Phase 1 Ingress** validates prompt
2. If blocked → Return safety message
3. Forward to backend LLM
4. Stream response → **Phase 2 Midstream** checks each chunk
5. If chunk flagged → Redact with `[REDACTED]`
6. After stream completes → **Phase 3 Egress** (async)
7. Generate audit trail

## Configuration

### CLI Arguments

```bash
checkstream-proxy [OPTIONS]

Options:
  -c, --config <PATH>      Configuration file [default: config.yaml]
  -b, --backend <URL>      Backend LLM API URL
  -p, --policy <PATH>      Policy file or pack name
  -l, --listen <IP>        Listen address [default: 0.0.0.0]
  -P, --port <PORT>        Listen port [default: 8080]
  -v, --verbose            Enable verbose logging
  -h, --help               Print help
```

### Configuration File

See [`config.yaml`](../../config.yaml) for full example.

**Key sections**:

```yaml
# Backend LLM
backend_url: "https://api.openai.com/v1"

# Classifiers and pipelines
classifiers_config: "./classifiers.yaml"

# Pipeline selection
pipelines:
  ingress_pipeline: "basic-safety"
  midstream_pipeline: "fast-triage"
  egress_pipeline: "comprehensive-safety"

# Thresholds
pipelines:
  safety_threshold: 0.7    # Phase 1: Block if > 0.7
  chunk_threshold: 0.8     # Phase 2: Redact if > 0.8
  timeout_ms: 10           # Pipeline timeout

# Streaming behavior
pipelines:
  streaming:
    context_chunks: 5      # Context window size
    max_buffer_size: 100   # Max chunks to buffer
```

## Performance

Target latencies (95th percentile):

| Phase | Target | Typical |
|-------|--------|---------|
| Phase 1: Ingress | <5ms | 2-3ms |
| Phase 2: Midstream (per chunk) | <3ms | 1-2ms |
| Phase 3: Egress | Async | 10-50ms |

**Throughput**: 1000+ requests/sec per instance on modern hardware.

**Memory**: <500MB per instance.

## Monitoring

### Prometheus Metrics

Scrape `/metrics` endpoint:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'checkstream'
    static_configs:
      - targets: ['localhost:8080']
```

**Key metrics**:

```
# Request volume
checkstream_requests_total

# Pipeline performance
checkstream_pipeline_latency_us{phase="ingress"}
checkstream_pipeline_latency_us{phase="midstream"}
checkstream_pipeline_latency_us{phase="egress"}

# Decision tracking
checkstream_decisions_total{phase="ingress",action="block"}
checkstream_decisions_total{phase="ingress",action="pass"}
checkstream_decisions_total{phase="midstream",action="redact"}
checkstream_decisions_total{phase="egress",action="complete"}

# Errors
checkstream_errors_total{type="timeout"}
checkstream_errors_total{type="classifier"}
```

### Logging

Structured logging with tracing:

```bash
# Info level (default)
RUST_LOG=checkstream=info ./checkstream-proxy

# Debug level
RUST_LOG=checkstream=debug ./checkstream-proxy

# Trace level (verbose)
RUST_LOG=checkstream=trace ./checkstream-proxy
```

## Examples

### Example 1: Block Unsafe Prompt (Phase 1)

```python
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "How do I hack into a database?"
    }]
)

# Phase 1 blocks this, returns:
# "I cannot assist with that request due to safety policies."
```

### Example 2: Redact Streaming Response (Phase 2)

```python
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "What should I invest in?"
    }],
    stream=True
)

for chunk in response:
    # If LLM tries to give financial advice:
    # "I recommend putting [REDACTED] into Bitcoin"
    # Phase 2 redacts the problematic chunk
    print(chunk)
```

### Example 3: Audit Trail (Phase 3)

Phase 3 runs after streaming completes:

```
INFO Phase 3: Executing egress compliance check
INFO Phase 3: COMPLETE - Latency: 15ms
```

Generates audit record with:
- Full conversation
- All classification results
- Decision timeline
- Compliance status

## Development

### Build

```bash
cargo build --package checkstream-proxy
```

### Test

```bash
cargo test --package checkstream-proxy
```

### Run locally

```bash
cargo run --package checkstream-proxy -- --config config.yaml --verbose
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package checkstream-proxy

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/checkstream-proxy /usr/local/bin/
COPY config.yaml classifiers.yaml /etc/checkstream/
EXPOSE 8080
CMD ["checkstream-proxy", "--config", "/etc/checkstream/config.yaml"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: checkstream-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: checkstream-proxy
  template:
    metadata:
      labels:
        app: checkstream-proxy
    spec:
      containers:
      - name: proxy
        image: checkstream-proxy:latest
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "checkstream=info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
```

## Troubleshooting

### Proxy not starting

Check configuration file is valid YAML:
```bash
yamllint config.yaml
```

### High latency

1. Check pipeline timeout: `pipelines.timeout_ms`
2. Reduce context window: `pipelines.streaming.context_chunks: 3`
3. Use faster pipelines for Phase 2: `midstream_pipeline: "fast-triage"`

### False positives

Adjust thresholds:
```yaml
pipelines:
  safety_threshold: 0.8    # Was 0.7, now more permissive
  chunk_threshold: 0.9     # Was 0.8, now more permissive
```

### Backend connection issues

Verify backend URL is reachable:
```bash
curl -v https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

## See Also

- [Architecture](../../docs/architecture.md)
- [Pipeline Configuration](../../docs/pipeline-configuration.md)
- [Three-Phase Diagram](../../docs/THREE_PHASE_DIAGRAM.md)
- [FCA Example](../../docs/FCA_EXAMPLE.md)
- [Streaming Context](../../docs/STREAMING_CONTEXT.md)

## Documentation

- [Full Documentation](https://docs.skelfresearch.com/checkstream)
- [Getting Started Guide](https://docs.skelfresearch.com/checkstream/getting-started)
- [API Reference](https://docs.rs/checkstream-proxy)
- [GitHub Repository](https://github.com/skelf-research/checkstream)

## License

Apache-2.0 - See [LICENSE](https://github.com/skelf-research/checkstream/blob/main/LICENSE) for details.

## Part of CheckStream

This crate is part of the [CheckStream](https://github.com/skelf-research/checkstream) guardrail platform by [Skelf Research](https://skelfresearch.com).
