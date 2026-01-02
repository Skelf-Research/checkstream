# Classifier System

CheckStream uses a tiered classifier system optimized for real-time streaming with minimal latency.

---

## Tier Overview

| Tier | Latency | Method | Use Case |
|------|---------|--------|----------|
| **A** | <2ms | Pattern matching | PII patterns, keywords, regex |
| **B** | <5ms | Quantized ML | Toxicity, sentiment, injection |
| **C** | <10ms | Full models | Complex classification |

---

## Tier A: Pattern Classifiers

Pattern classifiers use compiled regular expressions and DFA (Deterministic Finite Automata) for ultra-fast matching.

### Characteristics

- **Latency**: ~0.5ms average
- **Memory**: Minimal (compiled patterns)
- **Accuracy**: Exact matching (no false positives for patterns)
- **Best for**: Known patterns, formats, keywords

### Built-in Patterns

| Classifier | Detects |
|------------|---------|
| `pii_ssn` | Social Security Numbers |
| `pii_email` | Email addresses |
| `pii_phone` | Phone numbers |
| `pii_credit_card` | Credit card numbers |
| `prompt_injection_patterns` | Known injection phrases |

### Configuration

```yaml
classifiers:
  pii_detector:
    tier: A
    type: pattern
    patterns:
      - name: ssn
        pattern: '\b\d{3}-\d{2}-\d{4}\b'
        score: 1.0
      - name: email
        pattern: '\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b'
        score: 0.9
      - name: phone
        pattern: '\b\d{3}[-.]?\d{3}[-.]?\d{4}\b'
        score: 0.8
```

### Custom Patterns

Add domain-specific patterns:

```yaml
classifiers:
  custom_blocklist:
    tier: A
    type: pattern
    patterns:
      - name: competitor_mention
        pattern: '\b(CompetitorA|CompetitorB)\b'
        case_insensitive: true
        score: 0.7
      - name: internal_project
        pattern: '\b(ProjectX|CodeName\w+)\b'
        score: 0.9
```

---

## Tier B: Quantized ML Classifiers

Tier B classifiers use quantized transformer models for high-accuracy classification with controlled latency.

### Characteristics

- **Latency**: 2-5ms (GPU), 30-50ms (CPU)
- **Memory**: 50-200MB per model
- **Accuracy**: High (fine-tuned models)
- **Best for**: Semantic understanding, nuanced content

### Built-in Models

| Classifier | Model | Purpose |
|------------|-------|---------|
| `toxicity` | toxic-bert (quantized) | Offensive content |
| `sentiment` | distilbert-sentiment | Positive/negative tone |
| `prompt_injection` | injection-detector | Jailbreak attempts |

### Configuration

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
      quantization: int8
    device: auto  # cpu, cuda, metal
    max_length: 512
    batch_size: 1
```

### Model Loading

Models are loaded from HuggingFace Hub with automatic caching:

```
First request: Download → Cache → Load → Inference (~5-10s)
Subsequent:    Cache hit → Load → Inference (~100ms cold, ~5ms warm)
```

Cache location: `~/.cache/huggingface/hub/`

### Output Format

ML classifiers return scores and labels:

```json
{
  "classifier": "toxicity",
  "score": 0.87,
  "label": "toxic",
  "confidence": 0.87,
  "latency_ms": 3.2
}
```

---

## Tier C: Full Model Classifiers

Tier C classifiers use full-size models for maximum accuracy when latency is less critical.

### Characteristics

- **Latency**: 5-10ms (GPU), 100-200ms (CPU)
- **Memory**: 500MB-2GB per model
- **Accuracy**: Highest
- **Best for**: Egress phase, complex analysis

### Use Cases

- Full compliance analysis
- Nuanced content categorization
- Multi-label classification
- Domain-specific detection

### Configuration

```yaml
classifiers:
  financial_compliance:
    tier: C
    type: ml
    model:
      source: huggingface
      repo: "company/finance-classifier"
    device: cuda
    max_length: 1024
