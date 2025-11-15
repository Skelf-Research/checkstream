# Model Loading in CheckStream

**Summary of how to add and use ML models**

---

## TL;DR

**You don't need to write code for most models.**

```yaml
# Add to models/registry.yaml
models:
  my-new-model:
    source:
      type: huggingface
      repo: "org/model-name"
    architecture:
      type: bert-sequence-classification
      num_labels: 2
```

Done. CheckStream will auto-download and load it.

---

## Current Implementation Status

### ✅ What's Working Now

1. **Manual Model Loading** (`crates/checkstream-classifiers/src/toxicity.rs`)
   - BERT-based toxicity classifier
   - Loads from local path: `./models/toxic-bert`
   - ~100ms inference on CPU
   - Graceful fallback to pattern-based detection

2. **Model Download Scripts** (`scripts/download_models.sh`)
   - Downloads models from HuggingFace
   - Builds tokenizer.json from vocab
   - Verifies downloaded files

3. **Feature Flags** (`Cargo.toml`)
   - `cargo build` - Pattern-based only (no ML deps)
   - `cargo build --features ml-models` - Full ML support

4. **Model Registry Configuration** (`models/registry.yaml`)
   - YAML-based model definitions
   - Supports local and HuggingFace sources
   - Architecture specifications (BERT, DistilBERT, etc.)
   - Inference settings (device, threshold, etc.)

5. **Configuration Parsing** (`crates/checkstream-classifiers/src/model_config.rs`)
   - Type-safe YAML parsing
   - Validation of model configs
   - Support for multiple architectures

### 🚧 In Progress / Planned

1. **Generic Model Loader** (Next Step)
   - Automatic model loading from registry
   - Support for BERT, RoBERTa, DistilBERT variants
   - HuggingFace auto-download
   - No code needed for standard architectures

2. **Dynamic Classifier Registration**
   - Load classifiers at runtime based on config
   - Hot reload when config changes
   - Lazy loading (load only when needed)

3. **Advanced Features**
   - Quantization support (int8, float16)
   - A/B testing model variants
   - Model ensembles
   - GPU/MPS support

---

## How to Add a New Model Today

### Option 1: Local Model (Manual)

```bash
# 1. Create a new classifier file (copy toxicity.rs as template)
cp crates/checkstream-classifiers/src/toxicity.rs \
   crates/checkstream-classifiers/src/my_classifier.rs

# 2. Modify the new file:
#    - Change struct name
#    - Update model paths
#    - Adjust labels/num_labels

# 3. Add to lib.rs
echo "pub mod my_classifier;" >> crates/checkstream-classifiers/src/lib.rs

# 4. Register in classifiers.yaml
cat >> config/classifiers.yaml <<EOF
  my-classifier:
    type: custom
    tier: B
EOF

# 5. Build and test
cargo build --features ml-models
```

**Time**: ~30 minutes of coding

### Option 2: Using Model Registry (Future)

```bash
# 1. Add to registry
cat >> models/registry.yaml <<EOF
  my-model:
    source:
      type: huggingface
      repo: "org/my-model"
    architecture:
      type: bert-sequence-classification
      num_labels: 2
EOF

# 2. Reference in classifier config
cat >> config/classifiers.yaml <<EOF
  my-classifier:
    type: ml
    model: "my-model"
    tier: B
EOF

# 3. Done!
cargo run  # Auto-downloads and loads
```

**Time**: ~2 minutes of config editing

---

## Supported Model Architectures

### Currently Implemented

- ✅ **BERT** (`bert-sequence-classification`)
  - Example: `unitary/toxic-bert`
  - Supports: bert-base, bert-large variants

### Planned (Generic Loaders)

- 🚧 **RoBERTa** (`roberta-sequence-classification`)
- 🚧 **DistilBERT** (`distilbert-sequence-classification`)
- 🚧 **DeBERTa** (`deberta-sequence-classification`)
- 🚧 **Sentence Transformers** (`sentence-transformer`)
- 🚧 **Custom Heads** (`bert-custom-head`)

### When You Need Code

You only need to write Rust code if:

1. **Novel architecture** not in the list above
   - Example: Custom CNN+LSTM hybrid
   - Example: Specialized attention mechanism

