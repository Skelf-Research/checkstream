# Policy Language Reference

Complete syntax reference for CheckStream policies.

---

## Policy File Structure

```yaml
version: "1.0"                    # Required: Policy format version
name: "policy-name"               # Required: Unique policy name
description: "Description"        # Optional: Human-readable description

# Global defaults
defaults:
  mode: enforce
  priority: 50

# Policy rules
policies:
  - name: rule_name
    # ... rule definition
```

---

## Rule Definition

### Required Fields

```yaml
policies:
  - name: "unique_rule_name"      # Required: Unique identifier
    trigger: ...                   # Required: Activation condition
    action: ...                    # Required: What to do
```

### Optional Fields

```yaml
policies:
  - name: "rule_name"
    trigger: ...
    action: ...

    # Optional fields
    phase: ingress                 # ingress, midstream, egress, or all
    mode: enforce                  # enforce, shadow, disabled
    priority: 50                   # 0-100, higher = first
    message: "User message"        # Message for stop action
    regulation: "FCA COBS 9A"      # Regulatory reference
    tags: ["safety", "pii"]        # For filtering/reporting
    continue: false                # Continue to next rule after match
```

---

## Triggers

### Classifier Trigger

```yaml
trigger:
  classifier: toxicity            # Classifier name
  threshold: 0.8                  # Score threshold (0.0-1.0)
```

### Threshold Range

```yaml
trigger:
  classifier: toxicity
  min_threshold: 0.5              # Minimum score
  max_threshold: 0.8              # Maximum score
```

### Label Trigger

```yaml
trigger:
  classifier: sentiment
  label: negative                 # Expected label
  confidence: 0.7                 # Minimum confidence
```

### Pattern Trigger

```yaml
trigger:
  pattern: '\b(secret|password)\b'
  case_insensitive: true          # Default: false
  multiline: false                # Default: false
```

### Keyword Trigger

```yaml
trigger:
  keywords:
    - "ignore previous"
    - "system prompt"
  match: any                      # any or all
  case_insensitive: true
```

---

## Compound Triggers

### AND Logic

```yaml
trigger:
  all:
    - classifier: toxicity
      threshold: 0.6
    - classifier: contains_slur
      threshold: 0.5
```

### OR Logic

```yaml
trigger:
  any:
    - classifier: toxicity
      threshold: 0.8
    - classifier: hate_speech
      threshold: 0.7
```

### NOT Logic

```yaml
trigger:
  all:
    - classifier: toxicity
      threshold: 0.7
    - not:
        classifier: satire
        threshold: 0.8
```

### Nested Logic

```yaml
trigger:
  all:
    - classifier: advice
      threshold: 0.5
    - any:
        - classifier: financial
          threshold: 0.6
        - classifier: medical
          threshold: 0.6
    - not:
        pattern: 'general information'
```

---

## Actions

### Stop Action

Block request or stop generation.

```yaml
action: stop
message: "Request blocked for safety"
```

With options:

```yaml
action:
  type: stop
  message: "Blocked: ${rule_name}"
  include_scores: true            # Include classifier scores in response
  log_level: warn
```

### Redact Action

Replace content with placeholder.

```yaml
action: redact
replacement: "[REDACTED]"
```

With options:

```yaml
action:
  type: redact
  replacement: "[CONTENT REMOVED]"
  scope: matched                  # matched, word, sentence, paragraph
  preserve_length: false
  marker_style: bracket           # bracket, asterisk, custom
```

### Inject Action

Add content to response.

```yaml
action: inject
position: end                     # start, end, inline
content: |
  ---
  *Disclaimer text here*
```

With options:

```yaml
action:
  type: inject
  position: end
  content: "Disclaimer"
  format: markdown                # plain, markdown, html
  separator: "\n\n---\n\n"
  conditions:
    not_already_present: true     # Don't inject if content exists
```

### Log Action

Record without blocking.

```yaml
action: log
level: warn                       # debug, info, warn, error
```

With options:

```yaml
action:
  type: log
  level: warn
  include:
    - input
    - classifier_scores
    - timestamp
  tags: ["review-needed"]
```

### Audit Action

Create compliance record.

```yaml
action: audit
regulation: "FCA COBS 9A.2.1R"
```

With options:

```yaml
action:
  type: audit
  regulation: "REGULATION"
  include:
    - input_hash
    - output_hash
    - classifier_scores
    - rule_triggered
    - timestamp
    - tenant
  retention_days: 2555            # Override global retention
```

### Notify Action

Send alert (webhook, email, etc.).

```yaml
action:
  type: notify
  channel: webhook
  url: "https://hooks.slack.com/..."
  message: "Safety alert: ${rule_name} triggered"
```

### Transform Action

Modify content.

```yaml
action:
  type: transform
  operation: lowercase            # lowercase, uppercase, trim
```

### Multiple Actions

```yaml
action:
  - type: redact
    replacement: "[PII]"
  - type: log
    level: warn
  - type: audit
    regulation: "GDPR Art. 9"
```

---

## Phases

