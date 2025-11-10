# CheckStream Overview

## The Problem

Modern AI applications increasingly rely on **streaming LLM responses** to provide real-time, conversational experiences. When you stream tokens over HTTP Server-Sent Events (SSE) or WebSockets, every token flows immediately to the user—there's no opportunity to review, modify, or retract the complete response before it's seen.

This creates critical challenges:

### 1. Safety Without Second Chances

Traditional content moderation operates on complete responses. With streaming:
- **Unsafe content can leak** before you detect it
- **Retractions are impossible** in standard SSE (no "unsend" primitive)
- **User trust erodes** when they see harmful content, even briefly

### 2. Regulatory Compliance in Real-Time

Regulated industries face stringent requirements:
- **Financial services**: FCA Consumer Duty requires clear, fair, not misleading communications—verified before they reach customers
- **Healthcare**: HIPAA requires PII protection and appropriate medical disclaimers
- **Legal services**: Unauthorized practice of law must be prevented, not corrected after the fact

Batch moderation APIs that take 50-200ms per call cannot keep pace with 30-80 tokens/second generation rates without devastating user experience.

### 3. Security Threats During Generation

Adversarial attacks exploit streaming behavior:
- **Prompt injection**: Malicious instructions embedded in user input or retrieved context
- **Data exfiltration**: Secrets or PII extracted through carefully crafted prompts
- **Jailbreaks**: Multi-turn attacks that gradually bypass safety boundaries

Detection must happen **as the attack unfolds**, not after the damage is done.

### 4. Latency Budgets Are Tight

Users expect:
- **Time to First Token (TTFT)**: 150-300ms
- **Token cadence**: 30-80 tokens/second
- **Total completion**: <5 seconds for typical responses

Adding 50-100ms per moderation check destroys this experience. Traditional safety solutions force an impossible choice: **fast or safe**.

## The CheckStream Solution

CheckStream is a **streaming guardrail platform** that enforces safety, security, and regulatory compliance **during token generation**, with millisecond-level overhead.

### Core Principles

#### 1. Real-Time Enforcement
Safety decisions happen **as tokens stream**, using a sliding holdback buffer (8-32 tokens) that allows inspection before emission—without users perceiving delay.

#### 2. Multi-Tier Defense
- **Ingress** (2-8ms): Validate prompts, detect injections, set decoding constraints
- **Midstream** (3-6ms per chunk): Token-level classification, policy evaluation, adaptive patching
- **Egress**: Compliance footers, audit trail generation

#### 3. Latency Parity
Quantized classifiers (INT8/INT4), CPU-optimized inference, and clever scheduling keep total overhead under 10ms per chunk—imperceptible to users.

#### 4. Policy Transparency
Declarative rules with **explicit regulatory citations**:
```yaml
rule: suitability_assessment_required
trigger: classifier.advice_vs_info > 0.75
action: stop_with_message
regulation: "FCA COBS 9A.2.1R"
rationale: "Personalized recommendations require suitability assessment"
```

Every decision is **explainable** and **auditable**.

#### 5. Model Agnostic
Works with:
- **Cloud APIs**: OpenAI, Anthropic, Bedrock, Azure OpenAI, Google Vertex
- **Open source**: vLLM, TGI, Ollama, LocalAI
- **Custom models**: Any HTTP/SSE endpoint or Python inference engine

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Application                      │
└────────────────────────┬────────────────────────────────────┘
                         │ SSE stream
                         ▼
          ┌──────────────────────────────┐
          │   CheckStream Gateway        │
          │  ┌────────────────────────┐  │
          │  │  Ingress Filter        │  │  (prompt validation)
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  LLM Backend           │  │  (OpenAI/Anthropic/vLLM)
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  Token Stream          │  │
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  Holdback Buffer       │  │  (8-32 tokens)
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  Midstream Classifiers │  │  (toxicity, PII, regulatory)
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  Policy Engine         │  │  (allow/redact/stop/patch)
          │  └───────────┬────────────┘  │
          │              ▼                │
          │  ┌────────────────────────┐  │
          │  │  Safe Token Flush      │  │
          │  └───────────┬────────────┘  │
          └──────────────┼────────────────┘
                         │ safe SSE stream
                         ▼
          ┌──────────────────────────────┐
          │    Audit & Telemetry         │
          │  (hash chain, metrics, SIEM) │
          └──────────────────────────────┘