2. **Custom preprocessing** beyond config options
   - Example: Domain-specific tokenization
   - Example: Multi-modal inputs (text + images)

3. **Complex post-processing**
   - Example: Rule-based overrides
   - Example: Multi-model ensembles with custom logic

**Estimate**: ~90% of models won't need code.

---

## Model Loading Flow

### Current (Manual)

```
User Request
     ↓
ToxicityClassifier::new()
     ↓
Load tokenizer from ./models/toxic-bert/tokenizer.json
     ↓
Load BERT weights from ./models/toxic-bert/model.safetensors
     ↓
Ready for inference
```

### Future (Dynamic)

```
User Request
     ↓
ClassifierRegistry::get("toxicity")
     ↓
Read models/registry.yaml
     ↓
Check if model exists locally
     ↓
If not: Download from HuggingFace
     ↓
GenericModelLoader::load_bert_classifier()
     ↓
Ready for inference
```

---

## Configuration Examples

### Example 1: Local BERT Model

```yaml
models:
  toxicity:
    source:
      type: local
      path: "./models/toxic-bert"
    architecture:
      type: bert-sequence-classification
      num_labels: 6
      labels: [toxic, severe_toxic, obscene, threat, insult, identity_hate]
    inference:
      device: cpu
      max_length: 512
      threshold: 0.5
```

### Example 2: HuggingFace Model with Quantization

```yaml
models:
  prompt-injection:
    source:
      type: huggingface
      repo: "protectai/deberta-v3-base-prompt-injection"
    architecture:
      type: deberta-sequence-classification
      num_labels: 2
    inference:
      device: cpu
      quantization:
        enabled: true
        method: dynamic
        dtype: int8
```

### Example 3: A/B Testing Variants

```yaml
models:
  toxicity:
    variants:
      - name: default
        source: {type: local, path: "./models/toxic-bert"}
        weight: 0.8

      - name: experimental
        source: {type: huggingface, repo: "org/toxic-bert-v2"}
        weight: 0.2
```

---

## File Locations

```
checkstream/
├── models/
│   ├── registry.yaml              # Model definitions (EDIT THIS)
│   └── toxic-bert/                # Downloaded model files
│       ├── config.json
│       ├── tokenizer.json
│       ├── vocab.txt
│       └── model.safetensors
│
├── config/
│   └── classifiers.yaml           # Classifier config (EDIT THIS)
│
├── scripts/
│   ├── download_models.sh         # Download from HF
│   └── build_tokenizer.py         # Build tokenizer.json
│
└── crates/checkstream-classifiers/src/
    ├── model_config.rs            # Config parsing
    ├── toxicity.rs                # Manual BERT loader (current)
    └── model_loader.rs            # Generic loader (planned)
```

---

## Next Steps (Roadmap)

### Phase 1: Generic BERT Loader (Week 1-2)
- [ ] Implement `GenericModelLoader` for BERT
- [ ] Auto-download from HuggingFace
- [ ] Integrate with `ClassifierRegistry`
- [ ] Test with multiple BERT variants

### Phase 2: More Architectures (Week 3-4)
- [ ] Add RoBERTa support
- [ ] Add DistilBERT support
- [ ] Add DeBERTa support
- [ ] Add sentence transformer support

### Phase 3: Advanced Features (Week 5-6)
- [ ] Quantization (int8, float16)
- [ ] GPU/MPS device support
- [ ] Model caching and lazy loading
- [ ] Hot reload

### Phase 4: Production (Week 7-8)
- [ ] A/B testing variants
- [ ] Model ensembles
- [ ] Performance benchmarking
- [ ] Documentation

---

## Key Insight

**The goal**: Make it so easy to add models that users rarely need to write code.

**Current**: ~30 min of Rust coding per model
**Target**: ~2 min of YAML editing per model

**Benefit**:
- Faster iteration on model selection
- Non-developers can swap models
- A/B testing becomes trivial
- Multi-model experimentation is frictionless

---

## Questions?

See:
- [Dynamic Model Loading Guide](DYNAMIC_MODEL_LOADING.md) - Full specification
- [Model Config Reference](../models/registry.yaml) - Example configurations
- [Classifier Integration](../crates/checkstream-classifiers/src/toxicity.rs) - Current implementation

---

**Last Updated**: 2025-11-14
