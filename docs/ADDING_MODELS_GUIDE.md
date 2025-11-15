# Adding New Models - Step by Step Guide

**How to add a new ML model to CheckStream in 2 minutes** (no code required)

---

## Quick Reference

1. Edit `models/registry.yaml` - Add model config
2. Run CheckStream - Model auto-loads
3. Done!

---

## Example: Adding a Sentiment Analysis Model

### Step 1: Find a Model

Browse HuggingFace for models: https://huggingface.co/models

Example: `distilbert-base-uncased-finetuned-sst-2-english` (sentiment classifier)

### Step 2: Add to Registry

Edit `models/registry.yaml`:

```yaml
models:
  # ... existing models ...

  # NEW: Add sentiment model
  sentiment:
    name: "distilbert-sentiment"
    version: "1.0"
    description: "DistilBERT sentiment analysis"

    source:
      type: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
      revision: "main"

    architecture:
      type: distilbert-sequence-classification
      num_labels: 2
      labels:
        - negative
        - positive

    inference:
      device: "cpu"
      max_length: 256
      threshold: 0.5

    preprocessing:
      - type: lowercase
      - type: normalize-whitespace
```

### Step 3: Use It

```rust
// That's it! No code changes needed
let registry = DynamicClassifierRegistry::from_file("models/registry.yaml").await?;
let classifier = registry.get_classifier("sentiment").await?;
let result = classifier.classify("This product is amazing!").await?;
println!("Sentiment: {}", result.label);  // "positive"
```

### Step 4: Integrate with Pipelines

Edit `config/classifiers.yaml`:

```yaml
classifiers:
  sentiment:
    type: ml
    model: "sentiment"  # References models/registry.yaml
    tier: B

# Use in pipelines
pipelines:
  content-analysis:
    stages:
      - type: parallel
        classifiers:
          - toxicity
          - sentiment  # NEW
          - pii
```

**Done!** Your pipeline now includes sentiment analysis.

---

## Real-World Example: Adding Prompt Injection Detector

### Use Case

Detect malicious prompt injection attempts.

### Step 1: Find Model

Use: `protectai/deberta-v3-base-prompt-injection`

### Step 2: Add to Registry

```yaml
models:
  prompt-injection:
    name: "deberta-prompt-injection"
    version: "1.0"
    description: "DeBERTa prompt injection detector"

    source:
      type: huggingface
      repo: "protectai/deberta-v3-base-prompt-injection"

    architecture:
      type: deberta-sequence-classification
      num_labels: 2
      labels:
        - safe
        - injection

    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.8  # Conservative (fewer false positives)
```

### Step 3: Use in Ingress Pipeline

```yaml
# config/pipelines.yaml
pipelines:
  ingress_pipeline: "security-checks"

# config/classifiers.yaml
pipelines:
  security-checks:
    stages:
      - type: parallel
        classifiers:
          - prompt-injection  # NEW
          - pii
        aggregation: max_score
```

**Result**: All incoming prompts now checked for injection attempts.

---

## Supported Architectures (No Code Needed)

### BERT Family

```yaml
architecture:
  type: bert-sequence-classification
  num_labels: N
```

**Works with**:
- `bert-base-uncased`
- `bert-large-uncased`
- `roberta-base`
- `albert-base-v2`

### DistilBERT (Faster)

```yaml
architecture:
  type: distilbert-sequence-classification
  num_labels: N
```

**Works with**:
- `distilbert-base-uncased`
- `distilbert-base-multilingual-cased`

### DeBERTa (More Accurate)

```yaml
architecture:
  type: deberta-sequence-classification
  num_labels: N
```

**Works with**:
- `microsoft/deberta-base`
- `microsoft/deberta-v3-base`

---

## Model Configuration Options

### Source Types

#### 1. HuggingFace Hub

```yaml
source:
  type: huggingface
  repo: "organization/model-name"
  revision: "main"  # or specific commit/tag
```

**Auto-downloads** on first use, caches locally.

#### 2. Local Path

```yaml
source:
  type: local
  path: "./models/my-custom-model"
```

**Use for**:
- Custom fine-tuned models
- Offline deployments
- Models not on HuggingFace

#### 3. Built-in

```yaml
source:
  type: builtin
  implementation: "PatternPIIDetector"
```

