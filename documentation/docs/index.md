# CheckStream

**Production-ready streaming guardrails for LLM safety and compliance.**

CheckStream is a high-performance Rust guardrail platform that enforces safety, security, and regulatory compliance on LLM outputs as tokens stream in real-time with sub-10ms latency.

---

## Why CheckStream?

Modern AI applications require real-time safety enforcement without sacrificing user experience. CheckStream provides:

- **Real-time Protection**: Sub-10ms latency guardrails that work as tokens stream
- **Regulatory Compliance**: Built-in support for FCA, FINRA, GDPR, HIPAA regulations
- **Production Ready**: 122+ tests passing, graceful shutdown, Kubernetes health probes
- **Zero Python Dependencies**: Pure Rust with Candle ML framework for reliable deployment

---

## How It Works

CheckStream operates as a transparent proxy between your application and any LLM provider:

```
┌─────────────┐     ┌──────────────────────────────────────────────┐     ┌─────────────┐
│             │     │              CheckStream Proxy                │     │             │
│   Client    │────▶│  Phase 1 ──▶ Phase 2 (stream) ──▶ Phase 3   │────▶│  LLM API    │
│ Application │◀────│  Ingress     Midstream            Egress     │◀────│  Backend    │
│             │     │                                              │     │             │
└─────────────┘     └──────────────────────────────────────────────┘     └─────────────┘
```

**Three-Phase Pipeline:**

| Phase | When | Purpose | Latency |
|-------|------|---------|---------|
| **Ingress** | Before LLM | Validate prompts, block unsafe requests | ~3ms |
| **Midstream** | During streaming | Real-time token safety, redaction | ~2ms/chunk |
| **Egress** | After completion | Compliance checks, audit trail | Async |

---

## Key Features

### Tiered Classification System

| Tier | Latency | Method | Use Case |
|------|---------|--------|----------|
| **A** | <2ms | Pattern matching | PII, prompt injection patterns |
| **B** | <5ms | Quantized ML | Toxicity, sentiment |
| **C** | <10ms | Full models | Complex domain classifiers |

### Policy-as-Code

Define safety rules in simple YAML:

```yaml
policies:
  - name: block_financial_advice
    trigger:
      classifier: financial_advice
      threshold: 0.8
    action: stop
    message: "Financial advice requires suitability assessment"
    regulation: "FCA COBS 9A.2.1R"
```

### HuggingFace Integration

Auto-download and cache models from HuggingFace Hub:

```yaml
classifiers:
  toxicity:
    model: "unitary/toxic-bert"
    tier: B
    device: auto
```

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/checkstream/checkstream
cd checkstream
cargo build --release --features ml-models

# Download models
./scripts/download_models.sh

# Run proxy
./target/release/checkstream-proxy --config config.yaml
```

Then point your LLM client to `http://localhost:8080` instead of the upstream API.

[Get started with the full installation guide](getting-started/installation.md){ .md-button .md-button--primary }

---

## Supported Backends

CheckStream works with any OpenAI-compatible API:

- OpenAI
- Anthropic (via adapter)
- Azure OpenAI
- vLLM
- Ollama
- Any OpenAI-compatible endpoint

---

## Use Cases

- **Financial Services**: FCA/FINRA compliant AI assistants
- **Healthcare**: HIPAA-compliant patient communication
- **Enterprise**: Content moderation and brand safety
- **Security**: Prompt injection and jailbreak prevention

[Explore use cases](examples/financial-compliance.md){ .md-button }

---

## Performance

| Metric | Target | Actual |
|--------|--------|--------|
| Pattern classification | <2ms | ~0.5ms |
| ML classification (CPU) | <50ms | 30-50ms |
| Total proxy overhead | <10ms | 5-8ms |
| Throughput | 1000 req/s | 1000+ req/s |
| Memory | <500MB | <500MB |
