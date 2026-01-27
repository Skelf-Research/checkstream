# CheckStream

**High-Performance, Real-time Safety and Compliance Layer for Streaming LLMs**

CheckStream is a production-ready Rust guardrail platform that enforces safety, security, and regulatory compliance on LLM outputs as tokens stream—with **sub-10ms latency**. Works with any LLM provider.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-122%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)]()

## Current Status

**Version**: 0.1.0
**Status**: Core Complete - Production Ready for Testing

| Component | Status | Details |
|-----------|--------|---------|
| Three-Phase Proxy | **Complete** | Ingress, Midstream, Egress pipelines |
| ML Classifiers | **Working** | DistilBERT sentiment from HuggingFace |
| Pattern Classifiers | **Complete** | PII, prompt injection, custom patterns |
| Policy Engine | **Complete** | Triggers, actions, composite rules |
| Action Executor | **Complete** | Stop, Redact, Log, Audit actions |
| Audit Trail | **Complete** | Hash-chained, tamper-proof logging |
| Telemetry | **Complete** | Prometheus metrics, structured logging |
| Tests | **122 passing** | Unit, integration, ML classifier tests |

## Quick Start

### Build and Run

```bash
# Clone and build
git clone https://github.com/Skelf-Research/checkstream.git
cd checkstream
cargo build --release --features ml-models

# Run the proxy
./target/release/checkstream-proxy \
  --backend https://api.openai.com/v1 \
  --policy ./policies/default.yaml \
  --port 8080
```

### Test ML Model (Live Demo)

```bash
# Run the sentiment classifier example
cargo run --example test_hf_model --features ml-models

# Output:
# Model loaded successfully!
# "I love this movie!" → positive (1.000)
# "This is terrible."  → negative (1.000)
```

### Run Tests

```bash
cargo test --workspace  # 122 tests pass
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                          │
│            (OpenAI SDK, Anthropic SDK, etc.)                │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   CheckStream Proxy                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Phase 1    │  │   Phase 2    │  │   Phase 3    │      │
│  │   INGRESS    │→ │  MIDSTREAM   │→ │   EGRESS     │      │
│  │  Validate    │  │  Stream      │  │  Compliance  │      │
│  │  Prompt      │  │  Checks      │  │  & Audit     │      │
│  │  (~3ms)      │  │ (~2ms/chunk) │  │  (async)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Classifier Pipeline                      │   │
│  │  Pattern (Tier A) → ML Models (Tier B) → Policy      │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────┬──────────────────────────────────┘
                           │
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
      ┌────────┐     ┌─────────┐     ┌─────────┐
      │ OpenAI │     │ Claude  │     │  vLLM   │
      └────────┘     └─────────┘     └─────────┘
```

## Features

### ML Classifiers (Working)

Load models from HuggingFace with zero code:

```yaml
# models/registry.yaml
models:
  sentiment:
    source:
      type: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
    architecture:
      type: distil-bert-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"  # or "cuda" for GPU
      max_length: 512
```

**Performance** (CPU):
- DistilBERT: ~30-50ms per inference
- GPU (estimated): 2-10ms per inference

### Policy Engine

Define rules with triggers and actions:

```yaml
# policies/default.yaml
name: safety-policy
rules:
  - name: block-injection
    trigger:
      type: pattern
      pattern: "ignore previous instructions"
      case_insensitive: true
    actions:
      - type: stop
        message: "Request blocked"
        status_code: 403

  - name: toxicity-check
    trigger:
      type: classifier
      classifier: toxicity
      threshold: 0.8
    actions:
      - type: audit
        category: safety
        severity: high
```

### Three-Phase Pipeline

| Phase | Purpose | Latency |
|-------|---------|---------|
| **Ingress** | Validate prompts before LLM | ~3ms |
| **Midstream** | Check streaming chunks | ~2ms/chunk |
| **Egress** | Final compliance check | async |

### Health Endpoints

```bash
GET /health        # Basic health check
GET /health/live   # Kubernetes liveness probe
GET /health/ready  # Kubernetes readiness probe
GET /metrics       # Prometheus metrics
GET /audit         # Query audit trail
```

## Project Structure

```
checkstream/
├── crates/
│   ├── checkstream-core/        # Types, errors, token buffer
│   ├── checkstream-classifiers/ # ML models, patterns, pipeline
│   ├── checkstream-policy/      # Policy engine, triggers, actions
│   ├── checkstream-proxy/       # HTTP proxy server
│   └── checkstream-telemetry/   # Audit trail, metrics
├── examples/
│   ├── test_hf_model.rs         # Live ML model demo
│   └── full_dynamic_pipeline.rs # Complete pipeline example
├── policies/                    # Policy YAML files
├── models/                      # Model registry configs
└── docs/                        # Documentation
```

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | Technical design |
| [Getting Started](docs/getting-started.md) | Setup guide |
| [Model Loading](docs/model-loading.md) | ML model configuration |
| [Pipeline Configuration](docs/pipeline-configuration.md) | Classifier pipelines |
| [Policy Engine](docs/policy-engine.md) | Policy-as-code reference |
| [API Reference](docs/api-reference.md) | REST API docs |
| [FCA Example](docs/FCA_EXAMPLE.md) | Financial compliance example |
| [Deployment Modes](docs/deployment-modes.md) | Proxy vs Sidecar |
| [Security & Privacy](docs/security-privacy.md) | Data handling |
| [Regulatory Compliance](docs/regulatory-compliance.md) | FCA, FINRA, GDPR |

## Use Cases

- **Financial Services**: FCA Consumer Duty compliance, advice boundary detection
- **Healthcare**: HIPAA compliance, medical disclaimer injection
- **Security**: Prompt injection defense, PII protection, data exfiltration prevention
- **Content Moderation**: Real-time toxicity filtering

## Performance

| Component | Target | Actual |
|-----------|--------|--------|
| Pattern classifier | <2ms | ~0.5ms |
| ML classifier (CPU) | <50ms | ~30-50ms |
| ML classifier (GPU) | <10ms | ~2-10ms (est.) |
| Policy evaluation | <1ms | ~0.2ms |
| Total overhead | <10ms | ~5-8ms (patterns only) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 - See [LICENSE](LICENSE)

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/Skelf-Research/checkstream/issues)

---

**Built for trust at the speed of generation.**
