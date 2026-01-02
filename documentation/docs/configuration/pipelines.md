# Pipeline Configuration

Configure how classifiers work together in CheckStream pipelines.

---

## Pipeline Basics

A pipeline defines how classifiers are executed and how their results are combined.

```yaml
pipelines:
  content_safety:
    type: parallel
    classifiers:
      - toxicity
      - prompt_injection
    aggregation: max_score
```

---

## Pipeline Types

### Single Classifier

Run a single classifier:

```yaml
pipelines:
  simple_check:
    type: single
    classifier: toxicity
```

### Parallel Pipeline

Run multiple classifiers concurrently:

```yaml
pipelines:
  content_safety:
    type: parallel
    classifiers:
      - toxicity
      - prompt_injection
      - pii_detector
    aggregation: max_score
    timeout_ms: 50
```

### Sequential Pipeline

Run classifiers in order with early exit:

```yaml
pipelines:
  tiered_safety:
    type: sequential
    stages:
      - classifier: pii_detector        # Fast pattern check
        exit_on: match

      - classifier: prompt_injection    # ML check
        exit_on: threshold
        threshold: 0.9

      - classifier: toxicity            # Final check
```

### Conditional Pipeline

Run classifiers based on conditions:

```yaml
pipelines:
  smart_routing:
    type: conditional
    stages:
      - classifier: quick_filter
        conditions:
          - when: score > 0.5
            then:
              - classifier: detailed_analysis
          - when: score <= 0.5
            then: skip
```

---

## Aggregation Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| `max_score` | Highest score wins | Conservative safety |
| `min_score` | Lowest score wins | Permissive checks |
| `average` | Mean of all scores | Balanced approach |
| `weighted_average` | Weighted mean | Prioritize certain classifiers |
| `first_positive` | First score > threshold | Early detection |
| `unanimous` | All must agree | High confidence |

### Max Score (Default)

```yaml
pipelines:
  safety:
    type: parallel
    classifiers:
      - toxicity         # score: 0.3
      - hate_speech      # score: 0.8
    aggregation: max_score
    # Result: 0.8 (highest)
```

### Weighted Average

```yaml
pipelines:
  safety:
    type: parallel
    classifiers:
      - name: toxicity
        weight: 2.0       # More important
      - name: sentiment
        weight: 1.0
    aggregation: weighted_average
```

### First Positive

```yaml
pipelines:
  quick_check:
    type: parallel
    classifiers:
      - pii_detector
      - prompt_injection
    aggregation: first_positive
    threshold: 0.8
    # Returns as soon as any classifier exceeds 0.8
```

### Unanimous

```yaml
pipelines:
  strict_check:
    type: parallel
    classifiers:
      - toxicity
      - hate_speech
      - violence
    aggregation: unanimous
    threshold: 0.7
    # Only triggers if ALL classifiers exceed 0.7
```

---

## Stage Configuration

### Early Exit

```yaml
stages:
  - classifier: pattern_check
    exit_on: match              # Exit if any pattern matches

  - classifier: ml_check
    exit_on: threshold          # Exit if score exceeds threshold
    threshold: 0.9

  - classifier: final_check
    exit_on: never              # Always run (default)
```

### Timeout Per Stage

```yaml
stages:
  - classifier: fast_check
    timeout_ms: 5

  - classifier: slow_check
    timeout_ms: 100
    on_timeout: skip            # skip, error, default_score
```

### Score Transformation

```yaml
stages:
  - classifier: raw_classifier
    transform:
      type: normalize           # Normalize to 0-1
      min: -5
      max: 5
```

---

## Phase-Specific Pipelines

Configure different pipelines for each phase:

```yaml
pipeline:
  ingress:
    pipeline: ingress_safety

  midstream:
    pipeline: streaming_safety

  egress:
    pipeline: compliance_check

pipelines:
  ingress_safety:
    type: sequential
    stages:
      - classifier: pii_detector
        exit_on: match
      - classifier: prompt_injection
        threshold: 0.8

  streaming_safety:
    type: parallel
    classifiers:
      - toxicity
    aggregation: max_score
    timeout_ms: 10

  compliance_check:
    type: parallel
    classifiers:
      - financial_advice
      - medical_advice
      - legal_advice
    aggregation: max_score
```