| Phase | Timing | Available Actions |
|-------|--------|-------------------|
| `ingress` | Before LLM | stop, log, audit, transform |
| `midstream` | During streaming | stop, redact, log |
| `egress` | After completion | inject, log, audit, notify |
| `all` | All phases | Depends on phase |

```yaml
policies:
  - name: ingress_only
    phase: ingress
    trigger: ...
    action: stop

  - name: all_phases
    phase: all
    trigger: ...
    action: log
```

---

## Modes

| Mode | Behavior |
|------|----------|
| `enforce` | Apply action (default) |
| `shadow` | Log only, don't apply |
| `disabled` | Skip entirely |

```yaml
policies:
  - name: production_rule
    mode: enforce
    trigger: ...
    action: stop

  - name: testing_rule
    mode: shadow
    trigger: ...
    action: stop
    # Logs what would happen
```

---

## Variables

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `${input}` | User input text |
| `${output}` | Generated output |
| `${tenant}` | Tenant identifier |
| `${model}` | LLM model name |
| `${timestamp}` | ISO timestamp |
| `${request_id}` | Unique request ID |
| `${rule_name}` | Current rule name |
| `${classifier_name}` | Triggered classifier |
| `${score}` | Classifier score |

### Using Variables

```yaml
policies:
  - name: audit_rule
    trigger: ...
    action:
      type: audit
      metadata:
        tenant: "${tenant}"
        model: "${model}"
        score: "${score}"
        triggered_at: "${timestamp}"
```

---

## Conditions

### Score Conditions

```yaml
trigger:
  classifier: toxicity
  condition:
    score: "> 0.8"                # >, <, >=, <=, ==, !=
```

### Label Conditions

```yaml
trigger:
  classifier: sentiment
  condition:
    label: "in [negative, very_negative]"
```

### Text Conditions

```yaml
trigger:
  condition:
    input_length: "> 1000"
    contains: "specific phrase"
```

---

## Defaults

Set defaults for all rules:

```yaml
defaults:
  mode: enforce
  priority: 50
  phase: all
  continue: false

policies:
  - name: rule1
    # Inherits defaults
    trigger: ...
    action: ...

  - name: rule2
    mode: shadow                  # Override default
    trigger: ...
    action: ...
```

---

## Inheritance

### Base Policies

```yaml
# base.yaml
version: "1.0"
name: "base-policy"

policies:
  - name: base_safety
    trigger:
      classifier: toxicity
      threshold: 0.9
    action: stop
```

### Extended Policies

```yaml
# extended.yaml
version: "1.0"
name: "extended-policy"
extends: "./base.yaml"

policies:
  # Additional rules
  - name: custom_rule
    trigger: ...
    action: ...
```

---

## Comments and Documentation

```yaml
version: "1.0"
name: "documented-policy"

# This policy handles financial compliance
# Author: compliance-team
# Last updated: 2024-01-15

policies:
  # Block specific investment advice
  # Regulation: FCA COBS 9A.2.1R
  - name: block_advice
    trigger:
      classifier: investment_advice
      threshold: 0.8
    action: stop
    message: "Cannot provide investment advice"
    regulation: "FCA COBS 9A.2.1R"
```

---

## Complete Example

```yaml
version: "1.0"
name: "comprehensive-policy"
description: "Full-featured policy example"

defaults:
  mode: enforce
  priority: 50

policies:
  # HIGH PRIORITY: Block dangerous content
  - name: block_dangerous
    priority: 100
    phase: ingress
    trigger:
      any:
        - classifier: prompt_injection
          threshold: 0.85
        - pattern: 'ignore (all )?(previous |prior )?instructions'
          case_insensitive: true
    action: stop
    message: "Request blocked for security review"
    tags: ["security", "critical"]

  # MEDIUM PRIORITY: Redact PII in responses
  - name: redact_pii
    priority: 75
    phase: midstream
    trigger:
      any:
        - classifier: pii_detector
          threshold: 0.9
        - pattern: '\b\d{3}-\d{2}-\d{4}\b'  # SSN
    action:
      - type: redact
        replacement: "[PII REDACTED]"
      - type: audit
        regulation: "GDPR Article 5"

  # LOW PRIORITY: Add disclaimers
  - name: financial_disclaimer
    priority: 25
    phase: egress
    trigger:
      classifier: financial_topic
      threshold: 0.4
    action:
      type: inject
      position: end
      content: |

        ---
        *This is general information, not financial advice.*

  # SHADOW: Test new classifier
  - name: test_new_classifier
    mode: shadow
    trigger:
      classifier: experimental_v2
      threshold: 0.7
    action: stop
    # Logs but doesn't block
```

---

## Validation

Validate policy syntax:

```bash
./checkstream-proxy --validate-policy ./policy.yaml
```

Output:

```
Policy 'comprehensive-policy' validated successfully.
- 4 rules defined
- 0 errors
- 1 warning: Rule 'test_new_classifier' references unknown classifier 'experimental_v2'
```

---

## Next Steps

- [Policy Engine Guide](../guides/policy-engine.md) - Practical examples
- [API Reference](api.md) - Test policies via API
- [Compliance Guide](../guides/compliance.md) - Regulatory policies
