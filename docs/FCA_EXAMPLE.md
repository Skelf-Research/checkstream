# FCA Consumer Duty Example - Pipeline Phases

This document explains how CheckStream's pipeline system works in a real-world FCA (Financial Conduct Authority) Consumer Duty compliance scenario.

## Overview

When a financial services chatbot generates responses about products or advice, CheckStream operates in three phases to ensure compliance with FCA Consumer Duty regulations.

## The Three Phases

```
┌─────────────────────────────────────────────────────────────────┐
│                        Request Flow                             │
│                                                                 │
│  User Question → [Phase 1] → LLM → [Phase 2] → [Phase 3] → User│
│                   Ingress      Generation  Midstream   Egress    │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 1: Ingress (Pre-Generation)
**When**: Before the LLM generates any response
**Latency Budget**: 2-8ms
**Purpose**: Validate the user's prompt

### Phase 2: Midstream (During Generation)
**When**: As tokens stream from the LLM
**Latency Budget**: 3-6ms per chunk
**Purpose**: Real-time safety checks on output

### Phase 3: Egress (Post-Generation)
**When**: After generation is complete
**Latency Budget**: Flexible (not in critical path)
**Purpose**: Add compliance footers, audit logging

---

## Concrete FCA Example

Let's walk through a real example:

### User Question
```
"Should I invest my savings in crypto? I have £50,000."
```

---

## Phase 1: Ingress (Prompt Validation)

### What Happens

CheckStream intercepts the user's prompt **before** it reaches the LLM and runs it through a validation pipeline.

### Pipeline Configuration

```yaml
pipelines:
  fca-ingress-check:
    description: "Pre-generation prompt validation for FCA compliance"
    stages:
      # Stage 1: Fast pattern-based checks (Tier A)
      - type: parallel
        name: quick-filters
        classifiers:
          - pii-detector           # Check for personal data leaks
          - prompt-injection       # Detect jailbreak attempts
          - regulated-topic        # Identify financial advice requests
        aggregation: max_score

      # Stage 2: Risk assessment (Tier B, conditional)
      - type: conditional
        name: risk-assessment
        classifier: financial-risk-classifier
        condition:
          classifier_triggered:
            classifier: regulated-topic
```

### Execution Flow

```
Input: "Should I invest my savings in crypto? I have £50,000."

┌────────────────────────────────────────────────────────┐
│ Stage 1: quick-filters (parallel, ~2ms)               │
│                                                         │
│  ┌─────────────────┐                                  │
│  │ pii-detector    │ → Score: 0.3 (£50,000 = money)  │
│  └─────────────────┘                                  │
│                                                         │
│  ┌─────────────────┐                                  │
│  │ prompt-injection│ → Score: 0.1 (clean)            │
│  └─────────────────┘                                  │
│                                                         │
│  ┌─────────────────┐                                  │
│  │ regulated-topic │ → Score: 0.9 (investment advice!)│
│  └─────────────────┘                                  │
│                                                         │
│  Aggregation (max_score): 0.9                         │
│  Decision: regulated-topic TRIGGERED                  │
└────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────┐
│ Stage 2: risk-assessment (conditional, ~4ms)          │
│                                                         │
│  Condition: regulated-topic triggered? ✓ YES          │
│                                                         │
│  ┌─────────────────────────────────────┐             │
│  │ financial-risk-classifier            │             │
│  │                                      │             │
│  │ Detected:                            │             │
│  │  • Investment advice request         │             │
│  │  • Specific amount (£50,000)         │             │
│  │  • High-risk asset (crypto)          │             │
│  │                                      │             │
│  │ Risk Level: HIGH (0.95)             │             │
│  └─────────────────────────────────────┘             │
└────────────────────────────────────────────────────────┘

Final Decision:
  Score: 0.95 (HIGH RISK)
  Action: MODIFY CONTEXT
  Total Latency: ~6ms
```

### Outcome: Context Modification

CheckStream **doesn't block** the request, but modifies the LLM context:

```
Original prompt: "Should I invest my savings in crypto? I have £50,000."

Modified prompt sent to LLM:
─────────────────────────────────────────────────────────
SYSTEM CONTEXT (prepended by CheckStream):

You are a UK financial services chatbot subject to FCA Consumer Duty.
This user is asking about investment decisions.

