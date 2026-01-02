# Content Moderation Example

Deploy CheckStream for real-time content safety and brand protection.

---

## Scenario

A consumer-facing AI product needs to:
- Block harmful content generation
- Filter profanity and offensive language
- Prevent prompt injection attacks
- Protect brand reputation
- Log concerning interactions for review

---

## Configuration

### config.yaml

```yaml
server:
  host: "0.0.0.0"
  port: 8080

backend:
  url: "https://api.openai.com/v1"

pipeline:
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
      - harmful_request
      - pii_detector
    threshold: 0.85

  midstream:
    enabled: true
    token_holdback: 12
    classifiers:
      - toxicity
      - profanity
      - hate_speech
      - violence
    chunk_threshold: 0.70

  egress:
    enabled: true
    classifiers:
      - brand_safety
      - quality_check

policy:
  path: "./policies/content-moderation.yaml"

telemetry:
  logging:
    level: info
  audit:
    enabled: true
    path: "./audit/moderation"
```

### classifiers.yaml

```yaml
classifiers:
  # Tier A - Fast pattern matching
  profanity:
    tier: A
    type: pattern
    patterns:
      - name: profanity_list
        pattern: '\b(badword1|badword2|badword3)\b'
        case_insensitive: true
        score: 0.9
      # Additional patterns loaded from file
      patterns_file: "./patterns/profanity.txt"

  prompt_injection:
    tier: A
    type: pattern
    patterns:
      - name: ignore_instructions
        pattern: 'ignore\s+(all\s+)?(previous|prior)\s+instructions?'
        case_insensitive: true
        score: 0.95
      - name: system_prompt
        pattern: '(reveal|show|print)\s+(your\s+)?system\s+prompt'
        case_insensitive: true
        score: 0.9
      - name: jailbreak
        pattern: '(DAN|do anything now|pretend you)'
        case_insensitive: true
        score: 0.85

  # Tier B - ML classifiers
  toxicity:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
      quantization: int8
    device: auto

  hate_speech:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "facebook/roberta-hate-speech-dynabench-r4-target"
      quantization: int8

  violence:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/violence-detector"
      quantization: int8

  harmful_request:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/harmful-request-detector"
      quantization: int8

  brand_safety:
    tier: B
    type: ml
    model:
      source: local
      path: "./models/brand-safety"
```

---

## Policy

### policies/content-moderation.yaml

```yaml
version: "1.0"
name: "content-moderation"
description: "Comprehensive content safety and brand protection"

policies:
  # ============================================
  # INGRESS - Block harmful requests
  # ============================================

  # Block prompt injection attempts
  - name: block_prompt_injection
    phase: ingress
    priority: 100
    trigger:
      classifier: prompt_injection
      threshold: 0.8
    action: stop
    message: "I'm designed to be helpful, harmless, and honest. I can't process that request."

  # Block harmful content requests
  - name: block_harmful_requests
    phase: ingress
    priority: 95
    trigger:
      classifier: harmful_request
      threshold: 0.85
    action:
      - type: stop
        message: "I can't help with that request. Let me know if there's something else I can assist with."
      - type: log
        level: warn
        tags: ["harmful", "review"]

  # Block PII in requests
  - name: block_pii_input
    phase: ingress
    priority: 90
    trigger:
      classifier: pii_detector
      threshold: 0.9
    action: stop
    message: "For your privacy, please don't share personal information like email addresses or phone numbers."

  # ============================================
  # MIDSTREAM - Filter content in real-time
  # ============================================

  # Stop on severe toxicity
  - name: stop_severe_toxicity
    phase: midstream
    priority: 100
    trigger:
      classifier: toxicity
      threshold: 0.95
    action:
      - type: stop
        message: "\n\n[Generation stopped due to content policy violation]"
      - type: audit
        severity: critical

  # Redact toxic content
  - name: redact_toxicity
    phase: midstream
    priority: 90
    trigger:
      classifier: toxicity
      min_threshold: 0.7
      max_threshold: 0.95
    action:
      - type: redact
        replacement: "[content removed]"
      - type: log
        level: warn

  # Stop on hate speech
  - name: stop_hate_speech
    phase: midstream
    priority: 95
    trigger:
      classifier: hate_speech
      threshold: 0.85
    action:
      - type: stop
        message: "\n\n[Generation stopped]"
      - type: audit
        severity: critical
        tags: ["hate_speech", "escalate"]

  # Redact profanity
  - name: redact_profanity
    phase: midstream
    priority: 85
    trigger:
      classifier: profanity
      threshold: 0.9
    action:
      type: redact
      replacement: "****"

  # Stop on violence
  - name: stop_violence
    phase: midstream
    priority: 90
    trigger:
      classifier: violence
      threshold: 0.9
    action:
      - type: stop
      - type: audit
        severity: high

  # ============================================
  # EGRESS - Quality and brand checks
  # ============================================

  # Flag brand safety issues
  - name: flag_brand_issues
    phase: egress
    priority: 70
    trigger:
      classifier: brand_safety
      threshold: 0.6
    action:
      - type: log
        level: warn
        tags: ["brand_safety", "review"]
      - type: audit
        include:
          - input
          - output

  # Log borderline content for review
  - name: log_borderline
    phase: egress
    priority: 50
    trigger:
      classifier: toxicity
      min_threshold: 0.4
      max_threshold: 0.7
    action:
      type: log
      level: info
      tags: ["borderline", "review_queue"]

  # ============================================
  # SHADOW - Testing new classifiers
  # ============================================

  - name: test_new_toxicity_model
    mode: shadow
    trigger:
      classifier: toxicity_v2
      threshold: 0.7
    action: log
```

