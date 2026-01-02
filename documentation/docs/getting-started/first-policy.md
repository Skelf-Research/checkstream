# Your First Policy

Learn to create custom safety policies for your use case.

---

## Policy Structure

A CheckStream policy defines rules that trigger actions based on classifier results:

```yaml
version: "1.0"
name: "my-policy"

policies:
  - name: rule_name
    trigger:
      classifier: classifier_name
      threshold: 0.8
    action: stop|redact|log|inject
    message: "Optional message"
```

---

## Example: Content Moderation Policy

Create a policy that moderates inappropriate content:

```yaml
version: "1.0"
name: "content-moderation"

policies:
  # Block toxic content in prompts
  - name: block_toxic_prompts
    phase: ingress
    trigger:
      classifier: toxicity
      threshold: 0.85
    action: stop
    message: "Please rephrase your request without offensive language"

  # Redact toxic content in responses
  - name: redact_toxic_responses
    phase: midstream
    trigger:
      classifier: toxicity
      threshold: 0.7
    action: redact
    replacement: "[CONTENT MODERATED]"

  # Log borderline cases
  - name: log_borderline_content
    phase: egress
    trigger:
      classifier: toxicity
      threshold: 0.5
      max_threshold: 0.7
    action: log
    level: warn
```

---

## Available Actions

| Action | Description | Phases |
|--------|-------------|--------|
| `stop` | Block request/response | Ingress, Midstream |
| `redact` | Replace matched content | Midstream |
| `inject` | Add content to response | Egress |
| `log` | Record for analysis | All |
| `audit` | Create compliance record | Egress |

---

## Trigger Types

### Single Classifier

```yaml
trigger:
  classifier: toxicity
  threshold: 0.8
```

### Multiple Classifiers (AND)

```yaml
trigger:
  all:
    - classifier: toxicity
      threshold: 0.7
    - classifier: sentiment
      threshold: 0.6
      condition: negative
```

### Multiple Classifiers (OR)

```yaml
trigger:
  any:
    - classifier: toxicity
      threshold: 0.8
    - classifier: hate_speech
      threshold: 0.7
```

### Pattern Matching

```yaml
trigger:
  pattern: "\\b(password|secret|api.?key)\\b"
  case_insensitive: true
```

---

## Built-in Classifiers

| Classifier | Tier | Description |
|------------|------|-------------|
| `toxicity` | B | Offensive language detection |
| `sentiment` | B | Positive/negative sentiment |
| `prompt_injection` | A/B | Jailbreak attempt detection |
| `pii_detector` | A | Personal information (SSN, emails, etc.) |
| `financial_advice` | B | Regulated financial advice |

---

## Example: Financial Compliance Policy

```yaml
version: "1.0"
name: "fca-compliance"

policies:
  # Block unqualified financial advice
  - name: block_investment_advice
    trigger:
      classifier: financial_advice
      threshold: 0.75
    action: stop
    message: "I cannot provide specific investment advice. Please consult a qualified financial advisor."
    regulation: "FCA COBS 9A.2.1R"

  # Redact specific stock recommendations
  - name: redact_stock_tips
    trigger:
      pattern: "\\b(buy|sell|hold)\\s+(stock|shares?)\\s+in\\s+\\w+"
    action: redact
    replacement: "[INVESTMENT ADVICE REDACTED]"

  # Add compliance disclaimer
  - name: add_disclaimer
    phase: egress
    trigger:
      classifier: financial_advice
      threshold: 0.3
    action: inject
    position: end
    content: "\n\n---\nThis is general information only, not financial advice. Past performance does not guarantee future results."
```

---

## Example: PII Protection Policy

```yaml
version: "1.0"
name: "pii-protection"

policies:
  # Block PII in prompts
  - name: block_pii_prompts
    phase: ingress
    trigger:
      classifier: pii_detector
      threshold: 0.9
    action: stop
    message: "Please do not include personal information in your request"

  # Redact PII in responses
  - name: redact_pii_responses
    phase: midstream
    trigger:
      any:
        - pattern: "\\b\\d{3}-\\d{2}-\\d{4}\\b"  # SSN
        - pattern: "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b"  # Email
        - pattern: "\\b\\d{4}[- ]?\\d{4}[- ]?\\d{4}[- ]?\\d{4}\\b"  # Credit card
    action: redact
    replacement: "[PII REDACTED]"
```

---

## Testing Your Policy

### Dry Run Mode

Test policies without enforcement:

```yaml
policies:
  - name: test_rule
    mode: shadow  # Log only, don't enforce
    trigger:
      classifier: toxicity
      threshold: 0.7
    action: stop
```

### Local Testing

Use the test endpoint:

```bash
curl http://localhost:8080/admin/test-policy \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Your test input here",
    "policy": "content-moderation"
  }'
```

Response shows which rules would trigger:

```json
{
  "matches": [
    {
      "rule": "redact_toxic_responses",
      "score": 0.82,
      "action": "redact"
    }
  ]
}
```

---

## Policy Best Practices

1. **Start with shadow mode** - Test policies before enforcement
2. **Use appropriate thresholds** - Too low = false positives, too high = misses
3. **Layer your defenses** - Combine patterns (fast) with ML (accurate)
4. **Document regulations** - Include regulation references for compliance
5. **Monitor and tune** - Review logs and adjust thresholds

---

## Next Steps

- [Policy Language Reference](../reference/policy-language.md) - Complete syntax reference
- [Classifier System](../architecture/classifiers.md) - How classifiers work
- [Regulatory Compliance](../guides/compliance.md) - Pre-built compliance packs