**Use for**:
- Pattern-based classifiers (regex, etc.)
- No ML required

### Inference Settings

```yaml
inference:
  device: "cpu"        # cpu, cuda, mps
  max_length: 512      # Max tokens
  batch_size: 1        # Batch size
  threshold: 0.5       # Classification threshold

  # Optional: Quantization for speed
  quantization:
    enabled: true
    method: "dynamic"  # dynamic, static
    dtype: "int8"      # int8, int4, float16
```

### Preprocessing Steps

```yaml
preprocessing:
  - type: lowercase
  - type: remove-urls
  - type: truncate
    max_length: 512
  - type: normalize-whitespace
  - type: remove-emojis
```

### Output Configuration

```yaml
output:
  output_type: "multi-label"  # single-label, multi-label, regression
  aggregation: "max"           # max, mean, any (for multi-label)
```

---

## Complete Example: Multi-Model Setup

```yaml
# models/registry.yaml

version: "1.0"

models:
  # Toxicity detection
  toxicity:
    source: {type: local, path: "./models/toxic-bert"}
    architecture: {type: bert-sequence-classification, num_labels: 6}

  # Sentiment analysis
  sentiment:
    source:
      type: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
    architecture: {type: distilbert-sequence-classification, num_labels: 2}
    inference:
      quantization: {enabled: true, dtype: "int8"}  # Faster

  # Prompt injection
  prompt-injection:
    source:
      type: huggingface
      repo: "protectai/deberta-v3-base-prompt-injection"
    architecture: {type: deberta-sequence-classification, num_labels: 2}
    inference:
      threshold: 0.8  # Conservative

  # PII detection (pattern-based, no ML)
  pii:
    source: {type: builtin, implementation: "PiiClassifier"}
```

### Usage

```rust
let registry = DynamicClassifierRegistry::from_file("models/registry.yaml").await?;

// All these load dynamically from config
let toxicity = registry.get_classifier("toxicity").await?;
let sentiment = registry.get_classifier("sentiment").await?;
let injection = registry.get_classifier("prompt-injection").await?;
let pii = registry.get_classifier("pii").await?;

// Use in pipelines
let text = "User input here";
let toxic_result = toxicity.classify(text).await?;
let sentiment_result = sentiment.classify(text).await?;
```

---

## Troubleshooting

### Model Not Loading

**Error**: `Model 'xyz' not found in registry`

**Fix**: Check `models/registry.yaml` has entry for `xyz`.

### Download Fails

**Error**: `Failed to download from HuggingFace`

**Fix**:
1. Check internet connection
2. Verify repo name is correct
3. For private models, set `HF_TOKEN` environment variable

### Unsupported Architecture

**Error**: `Unsupported architecture: XYZ`

**Fix**:
- Check if architecture is in supported list
- If not, you'll need to write custom code (see CUSTOM_MODELS.md)

### Slow Inference

**Solution**: Enable quantization

```yaml
inference:
  quantization:
    enabled: true
    method: "dynamic"
    dtype: "int8"
```

**Speedup**: 2-4x faster, minimal accuracy loss

---

## Performance Tips

### 1. Use Quantization for Production

```yaml
inference:
  quantization:
    enabled: true
    dtype: "int8"  # 2-4x faster
```

### 2. Use DistilBERT Instead of BERT

- **DistilBERT**: ~40% faster, 97% accuracy of BERT
- **BERT**: More accurate, but slower

### 3. Adjust Max Length

```yaml
inference:
  max_length: 256  # Instead of 512 for short texts
```

**Result**: 2x faster inference

### 4. Preload Commonly Used Models

```rust
let registry = DynamicRegistryBuilder::new()
    .preload("toxicity")
    .preload("sentiment")
    .build().await?;
```

**Result**: No delay on first use

---

## Next Steps

- **Add your first model**: Follow Quick Reference above
- **A/B test models**: See [A/B Testing Guide](AB_TESTING.md)
- **Custom architectures**: See [Custom Models Guide](CUSTOM_MODELS.md)
- **Production deployment**: See [Deployment Guide](../deployment-modes.md)

---

**Time to add a model**: ~2 minutes
**Code changes needed**: 0
**Just edit**: `models/registry.yaml`