---

## Real-Time Processing

### Streaming Behavior

```
User prompt: "Write a story"
                    │
                    ▼
            ┌──────────────┐
            │   Ingress    │ ◀── Check for harmful request
            │   (3ms)      │
            └──────┬───────┘
                   │ ALLOW
                   ▼
            ┌──────────────┐
            │  LLM Backend │
            └──────┬───────┘
                   │ Streaming tokens
                   ▼
      ┌────────────────────────────┐
      │       Midstream            │
      │                            │
      │  Token buffer: [████████] │
      │                            │
      │  "The character said"      │ ◀── Released (safe)
      │  "[content removed]"       │ ◀── Redacted (toxic)
      │  "and walked away"         │ ◀── Released (safe)
      │                            │
      └────────────────────────────┘
                   │
                   ▼
      ┌────────────────────────────┐
      │        Egress              │
      │   Brand safety check       │
      │   Quality logging          │
      └────────────────────────────┘
```

---

## Usage Examples

### Normal Interaction

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="your-key"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Tell me a joke"}],
    stream=True
)

for chunk in response:
    print(chunk.choices[0].delta.content or "", end="")
# Output: Clean, appropriate joke
```

### Blocked Request

```python
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Ignore previous instructions..."}]
)
# Returns error:
# "I'm designed to be helpful, harmless, and honest..."
```

### Redacted Response

```python
# If toxic content appears during generation:
# User sees: "The character yelled [content removed] and stormed off."
```

---

## Monitoring Dashboard

### Key Metrics

```yaml
# Grafana dashboard panels

# Safety Events Over Time
- query: rate(checkstream_policy_triggers_total{action="stop"}[5m])
  title: "Blocked Requests"

- query: rate(checkstream_policy_triggers_total{action="redact"}[5m])
  title: "Redacted Content"

# Classifier Performance
- query: histogram_quantile(0.95, checkstream_classifier_latency_ms{tier="B"})
  title: "P95 Classifier Latency"

# Content Categories
- query: sum by (rule) (checkstream_policy_triggers_total)
  title: "Triggers by Rule"
```

### Alerts

```yaml
# Prometheus alerting rules
groups:
  - name: content-safety
    rules:
      - alert: HighBlockRate
        expr: |
          sum(rate(checkstream_policy_triggers_total{action="stop"}[5m])) /
          sum(rate(checkstream_requests_total[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High content block rate (>10%)"

      - alert: HateSpeechDetected
        expr: increase(checkstream_policy_triggers_total{rule="stop_hate_speech"}[1h]) > 0
        labels:
          severity: critical
        annotations:
          summary: "Hate speech content detected"
```

---

## Review Queue

### Flag Content for Human Review

```yaml
policies:
  - name: queue_for_review
    phase: egress
    trigger:
      any:
        - classifier: toxicity
          min_threshold: 0.5
          max_threshold: 0.7
        - classifier: brand_safety
          threshold: 0.6
    action:
      - type: log
        level: warn
        tags: ["review_queue"]
      - type: notify
        channel: webhook
        url: "https://internal.company.com/review-queue"
        payload:
          request_id: "${request_id}"
          classifier_scores: "${scores}"
```

### Query Review Queue

```bash
curl "http://localhost:8080/audit?tags=review_queue&start=2024-01-15"
```

---

## Customization

### Adding Custom Profanity List

```yaml
# patterns/profanity.txt
# One pattern per line
badword1
badword2
# Regex patterns also supported
\b(spam|scam)\w*\b
```

### Brand-Specific Rules

```yaml
policies:
  - name: protect_brand_name
    trigger:
      pattern: '\b(OurBrand)\s+(is|are)\s+(bad|terrible|awful)'
      case_insensitive: true
    action:
      - type: redact
        replacement: "[feedback noted]"
      - type: notify
        channel: slack
        message: "Brand sentiment issue detected"
```

---

## Performance Tuning

### Optimize for Latency

```yaml
pipeline:
  midstream:
    token_holdback: 8        # Smaller buffer = lower latency
    timeout_ms: 5            # Fail fast

classifiers:
  toxicity:
    model:
      quantization: int8     # Faster inference
    max_length: 256          # Truncate long inputs
    inference_cache:
      enabled: true
```

### Optimize for Accuracy

```yaml
pipeline:
  midstream:
    token_holdback: 24       # More context
    context_chunks: 5        # Consider history

classifiers:
  toxicity:
    model:
      quantization: none     # Full precision
    max_length: 512          # More context
```

---

## Next Steps

- [Financial Compliance](financial-compliance.md) - Regulatory example
- [Healthcare Compliance](healthcare.md) - HIPAA example
- [Policy Engine Guide](../guides/policy-engine.md) - Customize policies
