# CheckStream

**Provider-Agnostic, Real-time Safety and Compliance Layer for Streaming AI**

CheckStream is a streaming guardrail platform that enforces safety, security, and regulatory compliance on LLM outputs as tokens are generated—**regardless of which LLM provider you use**. Works with OpenAI, Anthropic, Bedrock, self-hosted models, or any future provider—without breaking latency budgets or compromising user experience.

> **🔑 Core Philosophy**: CheckStream doesn't care about your backend, deployment, or use case. It just makes your LLM applications safer, regardless of how you build them.

## Overview

When you stream LLM tokens over HTTP Server-Sent Events (SSE), you lose the luxury of "fixing it after the fact." CheckStream provides guardrails that work **before**, **during**, and **as** tokens leave the model—with millisecond budgets.

### Key Capabilities

#### Provider & Agent Agnostic
- ✅ **Works with ANY LLM**: OpenAI, Anthropic, Google, AWS, Azure, self-hosted (vLLM, Ollama), or custom
- ✅ **Works with ANY Agent Framework**: LangChain, AutoGen, CrewAI, Semantic Kernel, custom agents
- ✅ **Guards Final Output Only**: Agent does tools/planning/retrieval internally, CheckStream guards the streaming response
- ✅ **No Vendor Lock-in**: Switch providers with a config change, no code changes
- ✅ **Multi-Provider**: Route to different providers based on cost, latency, or compliance
- ✅ **Your Infrastructure**: All processing local, data never leaves your control

#### Safety & Compliance
- **Token-level Safety**: Inspect and control outputs as they stream, not after completion
- **Sub-10ms Latency**: Quantized classifiers and optimized orchestration keep overhead minimal
- **Adversarial Robustness**: Multi-layer defense against obfuscation, jailbreaks, and evasion
- **Regulatory Compliance**: FCA Consumer Duty, FINRA, MiFID II, GDPR, HIPAA
- **Policy-as-Code**: Declarative, auditable rules mapped to specific regulations
- **Cryptographic Audit Trail**: Hash-chained evidence for regulatory review

#### Deployment Flexibility
- **Run Anywhere**: Docker, Kubernetes, AWS, GCP, Azure, on-prem, edge
- **Any Mode**: Standalone proxy, sidecar, gateway, or embedded library
- **Data Sovereignty**: Deploy in your VPC, your region, your compliance zone

## Use Cases

### Financial Services
- **Retail Banking**: Prevent misleading promotions, ensure fee transparency
- **Lending Platforms**: Consumer Duty compliance, vulnerability detection
- **Investment Apps**: Risk disclosure requirements, advice vs. information boundaries
- **Insurtech**: Clear product communication, fair claims assistance

### Regulated Industries
- **Healthcare**: HIPAA compliance, medical disclaimer injection
- **Legal Services**: Ethical walls, unauthorized practice prevention
- **Government**: Classification control, PII protection

### Security
- **Prompt Injection Defense**: Real-time detection and blocking
- **Data Exfiltration Prevention**: Catch secrets and PII before they leak
- **Toxicity & Abuse Prevention**: Sub-second moderation with context awareness

## How It Works: Provider-Agnostic Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Your Application (Unchanged)                │
│   Use OpenAI SDK, Anthropic SDK, or any HTTP client     │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ Point to CheckStream instead of LLM API
                     │
                     ↓
┌─────────────────────────────────────────────────────────┐
│                  CheckStream Proxy                       │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐    │
│  │  Phase 1:   │  │   Phase 2:   │  │  Phase 3:   │    │
│  │  Ingress    │→ │  Midstream   │→ │   Egress    │    │
│  │ (~2-3ms)    │  │ (~1-2ms/chk) │  │  (async)    │    │
│  └─────────────┘  └──────────────┘  └─────────────┘    │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ Forward to ANY backend (you configure)
                     │
        ┌────────────┼────────────┬─────────────┬──────────┐
        ↓            ↓            ↓             ↓          ↓
   ┌────────┐  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────┐
   │ OpenAI │  │ Claude  │  │ Gemini  │  │  Bedrock │  │ vLLM │
   └────────┘  └─────────┘  └─────────┘  └──────────┘  └──────┘
