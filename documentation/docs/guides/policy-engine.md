# Policy Engine Guide

Learn to write effective safety policies for CheckStream.

---

## Overview

The policy engine evaluates classifier outputs and determines actions. Policies are defined in YAML and can be hot-reloaded without restart.

```yaml
version: "1.0"
name: "my-policy"

policies:
  - name: rule_name
    trigger: ...
    action: ...
```

---

## Policy Structure

### Basic Policy

```yaml
policies:
  - name: block_toxicity
    trigger:
      classifier: toxicity
      threshold: 0.8
    action: stop
    message: "Content blocked for safety"
```

### Policy Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique rule identifier |
| `trigger` | Yes | Condition to activate rule |
| `action` | Yes | What to do when triggered |
| `phase` | No | Limit to specific phase |
| `mode` | No | `enforce`, `shadow`, `disabled` |
| `message` | No | User-facing message |
| `regulation` | No | Regulatory reference |
| `priority` | No | Rule evaluation order |

---

## Trigger Types

### Classifier Trigger

```yaml
trigger:
  classifier: toxicity
  threshold: 0.8
```

### Threshold Range

```yaml
trigger:
  classifier: toxicity
  min_threshold: 0.5    # Minimum score
  max_threshold: 0.8    # Maximum score
```

### Pattern Trigger

```yaml
trigger:
  pattern: '\b(password|secret)\b'
  case_insensitive: true
```

### Label Trigger

```yaml
trigger:
  classifier: sentiment
  label: negative
  confidence: 0.7
```

---

## Compound Triggers

### All Conditions (AND)

```yaml
trigger:
  all:
    - classifier: toxicity
      threshold: 0.6
    - classifier: sentiment
      label: negative
      confidence: 0.7
```

### Any Condition (OR)

```yaml
trigger:
  any:
    - classifier: toxicity
      threshold: 0.8
    - classifier: hate_speech
      threshold: 0.7
```

### Nested Logic

```yaml
trigger:
  all:
    - classifier: contains_advice
      threshold: 0.5
    - any:
        - classifier: financial_advice
          threshold: 0.7
        - classifier: medical_advice
          threshold: 0.7
```

### NOT Condition

```yaml
trigger:
  all:
    - classifier: toxicity
      threshold: 0.7
    - not:
        classifier: satire_detector
        threshold: 0.8
```

---

## Actions

### Stop Action

Block request or stop generation:

```yaml
action: stop
message: "Request blocked for safety"
```

### Redact Action

Replace content with placeholder:

```yaml
action: redact
replacement: "[CONTENT REMOVED]"
```

Advanced redaction:

```yaml
action: redact
options:
  replacement: "[REDACTED]"
  scope: matched        # matched, sentence, paragraph, all
  preserve_length: false
```

### Inject Action

Add content to response:

```yaml
action: inject
position: end           # start, end, inline
content: |
  ---
  *Disclaimer: This is not professional advice.*
```

### Log Action

Record for analysis without blocking:

```yaml
action: log
level: warn             # debug, info, warn, error
include_context: true
```

### Audit Action

Create compliance record:

```yaml
action: audit
include:
  - input
  - output
  - classifier_scores
  - timestamp
regulation: "FCA COBS 9A.2.1R"
```

### Multiple Actions

```yaml
action:
  - type: redact
    replacement: "[PII REMOVED]"
  - type: log
    level: warn
  - type: audit
    regulation: "GDPR Article 9"
```

---

## Phase-Specific Policies

### Ingress Only

```yaml
policies:
  - name: block_injection
    phase: ingress
    trigger:
      classifier: prompt_injection
      threshold: 0.8
    action: stop
```

### Midstream Only

```yaml
policies:
  - name: redact_pii
    phase: midstream
    trigger:
      classifier: pii_detector
      threshold: 0.9
    action: redact
```

### Egress Only

```yaml
policies:
  - name: add_disclaimer
    phase: egress
    trigger:
      classifier: financial_advice
      threshold: 0.3
    action: inject
    position: end
    content: "\n\n*Not financial advice.*"
```

---

## Policy Modes

### Enforce Mode (Default)

```yaml
policies:
  - name: strict_safety
    mode: enforce
    trigger: ...
    action: stop
```

### Shadow Mode (Test)

Log what would happen without enforcing:

```yaml
policies:
  - name: test_rule
    mode: shadow
    trigger:
      classifier: new_classifier
      threshold: 0.7
    action: stop
    # Logs trigger but doesn't block
```

### Disabled Mode

```yaml
policies:
  - name: deprecated_rule
    mode: disabled
```

---

## Priority and Ordering

Higher priority rules are evaluated first:

