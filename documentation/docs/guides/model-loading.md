# Model Loading Guide

Load and manage ML models in CheckStream using Candle and HuggingFace Hub.

---

## Overview

CheckStream uses the Candle ML framework for inference, supporting:

- HuggingFace Hub integration
- SafeTensors and PyTorch formats
- CPU, CUDA, and Metal acceleration
- INT8/INT4 quantization
- Lazy loading with caching

---

## Loading from HuggingFace

### Basic Configuration

```yaml
classifiers:
  toxicity:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
```

### With Specific Revision

```yaml
classifiers:
  toxicity:
    model:
      source: huggingface
      repo: "unitary/toxic-bert"
      revision: "v1.0.0"          # Tag
      # revision: "abc123def"     # Commit hash
```

### Private Repositories

```yaml
classifiers:
  custom_model:
    model:
      source: huggingface
      repo: "company/private-model"
      token_env: "HF_TOKEN"       # Environment variable with token
```

---

## Loading Local Models

### SafeTensors Format (Recommended)

```yaml
classifiers:
  local_classifier:
    tier: B
    type: ml
    model:
      source: local
      path: "./models/my-classifier"
      weights: "model.safetensors"
      config: "config.json"
```

### PyTorch Format

```yaml
classifiers:
  pytorch_model:
    model:
      source: local
      path: "./models/pytorch-model"
      weights: "pytorch_model.bin"
      config: "config.json"
      format: pytorch
```

### Required Files

```
models/my-classifier/
├── config.json           # Model architecture config
├── model.safetensors     # Weights (or pytorch_model.bin)
├── tokenizer_config.json # Tokenizer config
├── vocab.txt             # Vocabulary (BERT-style)
└── special_tokens_map.json
```

---

## Device Selection

### Automatic Selection

```yaml
classifiers:
  toxicity:
    device: auto
    # Uses CUDA > Metal > CPU in order of preference
```

### Explicit Device

```yaml
classifiers:
  # CPU only
  pattern_classifier:
    device: cpu

  # NVIDIA GPU
  heavy_model:
    device: cuda
    device_id: 0          # Specific GPU

  # Apple Silicon
  mac_model:
    device: metal
```

### Memory Management

```yaml
classifiers:
  large_model:
    device: cuda
    memory:
      max_allocation_mb: 2048
      allow_growth: true
```

---

## Quantization

Reduce model size and improve inference speed.

### INT8 Quantization

```yaml
classifiers:
  toxicity:
    model:
      repo: "unitary/toxic-bert"
      quantization: int8
    # ~4x smaller, ~2x faster
```

### INT4 Quantization

```yaml
classifiers:
  toxicity:
    model:
      repo: "unitary/toxic-bert"
      quantization: int4
    # ~8x smaller, ~3x faster, some accuracy loss
```

### Quantization Comparison

| Quantization | Size | Speed | Accuracy |
|--------------|------|-------|----------|
| None (FP32) | 100% | 1x | 100% |
| FP16 | 50% | ~1.5x | ~99.9% |
| INT8 | 25% | ~2x | ~99% |
| INT4 | 12.5% | ~3x | ~95-98% |

---

## Caching

### Model Cache

Models are cached in `~/.cache/huggingface/hub/`:

```
~/.cache/huggingface/hub/
├── models--unitary--toxic-bert/
│   ├── snapshots/
│   │   └── abc123def/
│   │       ├── model.safetensors
│   │       └── config.json
│   └── refs/
│       └── main
```

### Custom Cache Location

```yaml
model_cache:
  path: "/opt/checkstream/models"
  max_size_gb: 50
```

Or via environment:

```bash
export HF_HOME=/opt/checkstream/models
```

### Inference Cache

Cache classification results:

```yaml
classifiers:
  toxicity:
    inference_cache:
      enabled: true
      max_entries: 10000
      ttl_seconds: 3600
```

---

## Tokenizer Configuration

### Auto-Load Tokenizer

```yaml
classifiers:
  toxicity:
    model:
      repo: "unitary/toxic-bert"
    # Tokenizer automatically loaded from same repo
```

### Custom Tokenizer

```yaml
classifiers:
  custom:
    model:
      source: local
      path: "./models/custom"
    tokenizer:
      source: huggingface
      repo: "bert-base-uncased"
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
      return_attention_mask: true
```

---

## Model Warmup

Pre-load models at startup:

```yaml
startup:
  warmup_models: true
  warmup_timeout_s: 60
```

Or trigger manually:

```bash
curl -X POST http://localhost:8080/admin/warmup
```

---

## Multiple Model Instances

### Model Registry

```yaml
model_registry:
  toxicity_v1:
    repo: "unitary/toxic-bert"
    revision: "v1.0"

  toxicity_v2:
    repo: "unitary/toxic-bert"
    revision: "v2.0"

classifiers:
  toxicity_stable:
    model_ref: toxicity_v1

  toxicity_canary:
    model_ref: toxicity_v2
    mode: shadow
```

### A/B Testing

```yaml
classifiers:
  toxicity:
    ab_test:
      enabled: true
      variants:
        - model_ref: toxicity_v1
          weight: 90
        - model_ref: toxicity_v2
          weight: 10
```

---

## Supported Model Architectures

### BERT-based

```yaml
# DistilBERT, BERT, RoBERTa, etc.
classifiers:
  sentiment:
    model:
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
      architecture: bert_sequence_classification
```

### DeBERTa

```yaml
classifiers:
  prompt_injection:
    model:
      repo: "protectai/deberta-v3-base-prompt-injection"
      architecture: deberta_sequence_classification
```

### Supported Architectures

| Architecture | Description |
|--------------|-------------|
| `bert_sequence_classification` | BERT for text classification |
| `bert_token_classification` | BERT for NER/token tasks |
| `deberta_sequence_classification` | DeBERTa classifier |
| `distilbert_sequence_classification` | DistilBERT classifier |

---

## Troubleshooting

### Model Not Loading

```bash
# Check model files
ls -la ~/.cache/huggingface/hub/models--unitary--toxic-bert/

# Verify config
cat ~/.cache/huggingface/hub/models--unitary--toxic-bert/snapshots/*/config.json
```

### Out of Memory

```yaml
classifiers:
  large_model:
    model:
      quantization: int8    # Reduce memory
    max_length: 256         # Shorter sequences
    batch_size: 1           # Smaller batches
```

### Slow Inference

```yaml
# Use GPU
classifiers:
  slow_model:
    device: cuda

# Or quantize
classifiers:
  slow_model:
    model:
      quantization: int8
```

### Cache Issues

```bash
# Clear model cache
rm -rf ~/.cache/huggingface/hub/models--unitary--toxic-bert/

# Force re-download
curl -X POST http://localhost:8080/admin/reload-models
```

---

## Best Practices

1. **Use SafeTensors** - Faster and safer than PyTorch format
2. **Enable quantization** - INT8 for production, FP32 for accuracy testing
3. **Warmup at startup** - Avoid cold start latency
4. **Use caching** - Both model and inference caching
5. **Monitor memory** - Track `checkstream_model_memory_bytes` metric
6. **Test locally first** - Verify models before deployment

---

## Next Steps

- [Classifier Configuration](../configuration/classifiers.md) - Full classifier options
- [Pipeline Configuration](../configuration/pipelines.md) - Use models in pipelines
- [Classifier System](../architecture/classifiers.md) - Understanding tiers