```

### Key Capabilities

#### Real-Time Token Control
- **Sliding window analysis**: Never stream raw tokens immediately; inspect rolling buffer
- **Patch and continue**: Replace unsafe spans with `[REDACTED]` or safer paraphrases
- **Adaptive decoding** (vLLM mode): Adjust temperature, top-p, or mask logits when risk increases
- **Circuit breakers**: Terminate stream gracefully with compliant refusal when thresholds exceeded

#### Regulatory Classifiers
Pre-trained models for:
- **FCA Consumer Duty**: Promotional balance, advice boundaries, vulnerability detection, suitability risk
- **FINRA**: Rule 2210 communications, suitability (Rule 2111)
- **MiFID II**: Investment advice, appropriateness assessment
- **HIPAA**: PHI detection, medical disclaimer requirements
- **General safety**: Toxicity, hate speech, sexual content, self-harm

#### Policy-as-Code Engine
- **Declarative YAML/Rego** with hot-reload
- **Rule composition**: Combine classifiers, regex, context signals
- **Action primitives**: `allow`, `allow_constrained`, `redact`, `inject_disclaimer`, `stop`, `adapt_tone`
- **Explainability**: Every action includes rule ID, regulation citation, confidence score

#### Cryptographic Audit Trail
- **Hash-chained events**: Each decision linked to previous via SHA-256
- **Immutable logs**: Tampering is detectable
- **Evidence packs**: Export CSV/PDF summaries for regulators
- **Replay capability**: Reconstruct exactly what happened in any stream

#### Privacy-First Telemetry
Two modes:
- **Aggregate**: Only counters and rates leave customer VPC
- **Full evidence**: Per-decision records with PII minimization (hashes, short spans only)

## Value Proposition

### For Developers
- **Drop-in integration**: Proxy mode requires no model changes
- **<10ms overhead**: Latency remains imperceptible
- **Streaming visualization**: Real-time dashboard showing tokens, risk scores, actions
- **Local dev mode**: Test policies on localhost before production

### For Risk & Compliance Teams
- **Deterministic enforcement**: Rules applied before text leaves your infrastructure
- **Audit-ready evidence**: Signed logs with regulatory citations
- **Policy-as-code**: Version-controlled, reviewable rules
- **Compliance dashboards**: Consumer Duty outcomes, breach trends, vulnerability detection rates

### For Security Teams
- **Runtime protection**: Block prompt injection and data exfiltration in real-time
- **Zero-trust AI**: Enforce least-privilege tool use and context boundaries
- **Threat telemetry**: Structured logs of attempted jailbreaks and exploits
- **SIEM integration**: Splunk, Datadog, Chronicle, Sentinel connectors

### For Executives
- **Risk mitigation**: Prevent regulatory fines and reputational damage
- **Faster time-to-market**: Ship AI features with built-in compliance
- **Cost efficiency**: Unified platform replacing multiple point solutions
- **Data sovereignty**: Deploy in-VPC or on-premises for regulated data

## Why Now?

### Market Timing
- **Streaming adoption**: Exploding due to chat UX and agentic runtimes
- **Regulatory momentum**: EU AI Act, FCA Consumer Duty, US AI Safety standards
- **Latency expectations**: <200ms TTFT, ~50 tok/s—no room for slow safety layers
- **Security urgency**: Prompt injection and data exfiltration now common attack vectors

### Technology Readiness
- **Quantized models**: INT8/INT4 inference enables sub-10ms classifier latency
- **Efficient serving**: vLLM, TGI provide hooks for deep integration
- **Policy frameworks**: Cedar, Rego provide proven declarative rule engines
- **SSE maturity**: Well-understood protocol with broad client support

### Competitive Gap
No existing solution provides:
- True **token-level** streaming enforcement (not batch/chunk)
- **Sub-10ms** latency (vs. 50-200ms for API-based moderation)
- **Regulatory taxonomy** out-of-the-box (vs. generic "toxicity" labels)
- **Cryptographic auditability** (vs. basic logging)
- **In-VPC deployment** with optional SaaS control plane

CheckStream is the first platform purpose-built for this convergence of requirements.

## Next Steps

- **Quickstart**: See [Getting Started](getting-started.md)
- **Architecture deep-dive**: See [Architecture](architecture.md)
- **Choose deployment mode**: See [Deployment Modes](deployment-modes.md)
- **Explore use cases**: See [Use Cases](use-cases.md)
- **Understand compliance**: See [Regulatory Compliance](regulatory-compliance.md)