```yaml
policies:
  - name: critical_safety
    priority: 100
    trigger: ...
    action: stop

  - name: moderate_check
    priority: 50
    trigger: ...
    action: log

  - name: low_priority
    priority: 10
    trigger: ...
    action: audit
```

First matching rule wins (unless `continue: true`):

```yaml
policies:
  - name: log_everything
    priority: 100
    trigger:
      classifier: any
      threshold: 0
    action: log
    continue: true    # Continue to next rule

  - name: block_severe
    priority: 50
    trigger:
      classifier: toxicity
      threshold: 0.9
    action: stop      # Stops evaluation
```

---

## Variables and Context

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `${input}` | User input text |
| `${output}` | Generated output |
| `${tenant}` | Tenant identifier |
| `${model}` | LLM model name |
| `${timestamp}` | Current timestamp |

### Using Variables

```yaml
policies:
  - name: audit_with_context
    trigger:
      classifier: financial_advice
      threshold: 0.5
    action: audit
    metadata:
      tenant: "${tenant}"
      model: "${model}"
      timestamp: "${timestamp}"
```

---

## Real-World Examples

### Financial Compliance

```yaml
version: "1.0"
name: "fca-compliance"

policies:
  - name: block_specific_advice
    phase: ingress
    trigger:
      all:
        - classifier: financial_advice
          threshold: 0.8
        - pattern: '\b(buy|sell|invest)\s+(in|into)\b'
    action: stop
    message: "I cannot provide specific investment recommendations."
    regulation: "FCA COBS 9A.2.1R"

  - name: redact_projections
    phase: midstream
    trigger:
      pattern: '\b\d+%\s+(return|growth|yield)\b'
    action: redact
    replacement: "[PROJECTION REDACTED]"

  - name: add_risk_warning
    phase: egress
    trigger:
      classifier: investment_discussion
      threshold: 0.3
    action: inject
    position: end
    content: |

      ---
      **Risk Warning**: Past performance is not a guide to future performance.
      The value of investments can fall as well as rise.
```

### Healthcare Compliance

```yaml
version: "1.0"
name: "hipaa-compliance"

policies:
  - name: block_phi_requests
    phase: ingress
    trigger:
      pattern: '(patient|medical)\s+record'
    action: stop
    message: "I cannot access or discuss specific patient records."

  - name: redact_phi
    phase: midstream
    trigger:
      any:
        - classifier: pii_detector
          threshold: 0.9
        - pattern: '\b(MRN|DOB|SSN)[\s:]+\S+'
    action: redact
    replacement: "[PHI REDACTED]"

  - name: medical_disclaimer
    phase: egress
    trigger:
      classifier: medical_advice
      threshold: 0.4
    action: inject
    position: end
    content: |

      ---
      *This information is for educational purposes only and is not a substitute
      for professional medical advice. Please consult a healthcare provider.*
```

### Content Moderation

```yaml
version: "1.0"
name: "content-moderation"

policies:
  - name: block_hate_speech
    trigger:
      classifier: hate_speech
      threshold: 0.85
    action: stop
    message: "This content violates our community guidelines."

  - name: redact_profanity
    phase: midstream
    trigger:
      classifier: profanity
      threshold: 0.9
    action: redact
    replacement: "****"

  - name: flag_borderline
    trigger:
      classifier: toxicity
      min_threshold: 0.5
      max_threshold: 0.85
    action:
      - type: log
        level: warn
      - type: audit
        metadata:
          review_required: true
```

---

## Testing Policies

### Validate Syntax

```bash
./checkstream-proxy --validate-policy ./policies/my-policy.yaml
```

### Test Against Input

```bash
curl http://localhost:8080/admin/test-policy \
  -H "Content-Type: application/json" \
  -d '{
    "policy": "fca-compliance",
    "text": "You should buy AAPL stock",
    "phase": "ingress"
  }'
```

### Shadow Mode Analysis

```bash
# Enable shadow mode for new policy
# Review logs for trigger patterns
grep "shadow_trigger" /var/log/checkstream/*.log | jq .
```

---

## Best Practices

1. **Start with shadow mode** - Test before enforcing
2. **Use specific patterns** - Avoid over-broad triggers
3. **Layer defenses** - Multiple rules for important cases
4. **Document regulations** - Include `regulation` field
5. **Set appropriate thresholds** - Balance safety vs usability
6. **Use phases wisely** - Fast checks in ingress, heavy in egress
7. **Review regularly** - Update thresholds based on data

---

## Next Steps

- [Policy Language Reference](../reference/policy-language.md) - Complete syntax
- [Regulatory Compliance](compliance.md) - Pre-built compliance packs
- [Pipeline Configuration](../configuration/pipelines.md) - Classifier pipelines
