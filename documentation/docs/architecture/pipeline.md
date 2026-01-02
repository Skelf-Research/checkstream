# Three-Phase Pipeline

CheckStream processes requests through three distinct phases, each optimized for its specific purpose.

---

## Phase Overview

| Phase | Timing | Purpose | Blocking |
|-------|--------|---------|----------|
| **Ingress** | Before LLM call | Validate prompts | Yes |
| **Midstream** | During streaming | Real-time safety | Yes |
| **Egress** | After completion | Compliance & audit | No |

---

## Phase 1: Ingress

The ingress phase validates user prompts before they reach the LLM backend.

### Purpose

- Detect prompt injection attempts
- Block malicious or policy-violating inputs
- Validate request format and content
- Apply rate limiting and quotas

### Flow

```
Request → Parse → Classify Prompt → Evaluate Policy → Decision
                                                         │
                                    ┌────────────────────┴────────────────────┐
                                    ▼                                         ▼
                              ALLOW                                        BLOCK
                                    │                                         │
                                    ▼                                         ▼
                          Forward to LLM                               Return Error
                          (with optional                              (with reason)
                           context injection)
```

### Configuration

```yaml
pipeline:
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
      - pii_detector
    threshold: 0.85
    timeout_ms: 50
```

### Actions Available

| Action | Description |
|--------|-------------|
| `allow` | Forward request to backend |
| `block` | Reject with error message |
| `modify` | Transform prompt before forwarding |
| `inject` | Add system context |

### Example: Prompt Injection Detection

```yaml
policies:
  - name: block_jailbreak
    phase: ingress
    trigger:
      classifier: prompt_injection
      threshold: 0.8
    action: stop
    message: "Request blocked for safety review"
```

---

## Phase 2: Midstream

The midstream phase processes tokens as they stream from the LLM, enabling real-time safety enforcement.

### Purpose

- Monitor streaming tokens for unsafe content
- Redact problematic content inline
- Stop generation if threshold exceeded
- Maintain streaming UX while enforcing safety

### Holdback Buffer

To classify content effectively, midstream uses a holdback buffer:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Token Stream                                  │
│                                                                  │
│  Released      │  Holdback Buffer (16 tokens)  │  Incoming      │
│  ─────────────▶│  ═══════════════════════════  │◀──────────     │
│    to client   │        being classified       │   from LLM     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

As new tokens arrive:
1. Oldest tokens in buffer are classified
2. Safe tokens are released to client
3. Unsafe tokens are redacted or generation is stopped
4. New tokens enter the buffer

### Configuration

```yaml
pipeline:
  midstream:
    enabled: true
    token_holdback: 16          # Buffer size
    context_chunks: 3           # History for context
    classifiers:
      - toxicity
      - pii_detector
    chunk_threshold: 0.75
```

### Actions Available

| Action | Description |
|--------|-------------|
| `release` | Send tokens to client |
| `redact` | Replace with placeholder |
| `stop` | End generation |
| `buffer` | Hold for more context |

### Example: Toxicity Redaction

```yaml
policies:
  - name: redact_toxic
    phase: midstream
    trigger:
      classifier: toxicity
      threshold: 0.7
    action: redact
    replacement: "[CONTENT REMOVED]"
```

### Streaming Behavior

When content is redacted, the stream continues:

```
User sees: "The answer is [CONTENT REMOVED] and that's why..."
           ──────────────────────────────────────────────────▶
                              time
```

When generation is stopped:

```
User sees: "The answer is... [Generation stopped for safety]"
                                          │
                                          ▼
                                    Stream ends
```

---

## Phase 3: Egress

The egress phase performs comprehensive analysis after generation completes. It runs asynchronously and does not block the response.

### Purpose

- Full compliance verification
- Add required disclaimers
- Generate audit records
- Aggregate metrics

### Flow

```
Complete Response
        │
        ▼
┌───────────────────┐
│ Full Text Analysis│
│  - Compliance     │
│  - PII scan       │
│  - Quality check  │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│  Policy Engine    │
│  - Add disclaimers│
│  - Flag issues    │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│   Audit Trail     │
│  - Hash chain     │
│  - Compliance log │
└───────────────────┘
```

### Configuration

```yaml
pipeline:
  egress:
    enabled: true
    audit: true
    classifiers:
      - financial_advice
      - compliance_check
    inject_disclaimers: true
```

### Actions Available

| Action | Description |
|--------|-------------|
| `audit` | Create compliance record |
| `inject` | Add disclaimer/footer |
| `flag` | Mark for human review |
| `notify` | Send alert |

### Example: Financial Disclaimer

```yaml
policies:
  - name: add_financial_disclaimer
    phase: egress
    trigger:
      classifier: financial_advice
      threshold: 0.4
    action: inject
    position: end
    content: |

      ---
      *This information is for educational purposes only and does not
      constitute financial advice. Please consult a qualified advisor.*
```

---

## Phase Interaction

The three phases work together for comprehensive protection:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│  Request: "Tell me how to hack into a bank account"                    │
│                                                                         │
│  ┌──────────────────┐                                                   │
│  │   INGRESS        │                                                   │
│  │   prompt_injection│ = 0.92                                           │
│  │   threshold: 0.8 │                                                   │
│  │   ACTION: BLOCK  │◀────── Request stopped here                      │
│  └──────────────────┘                                                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│  Request: "Write a story with some dialogue"                           │
│                                                                         │
│  ┌──────────────────┐                                                   │
│  │   INGRESS        │                                                   │
│  │   prompt_injection│ = 0.12                                           │
│  │   ACTION: ALLOW  │                                                   │
│  └────────┬─────────┘                                                   │
│           │                                                             │
│           ▼                                                             │
│  ┌──────────────────┐                                                   │
│  │   MIDSTREAM      │                                                   │
│  │   "...you idiot" │                                                   │
│  │   toxicity = 0.78│                                                   │
│  │   ACTION: REDACT │──────▶ "[REMOVED]"                               │
│  └────────┬─────────┘                                                   │
│           │                                                             │
│           ▼                                                             │
│  ┌──────────────────┐                                                   │
│  │   EGRESS         │                                                   │
│  │   compliance ✓   │                                                   │
│  │   ACTION: AUDIT  │                                                   │
│  └──────────────────┘                                                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Performance Characteristics

| Phase | Target Latency | Actual | Notes |
|-------|---------------|--------|-------|
| Ingress | <5ms | 2-4ms | Pattern + ML classifiers |
| Midstream | <3ms/chunk | 1-2ms | Per-chunk processing |
| Egress | Async | N/A | Non-blocking |

---

## Best Practices

1. **Ingress**: Use fast classifiers (Tier A/B) to minimize request latency
2. **Midstream**: Balance holdback size with latency requirements
3. **Egress**: Run expensive analysis here since it's async
4. **Layer defenses**: Check for issues at multiple phases

---

## Next Steps

- [Classifier System](classifiers.md) - Understanding classifier tiers
- [Policy Engine](../guides/policy-engine.md) - Writing effective policies
- [Configuration](../configuration/pipelines.md) - Pipeline configuration options