MANDATORY REQUIREMENTS:
1. DO NOT provide personalized investment advice
2. Clearly state you cannot recommend specific investments
3. Suggest speaking to a regulated financial adviser
4. Include appropriate risk warnings
5. Clarify the distinction between information and advice

PROHIBITED:
- Recommending specific investments
- Suggesting portfolio allocations
- Predicting returns
- Encouraging high-risk investments

USER QUESTION:
Should I invest my savings in crypto? I have £50,000.
─────────────────────────────────────────────────────────
```

Now the LLM generates with **compliance guardrails built into the context**.

---

## Phase 2: Midstream (Streaming Checks)

### What Happens

As the LLM streams tokens back, CheckStream checks **each chunk** before sending it to the user.

### Pipeline Configuration

```yaml
pipelines:
  fca-midstream-check:
    description: "Per-chunk streaming validation"
    stages:
      # Use fastest possible pipeline (Tier A/B only)
      - type: parallel
        name: stream-safety
        classifiers:
          - advice-vs-information    # Tier B: 2ms
          - risk-disclosure-check    # Tier A: 1ms
          - misleading-claims        # Tier B: 3ms
        aggregation: first_positive
        threshold: 0.8    # High confidence to avoid false positives
```

### LLM Starts Streaming

```
LLM Output (streaming tokens):

Chunk 1: "I understand you're considering crypto"
Chunk 2: " investments. While I can provide general"
Chunk 3: " information, I cannot give personalized"
Chunk 4: " investment advice. Cryptocurrencies are"
Chunk 5: " highly volatile and you could lose all"
Chunk 6: " your money. You should speak to an FCA"
Chunk 7: "-regulated financial adviser who can assess"
Chunk 8: " your individual circumstances."
```

### Execution Per Chunk

```
┌─────────────────────────────────────────────────────────┐
│ Chunk 1: "I understand you're considering crypto"      │
│                                                          │
│  Pipeline: fca-midstream-check                          │
│  ┌──────────────────┐                                  │
│  │ advice-detector  │ → Score: 0.1 (information, OK)   │
│  │ risk-disclosure  │ → Score: 0.0 (neutral)           │
│  │ misleading       │ → Score: 0.0 (clean)             │
│  └──────────────────┘                                  │
│  Decision: PASS                                         │
│  Latency: 3ms                                           │
└─────────────────────────────────────────────────────────┘
         ↓
    Send to User ✓


┌─────────────────────────────────────────────────────────┐
│ Chunk 5: " highly volatile and you could lose all"     │
│                                                          │
│  Pipeline: fca-midstream-check                          │
│  ┌──────────────────┐                                  │
│  │ advice-detector  │ → Score: 0.2 (still info, OK)    │
│  │ risk-disclosure  │ → Score: 0.9 (GOOD! has warning) │
│  │ misleading       │ → Score: 0.0 (clean)             │
│  └──────────────────┘                                  │
│  Decision: PASS (appropriate risk warning)              │
│  Latency: 3ms                                           │
└─────────────────────────────────────────────────────────┘
         ↓
    Send to User ✓
```

### If LLM Goes Off-Track (Example)

What if the LLM accidentally says something non-compliant despite our context?

```
Bad Chunk: " I recommend putting 70% in Bitcoin and"

┌─────────────────────────────────────────────────────────┐
│ Chunk X: " I recommend putting 70% in Bitcoin and"     │
│                                                          │
│  Pipeline: fca-midstream-check                          │
│  ┌──────────────────┐                                  │
│  │ advice-detector  │ → Score: 0.95 (ADVICE! ⚠️)       │
│  │ risk-disclosure  │ → Score: 0.1                     │
│  │ misleading       │ → Score: 0.85 (specific rec!)    │
│  └──────────────────┘                                  │
│  Aggregation: first_positive                            │
│  Result: 0.95 (TRIGGERED)                              │
│  Decision: BLOCK & REDACT                               │
│  Latency: 2ms (early exit on first_positive)           │
└─────────────────────────────────────────────────────────┘
         ↓
    [REDACTED - Content does not meet our compliance standards]
         ↓
    Close stream immediately