```

**Change backend with one line**:
```yaml
backend_url: "https://api.openai.com/v1"      # Use OpenAI
# backend_url: "https://api.anthropic.com/v1"  # Or Claude
# backend_url: "http://localhost:8000/v1"      # Or local vLLM
```

### Three-Phase Pipeline

CheckStream operates in three stages:

1. **Ingress** (pre-generation): Prompt validation, PII detection, policy evaluation (2-8ms)
2. **Midstream** (during generation): Sliding token buffer, per-chunk safety checks, adaptive control (3-6ms per chunk)
3. **Egress** (finalization): Compliance footers, audit logging, evidence generation

**All phases work the same** regardless of which LLM backend you use.

### Deployment Modes

**Proxy Mode** (Universal)
- Drop-in HTTP/SSE proxy for any LLM API
- Model-agnostic, cloud-neutral
- ~10ms added latency
- Works with OpenAI, Anthropic, Bedrock, Azure OpenAI

**Sidecar Mode** (Advanced)
- Deep integration with vLLM inference engine
- Logit masking for preventive safety
- Adaptive decoding (temperature, top-p adjustment)
- ~5ms added latency, stronger guarantees

**Control Plane** (Enterprise)
- SaaS policy management and distribution
- Fleet orchestration across deployment modes
- Centralized telemetry and compliance dashboards
- Out-of-band architecture (no LLM traffic through control plane)

## Technology Stack

CheckStream is built with **Rust** for maximum performance and reliability:

- **Async Runtime**: Tokio for high-concurrency streaming workloads
- **HTTP/2 & WebSocket**: Hyper and Axum for low-latency proxy operations
- **Zero-Copy Streaming**: Efficient buffer management with minimal allocations
- **ML Inference**: Candle for on-device classifier execution
- **Classifier Pipelines**: Parallel and sequential execution with conditional logic
- **Sub-millisecond Overhead**: Optimized release builds with LTO and aggressive optimizations

### Classifier System

CheckStream's classifier system is organized into three tiers based on latency budgets:

- **Tier A (<2ms)**: Pattern matching, regex, PII detection using Aho-Corasick
- **Tier B (<5ms)**: Quantized neural classifiers for toxicity, prompt injection, sentiment
- **Tier C (<10ms)**: Full-size models for nuanced classification tasks

Classifiers can be:
- **Chained sequentially** for progressive depth analysis
- **Run in parallel** for maximum throughput
- **Conditionally executed** based on previous results
- **Aggregated** using various strategies (max, min, unanimous, weighted average)

See [`docs/pipeline-configuration.md`](docs/pipeline-configuration.md) for details.

## Quick Start

> **Note**: CheckStream is currently in active development. The installation commands below represent the target architecture.

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/checkstream.git
cd checkstream

# Build with release optimizations
cargo build --release

# Run the proxy
./target/release/checkstream-proxy \
  --backend https://api.openai.com/v1 \
  --policy ./policies/default.yaml \
  --port 8080
```

### Proxy Mode

```bash
# Start proxy with default safety policies
checkstream-proxy start \
  --backend https://api.openai.com/v1 \
  --policy ./policies/default.yaml

# Test with a client
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Tell me about investing"}],
    "stream": true
  }'
```

### With Regulatory Policies

```bash
# Use FCA Consumer Duty compliance pack
checkstream-proxy start \
  --backend https://api.anthropic.com/v1 \
  --policy-pack fca-consumer-duty \
  --telemetry aggregate
```

### Docker Deployment

```bash
# Build Docker image
docker build -t checkstream/proxy:latest .

# Run with Docker Compose
docker-compose up
```

## Key Differentiators