```

---

## Classifier Pipeline

Classifiers can be combined into pipelines for sophisticated analysis.

### Pipeline Types

| Type | Description |
|------|-------------|
| `single` | Run one classifier |
| `parallel` | Run multiple classifiers concurrently |
| `sequential` | Chain classifiers in order |
| `conditional` | Run based on previous results |

### Parallel Pipeline

Run multiple classifiers and aggregate results:

```yaml
pipelines:
  content_safety:
    type: parallel
    classifiers:
      - toxicity
      - prompt_injection
      - pii_detector
    aggregation: max_score
```

### Sequential Pipeline

Chain classifiers with early exit:

```yaml
pipelines:
  tiered_check:
    type: sequential
    stages:
      - classifier: pii_detector      # Fast pattern check first
        exit_on: match
      - classifier: prompt_injection  # Then ML check
        exit_on: threshold
        threshold: 0.9
      - classifier: toxicity          # Finally toxicity
```

### Conditional Pipeline

Run classifiers based on conditions:

```yaml
pipelines:
  smart_check:
    type: conditional
    stages:
      - classifier: quick_filter
        on_positive:
          - classifier: detailed_analysis
        on_negative:
          - skip
```

### Aggregation Strategies

| Strategy | Description |
|----------|-------------|
| `max_score` | Highest score from any classifier |
| `min_score` | Lowest score |
| `average` | Mean of all scores |
| `weighted_average` | Weighted mean |
| `first_positive` | First score above threshold |
| `unanimous` | All must agree |

---

## Custom Classifiers

### Adding a Custom Model

1. **Prepare the model**: Export to SafeTensors or PyTorch format

2. **Configure in YAML**:

```yaml
classifiers:
  my_classifier:
    tier: B
    type: ml
    model:
      source: local
      path: "./models/my-classifier"
      config: "config.json"
      weights: "model.safetensors"
    tokenizer:
      path: "./models/my-classifier"
    labels:
      - safe
      - unsafe
```

3. **Use in policies**:

```yaml
policies:
  - name: custom_check
    trigger:
      classifier: my_classifier
      threshold: 0.8
    action: stop
```

### Classifier Interface

Custom classifiers must implement:

```rust
pub trait Classifier: Send + Sync {
    fn name(&self) -> &str;
    fn tier(&self) -> ClassifierTier;
    fn classify(&self, text: &str) -> ClassifierResult;
}
```

---

## Performance Tuning

### Latency Optimization

1. **Use appropriate tiers**: Tier A for ingress, B for midstream
2. **Enable GPU**: Set `device: cuda` for ML classifiers
3. **Batch processing**: Group multiple texts when possible
4. **Quantization**: Use INT8 models for 2-4x speedup

### Memory Optimization

1. **Lazy loading**: Models loaded on first use
2. **Model sharing**: Single instance across requests
3. **Quantization**: 4x smaller model size with INT8

### Configuration Example

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      repo: "unitary/toxic-bert"
      quantization: int8      # Smaller, faster
    device: cuda              # GPU acceleration
    max_length: 256           # Truncate for speed
    cache_embeddings: true    # Cache tokenization
```

---

## Monitoring Classifiers

### Metrics

```
checkstream_classifier_latency_ms{classifier="toxicity",tier="B"}
checkstream_classifier_calls_total{classifier="toxicity",result="positive"}
checkstream_classifier_errors_total{classifier="toxicity",error="timeout"}
```

### Health Check

```bash
curl http://localhost:8080/health/ready
```

Returns classifier status:

```json
{
  "status": "ready",
  "classifiers": {
    "toxicity": "loaded",
    "prompt_injection": "loaded",
    "pii_detector": "loaded"
  }
}
```

---

## Next Steps

- [Pipeline Configuration](../configuration/pipelines.md) - Configure classifier pipelines
- [Model Loading Guide](../guides/model-loading.md) - Loading custom models
- [Policy Engine](../guides/policy-engine.md) - Using classifiers in policies