```

The user sees:
```
I understand you're considering crypto investments. While I can provide
general information, I cannot give personalized investment advice.
Cryptocurrencies are highly volatile and you could lose all [REDACTED]
```

And the conversation is terminated safely.

---

## Phase 3: Egress (Finalization)

### What Happens

After the full response is generated, CheckStream:
1. Adds compliance footers
2. Logs to audit trail
3. Generates evidence for regulatory review

### Pipeline Configuration

```yaml
pipelines:
  fca-egress-finalization:
    description: "Post-generation compliance and audit"
    stages:
      # Full content analysis (no streaming pressure, can be thorough)
      - type: sequential
        name: comprehensive-review
        classifiers:
          - complete-content-analyzer    # Tier C: 8ms (OK here)
          - fca-duty-validator           # Tier C: 7ms
          - audit-trail-generator        # Tier B: 3ms
```

### Execution

```
Full Response (completed):
─────────────────────────────────────────────────────
I understand you're considering crypto investments. While I can
provide general information, I cannot give personalized investment
advice. Cryptocurrencies are highly volatile and you could lose
all your money. You should speak to an FCA-regulated financial
adviser who can assess your individual circumstances.
─────────────────────────────────────────────────────

┌──────────────────────────────────────────────────────┐
│ Stage: comprehensive-review (sequential)             │
│                                                       │
│  Step 1: complete-content-analyzer (8ms)            │
│  ┌────────────────────────────────────────┐         │
│  │ Full message analysis:                  │         │
│  │  ✓ Appropriate disclaimers              │         │
│  │  ✓ Risk warnings included               │         │
│  │  ✓ Referral to FCA adviser              │         │
│  │  ✓ No personalized advice given         │         │
│  │  Score: 0.95 (COMPLIANT)               │         │
│  └────────────────────────────────────────┘         │
│                                                       │
│  Step 2: fca-duty-validator (7ms)                   │
│  ┌────────────────────────────────────────┐         │
│  │ Consumer Duty Checklist:                │         │
│  │  ✓ Act in good faith                    │         │
│  │  ✓ Avoid foreseeable harm               │         │
│  │  ✓ Enable informed decisions            │         │
│  │  ✓ Provide fair value                   │         │
│  │  Score: 0.92 (PASS)                     │         │
│  └────────────────────────────────────────┘         │
│                                                       │
│  Step 3: audit-trail-generator (3ms)                │
│  ┌────────────────────────────────────────┐         │
│  │ Generated audit record:                 │         │
│  │  - Hash: abc123def456...                │         │
│  │  - Timestamp: 2025-11-13T20:15:32Z     │         │
│  │  - User ID: user_789 (hashed)          │         │
│  │  - Risk Level: HIGH                     │         │
│  │  - Compliance: PASS                     │         │
│  │  - Stored: Immutable ledger             │         │
│  └────────────────────────────────────────┘         │
│                                                       │
│  Total Latency: 18ms (not in user-facing path)     │
└──────────────────────────────────────────────────────┘
```

### Footer Injection

CheckStream appends a compliance footer:

```
Final Response Sent to User:
─────────────────────────────────────────────────────
I understand you're considering crypto investments. While I can
provide general information, I cannot give personalized investment
advice. Cryptocurrencies are highly volatile and you could lose
all your money. You should speak to an FCA-regulated financial
adviser who can assess your individual circumstances.

─────────────────────────────────────────────────────
ℹ️ This is general information only, not financial advice.
📞 Speak to an FCA-regulated adviser: www.unbiased.co.uk
⚠️ Investments can go down as well as up. You may lose money.
🔒 Your conversation is logged for compliance purposes.

Interaction ID: CS-20251113-789ABC (for your records)
─────────────────────────────────────────────────────
```

### Audit Trail

In the background (asynchronous), CheckStream creates an immutable audit record:

```json
{
  "interaction_id": "CS-20251113-789ABC",
  "timestamp": "2025-11-13T20:15:32Z",
  "user_id_hash": "sha256:abc123...",
  "session_id": "sess_xyz789",

  "phase_1_ingress": {
    "pipeline": "fca-ingress-check",
    "detected_risk": "investment-advice-request",
    "risk_score": 0.95,
    "action": "context_modification",
    "latency_ms": 6,
    "classifiers_triggered": ["regulated-topic", "financial-risk-classifier"]
  },

  "phase_2_midstream": {
    "pipeline": "fca-midstream-check",
    "chunks_analyzed": 8,
    "chunks_redacted": 0,
    "total_latency_ms": 24,
    "flags_raised": []
  },

  "phase_3_egress": {
    "pipeline": "fca-egress-finalization",
    "compliance_score": 0.92,
    "consumer_duty_pass": true,
    "footer_added": true,
    "latency_ms": 18,
    "audit_hash": "sha256:def456..."
  },

  "regulatory_evidence": {
    "prompt_hash": "sha256:...",
    "response_hash": "sha256:...",
    "full_transcript_encrypted": "vault://path/to/encrypted",
    "retention_until": "2032-11-13",
    "fca_duty_compliant": true
  }
}
```

This record is:
- **Cryptographically signed** (tamper-proof)
- **Stored immutably** (cannot be modified)
- **Retrievable for audits** (regulators can request)
- **Privacy-preserving** (user IDs hashed)

---

## Complete Timeline

```
Time  Phase              Action                           Latency
────  ─────              ──────                           ───────
0ms   User              Submits question