| Feature | CheckStream | Cloud Guardrails | Security SaaS | Offline Checkers |
|---------|-------------|------------------|---------------|------------------|
| **Streaming-aware** | Token-level | Chunk-level | Coarse | N/A |
| **Latency** | <10ms | 50-100ms | 30-60ms | Offline |
| **Regulatory taxonomy** | FCA, FINRA, MiFID | Generic harms | Security-only | Marketing-only |
| **Audit trail** | Cryptographic chain | Basic logs | Monitoring | Reports |
| **Deployment** | Proxy/Sidecar/SaaS | Managed only | SaaS proxy | Desktop |
| **Model-agnostic** | ✓ | Vendor-locked | ✓ | ✓ |
| **Data residency** | In-VPC option | Cloud-only | SaaS-only | Local |

## Policy Example

```yaml
policies:
  - name: investment-advice-boundary
    description: Detect regulated advice vs. factual information
    rules:
      - classifier: advice_vs_info
        threshold: 0.75
        action: inject_disclaimer
        disclaimer: "This is information only, not financial advice."
        regulation: "FCA COBS 9A"

      - classifier: suitability_risk
        threshold: 0.8
        action: stop
        message: "I cannot provide personalized recommendations without assessing your circumstances."
        regulation: "FCA PRIN 2A"

  - name: vulnerability-support
    description: Detect and respond to vulnerable customer cues
    rules:
      - pattern: "(can't pay|struggling|bereaved|disabled|anxious)"
        action: adapt_tone
        response_mode: supportive
        inject_resources: true
        regulation: "FCA FG21/1"
```

## Documentation

### Core Concepts
- [Overview](docs/overview.md) - Problem statement and solution approach
- [Architecture](docs/architecture.md) - Technical design and components
- [Adversarial Robustness](docs/adversarial-robustness.md) - Classifier training, evasion detection, red teaming
- [Pre-Production Validation](docs/pre-production-validation.md) - Testing, risk assessment, compliance sign-off
- [Deployment Modes](docs/deployment-modes.md) - Proxy vs. Sidecar detailed comparison

### Implementation Guides
- [Getting Started](docs/getting-started.md) - Installation and setup guide
- [Policy Engine](docs/policy-engine.md) - Policy-as-code reference
- [Control Plane](docs/control-plane.md) - SaaS management and orchestration
- [Security & Privacy](docs/security-privacy.md) - Data residency and audit model
- [API Reference](docs/api-reference.md) - REST API and integration guide

### Use Cases & Business
- [Use Cases](docs/use-cases.md) - Industry scenarios and examples
- [Regulatory Compliance](docs/regulatory-compliance.md) - FCA Consumer Duty and other frameworks
- [Business Positioning](docs/business-positioning.md) - Market analysis and value proposition

## Roadmap

### Phase 1: Proxy MVP (Months 0-3) - In Progress
- [ ] HTTP/SSE proxy implementation (Rust with Tokio/Axum)
- [ ] Sliding token buffer with holdback
- [ ] Tier-A classifiers (toxicity, PII, prompt injection)
- [ ] YAML policy engine
- [ ] Basic telemetry and logging

### Phase 2: vLLM Sidecar (Months 4-6)
- [ ] vLLM callback hooks and IPC
- [ ] Logit masking and adaptive decoding
- [ ] KV-cache reuse for classifiers
- [ ] Enhanced telemetry with token embeddings

### Phase 3: Control Plane (Months 7-9)
- [ ] Multi-tenant SaaS policy distribution
- [ ] Fleet management and health monitoring
- [ ] Compliance dashboards (Consumer Duty outcomes)
- [ ] Model registry and versioning

### Phase 4: Continuous Learning (Months 10-12)
- [ ] Weak supervision and labeling pipeline
- [ ] Teacher-student model distillation
- [ ] Canary rollouts with approval gates
- [ ] Drift monitoring and auto-degrade

## License

Apache 2.0 (see LICENSE file)

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/yourusername/checkstream/issues)
- Enterprise Support: contact@checkstream.ai

## Security

For security concerns or vulnerability reports, please email security@checkstream.ai. Do not open public issues for security matters.

---

**Built for trust at the speed of generation.**