---

## Advanced Patterns

### Tiered Classification

Fast checks first, expensive checks only if needed:

```yaml
pipelines:
  tiered_safety:
    type: sequential
    stages:
      # Tier A: Fast patterns (~0.5ms)
      - name: patterns
        type: parallel
        classifiers:
          - pii_detector
          - prompt_injection_patterns
        exit_on: threshold
        threshold: 0.95

      # Tier B: ML models (~5ms)
      - name: ml_basic
        type: parallel
        classifiers:
          - toxicity
          - prompt_injection
        exit_on: threshold
        threshold: 0.9

      # Tier C: Heavy analysis (~50ms, only for edge cases)
      - name: deep_analysis
        type: single
        classifier: comprehensive_classifier
```

### Category-Specific Routing

```yaml
pipelines:
  category_router:
    type: conditional
    stages:
      - classifier: content_type
        conditions:
          - when: label == "financial"
            then:
              - classifier: financial_compliance
          - when: label == "medical"
            then:
              - classifier: medical_compliance
          - when: label == "general"
            then:
              - classifier: general_safety
```

### Ensemble Classification

Combine multiple models for higher accuracy:

```yaml
pipelines:
  ensemble_toxicity:
    type: parallel
    classifiers:
      - toxicity_model_a
      - toxicity_model_b
      - toxicity_model_c
    aggregation: average
    # More robust than single model
```

---

## Pipeline Metrics

Each pipeline reports metrics:

```
checkstream_pipeline_latency_ms{pipeline="content_safety",phase="ingress"}
checkstream_pipeline_calls_total{pipeline="content_safety",result="allow"}
checkstream_pipeline_stage_skipped_total{pipeline="tiered_safety",stage="deep_analysis"}
```

---

## Complete Example

```yaml
# config.yaml
pipeline:
  ingress:
    pipeline: ingress_pipeline

  midstream:
    pipeline: midstream_pipeline

  egress:
    pipeline: egress_pipeline

pipelines:
  # Ingress: Block bad prompts fast
  ingress_pipeline:
    type: sequential
    stages:
      - name: fast_patterns
        type: parallel
        classifiers:
          - pii_detector
          - prompt_injection_patterns
        aggregation: max_score
        exit_on: threshold
        threshold: 0.95

      - name: ml_injection
        classifier: prompt_injection
        exit_on: threshold
        threshold: 0.85

  # Midstream: Real-time content safety
  midstream_pipeline:
    type: parallel
    classifiers:
      - toxicity
      - pii_detector
    aggregation: max_score
    timeout_ms: 10

  # Egress: Comprehensive compliance
  egress_pipeline:
    type: parallel
    classifiers:
      - financial_advice
      - medical_advice
      - legal_advice
      - compliance_summary
    aggregation: max_score
```

---

## Testing Pipelines

### Test Pipeline Execution

```bash
curl http://localhost:8080/admin/test-pipeline \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": "ingress_pipeline",
    "text": "Test input text"
  }'
```

Response:

```json
{
  "pipeline": "ingress_pipeline",
  "result": {
    "score": 0.23,
    "action": "allow",
    "stages": [
      {
        "name": "fast_patterns",
        "score": 0.1,
        "latency_ms": 0.5,
        "exit": false
      },
      {
        "name": "ml_injection",
        "score": 0.23,
        "latency_ms": 4.2,
        "exit": false
      }
    ]
  },
  "total_latency_ms": 4.7
}
```

---

## Next Steps

- [Classifier Configuration](classifiers.md) - Configure individual classifiers
- [Policy Engine](../guides/policy-engine.md) - Use pipeline results in policies
- [Classifier System](../architecture/classifiers.md) - Understanding classifier tiers
