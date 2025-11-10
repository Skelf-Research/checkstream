# CheckStream

**Real-time Safety and Compliance Layer for Streaming AI**

CheckStream is a streaming guardrail platform that enforces safety, security, and regulatory compliance on LLM outputs as tokens are generated—without breaking latency budgets or compromising user experience.

## Overview

When you stream LLM tokens over HTTP Server-Sent Events (SSE), you lose the luxury of "fixing it after the fact." CheckStream provides guardrails that work **before**, **during**, and **as** tokens leave the model—with millisecond budgets.

### Key Capabilities

- **Token-level Safety**: Inspect and control outputs as they stream, not after completion
- **Sub-10ms Latency**: Quantized classifiers and optimized orchestration keep overhead minimal
- **Adversarial Robustness**: Multi-layer defense trained against obfuscation, jailbreaks, and evasion attempts
- **Regulatory Compliance**: Built-in support for FCA Consumer Duty, FINRA, MiFID II, and other regulations
- **Policy-as-Code**: Declarative, auditable rules mapped to specific regulations
- **Model-Agnostic**: Works with OpenAI, Anthropic, Bedrock, Azure, and self-hosted models
- **Cryptographic Audit Trail**: Hash-chained evidence for regulatory review
- **Data Sovereignty**: Deploy in-VPC with optional SaaS control plane

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

## Architecture

CheckStream operates in three stages:

1. **Ingress** (pre-generation): Prompt validation, PII detection, policy evaluation (2-8ms)
2. **Midstream** (during generation): Sliding token buffer, per-chunk safety checks, adaptive control (3-6ms per chunk)
3. **Egress** (finalization): Compliance footers, audit logging, evidence generation

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

## Quick Start

### Proxy Mode

```bash
# Install CheckStream
pip install checkstream

# Start proxy with default safety policies
checkstream proxy start \
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
checkstream proxy start \
  --backend https://api.anthropic.com/v1 \
  --policy-pack fca-consumer-duty \
  --telemetry aggregate
```

### vLLM Sidecar Mode

```bash
# Launch vLLM with CheckStream sidecar
docker-compose up vllm checkstream-sidecar

# Sidecar automatically hooks into vLLM generation
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

### Phase 1: Proxy MVP (Months 0-3)
- [x] HTTP/SSE proxy implementation (Go/Rust)
- [x] Sliding token buffer with holdback
- [x] Tier-A classifiers (toxicity, PII, prompt injection)
- [x] YAML policy engine
- [x] Basic telemetry and logging

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