6ms   Phase 1 (Ingress) Validates prompt                 6ms
                        Detects investment advice
                        Modifies LLM context

6ms   LLM               Starts generation

50ms  Phase 2 (Midstream) Checks chunk 1                 3ms
53ms                      ✓ Pass, send to user
56ms                      Checks chunk 2                 3ms
59ms                      ✓ Pass, send to user
...                       (continues for all chunks)
150ms                     Checks chunk 8                 3ms
153ms                     ✓ Pass, send to user

153ms LLM               Generation complete

153ms Phase 3 (Egress)  Comprehensive analysis          18ms
                        Adds compliance footer
                        Generates audit trail

171ms User              Receives complete response

171ms Background        Stores audit record
                        Sends to compliance DB
                        Updates metrics
```

**Total User-Facing Latency**: 171ms
- LLM generation: ~147ms (normal)
- CheckStream overhead: ~24ms in critical path (6ms + 18ms)
- Per-chunk checks: 3ms each (concurrent with user reading)

---

## Why This Matters for FCA Compliance

### Consumer Duty Requirements

The FCA's Consumer Duty requires firms to:

1. **Act in good faith** ✓
   - Phase 1 detects risky topics and modifies context
   - Ensures LLM is set up to be helpful but compliant

2. **Avoid foreseeable harm** ✓
   - Phase 2 catches inappropriate advice in real-time
   - Prevents user from receiving harmful guidance

3. **Enable customers to make informed decisions** ✓
   - Phase 3 ensures risk warnings are present
   - Adds clear disclaimers and adviser referrals

4. **Provide fair value** ✓
   - Audit trail proves due diligence
   - Shows firm took reasonable steps

### Regulatory Evidence

If the FCA audits this interaction, the firm can show:

- ✅ Prompt was analyzed for risk (Phase 1)
- ✅ Real-time monitoring during generation (Phase 2)
- ✅ Post-generation validation (Phase 3)
- ✅ Appropriate disclaimers added
- ✅ User referred to regulated adviser
- ✅ Complete audit trail maintained
- ✅ All Consumer Duty principles met

**This is defensible evidence that the firm acted reasonably.**

---

## Pipeline Configuration Summary

### config.yaml
```yaml
# Main proxy configuration
proxy:
  phases:
    ingress:
      pipeline: fca-ingress-check
      timeout_ms: 10

    midstream:
      pipeline: fca-midstream-check
      timeout_ms: 5
      per_chunk: true

    egress:
      pipeline: fca-egress-finalization
      timeout_ms: 50
      async: true  # Not in critical path

  thresholds:
    ingress_block: 0.99        # Very high bar (rarely block)
    ingress_modify: 0.7        # Modify context instead
    midstream_redact: 0.8      # Per-chunk redaction
    egress_footer_required: 0.6  # When to add footer
```

---

## Key Takeaways

1. **Three Phases = Defense in Depth**
   - Phase 1: Set up LLM for success
   - Phase 2: Catch mistakes in real-time
   - Phase 3: Prove compliance

2. **Different Goals Per Phase**
   - Ingress: Proactive (modify context)
   - Midstream: Reactive (block if needed)
   - Egress: Evidential (create record)

3. **Latency Budget Allocation**
   - Ingress: Fast (6-8ms)
   - Midstream: Ultra-fast per chunk (3-5ms)
   - Egress: Flexible (can be 20-50ms)

4. **Regulatory Compliance**
   - Meets FCA Consumer Duty requirements
   - Creates audit trail
   - Provides defensible evidence
   - Privacy-preserving

---

## See Also

- [Pipeline Configuration Guide](pipeline-configuration.md)
- [Integration Guide](INTEGRATION_GUIDE.md)
- [Regulatory Compliance](regulatory-compliance.md)
