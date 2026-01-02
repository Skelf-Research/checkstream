# Classifier Configuration

Configure ML models and pattern classifiers for CheckStream.

---

## Configuration File

Classifiers are configured in `classifiers.yaml` or inline in `config.yaml`.

```yaml
# classifiers.yaml
classifiers:
  toxicity:
    # ... configuration
  prompt_injection:
    # ... configuration
```

---

## Pattern Classifier Configuration

### Basic Pattern Classifier

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
```

### Pattern Options

| Option | Type | Description |
|--------|------|-------------|
| `name` | string | Pattern identifier |
| `pattern` | string | Regex pattern |
| `score` | float | Score when matched (0.0-1.0) |
| `case_insensitive` | bool | Ignore case (default: false) |
| `multiline` | bool | Match across lines |

### Complex Pattern Example

```yaml
classifiers:
  prompt_injection_patterns:
    tier: A
    type: pattern
    patterns:
      - name: ignore_instructions
        pattern: 'ignore\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?)'
        case_insensitive: true
        score: 0.95

      - name: system_prompt_leak
        pattern: '(reveal|show|display|print)\s+(your\s+)?(system\s+)?prompt'
        case_insensitive: true
        score: 0.9

      - name: role_override
        pattern: 'you\s+are\s+(now|actually)\s+a'
        case_insensitive: true
        score: 0.85
```

---

## ML Classifier Configuration

### HuggingFace Model

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
      revision: "main"           # Optional: specific commit/tag
    device: auto                  # auto, cpu, cuda, metal
    max_length: 512
```

### Local Model

```yaml
classifiers:
  custom_classifier:
    tier: B
    type: ml
    model:
      source: local
      path: "./models/my-classifier"
      config: "config.json"
      weights: "model.safetensors"
    tokenizer:
      path: "./models/my-classifier"
      config: "tokenizer_config.json"
```

### Model Options

| Option | Type | Description |
|--------|------|-------------|
| `source` | string | `huggingface` or `local` |
| `repo` | string | HuggingFace repo ID |
| `path` | string | Local model directory |
| `revision` | string | Git revision (tag/commit) |
| `quantization` | string | `none`, `int8`, `int4` |

---

## Device Configuration

### Automatic Device Selection

```yaml
classifiers:
  toxicity:
    device: auto   # Uses GPU if available, else CPU
```

### Specific Device

```yaml
classifiers:
  toxicity:
    device: cuda        # NVIDIA GPU
    device_id: 0        # Specific GPU

  sentiment:
    device: metal       # Apple Silicon

  pii:
    device: cpu         # Force CPU
```

---

## Quantization

Reduce model size and improve inference speed:

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      repo: "unitary/toxic-bert"
      quantization: int8    # 4x smaller, ~2x faster
```

| Quantization | Size | Speed | Accuracy |
|--------------|------|-------|----------|
| `none` | 100% | 1x | Best |
| `int8` | ~25% | ~2x | Good |
| `int4` | ~12.5% | ~3x | Acceptable |

---

## Label Mapping

Map model outputs to meaningful labels:

```yaml
classifiers:
  sentiment:
    tier: B
    type: ml
    model:
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
    labels:
      0: negative
      1: positive
    threshold_label: negative   # Which label to threshold on
```

### Multi-Label Classification

```yaml
classifiers:
  content_type:
    tier: B
    type: ml
    model:
      repo: "company/multi-label-classifier"
    labels:
      0: safe
      1: violence
      2: adult
      3: hate
    multi_label: true           # Multiple labels can be active
    threshold_per_label:
      violence: 0.8
      adult: 0.9
      hate: 0.85
```

---

## Tokenizer Configuration

### Default Tokenizer

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      repo: "unitary/toxic-bert"
    # Tokenizer auto-loaded from same repo
```

### Custom Tokenizer

```yaml
classifiers:
  custom:
    tier: B
    type: ml
    model:
      source: local
      path: "./models/custom"
    tokenizer:
      source: huggingface
      repo: "bert-base-uncased"   # Use different tokenizer
```

### Tokenizer Options

```yaml
classifiers:
  toxicity:
    tokenizer:
      max_length: 512
      truncation: true
      padding: max_length
      add_special_tokens: true
```

---

## Caching Configuration

### Model Caching

```yaml
classifiers:
  toxicity:
    cache:
      enabled: true
      path: "~/.cache/checkstream/models"
      ttl_hours: 168            # 1 week
```

### Inference Caching

Cache classification results for repeated inputs:

```yaml
classifiers:
  toxicity:
    inference_cache:
      enabled: true
      max_entries: 10000
      ttl_seconds: 3600         # 1 hour
```

---

## Batching Configuration

```yaml
classifiers:
  toxicity:
    batching:
      enabled: true
      max_batch_size: 8
      max_wait_ms: 5            # Max time to wait for batch
```

---

## Complete Example

```yaml
# classifiers.yaml
version: "1.0"

defaults:
  device: auto
  max_length: 512
  cache:
    enabled: true

classifiers:
  # Tier A - Pattern Matching
  pii_detector:
    tier: A
    type: pattern
    patterns:
      - name: ssn
        pattern: '\b\d{3}-\d{2}-\d{4}\b'
        score: 1.0
      - name: credit_card
        pattern: '\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b'
        score: 1.0
      - name: email
        pattern: '\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b'
        score: 0.9

  prompt_injection_patterns:
    tier: A
    type: pattern
    patterns:
      - name: ignore_instructions
        pattern: 'ignore\s+(all\s+)?(previous|prior)\s+instructions?'
        case_insensitive: true
        score: 0.95

  # Tier B - Quantized ML
  toxicity:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
      quantization: int8
    device: auto
    max_length: 512
    labels:
      0: non-toxic
      1: toxic
    threshold_label: toxic

  sentiment:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
      quantization: int8
    labels:
      0: negative
      1: positive

  prompt_injection:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "protectai/deberta-v3-base-prompt-injection"
      quantization: int8

  # Tier C - Full Models (for egress)
  financial_advice:
    tier: C
    type: ml
    model:
      source: local
      path: "./models/financial-classifier"
    device: cuda
    max_length: 1024
```

---

## Verifying Classifiers

### List Loaded Classifiers

```bash
curl http://localhost:8080/admin/classifiers
```

```json
{
  "classifiers": [
    {"name": "toxicity", "tier": "B", "status": "loaded"},
    {"name": "pii_detector", "tier": "A", "status": "loaded"},
    {"name": "prompt_injection", "tier": "B", "status": "loaded"}
  ]
}
```

### Test a Classifier

```bash
curl http://localhost:8080/admin/test-classifier \
  -H "Content-Type: application/json" \
  -d '{
    "classifier": "toxicity",
    "text": "This is a test message"
  }'
```

```json
{
  "classifier": "toxicity",
  "score": 0.12,
  "label": "non-toxic",
  "latency_ms": 2.3
}
```

---

## Next Steps

- [Pipeline Configuration](pipelines.md) - Combine classifiers into pipelines
- [Model Loading Guide](../guides/model-loading.md) - Advanced model loading
- [Classifier System](../architecture/classifiers.md) - Understanding tiers
