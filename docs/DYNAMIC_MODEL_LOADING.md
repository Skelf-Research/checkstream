# Dynamic Model Loading

**Load ML models without writing code**

---

## Philosophy

CheckStream supports **configuration-driven model loading**. You should be able to:

1. Add a new toxicity model by editing a YAML file
2. Swap between different BERT variants without code changes
3. Define custom preprocessing pipelines via config
4. Register models from HuggingFace or local paths

**Write code only for truly custom model architectures.**

---

## Model Configuration

### Basic Example

```yaml
# models.yaml

models:
  toxicity:
    # Model metadata
    name: "toxic-bert"
    version: "1.0"
    description: "BERT-based toxicity classifier"

    # Model source
    source:
      type: huggingface
      repo: "unitary/toxic-bert"
      revision: "main"

    # Or local path
    # source:
    #   type: local
    #   path: "./models/toxic-bert"

    # Model architecture (pre-defined loaders)
    architecture:
      type: "bert-sequence-classification"
      num_labels: 6
      labels:
        - toxic
        - severe_toxic
        - obscene
        - threat
        - insult
        - identity_hate

    # Inference settings
    inference:
      device: "cpu"           # cpu, cuda, mps
      max_length: 512
      batch_size: 1
      threshold: 0.5          # Classification threshold

    # Output mapping (how to interpret model outputs)
    output:
      type: "multi-label"     # multi-label, single-label, regression
      aggregation: "max"      # For multi-label: max, mean, any
```

### Swapping Models (No Code Changes)

```yaml
# Want to try a different toxicity model? Just update config:

models:
  toxicity:
    source:
      type: huggingface
      repo: "martin-ha/toxic-comment-model"  # Different model

    architecture:
      type: "distilbert-sequence-classification"  # Different architecture
      num_labels: 1
```

---

## Supported Architectures (Out of the Box)

CheckStream includes loaders for common architectures:

### 1. BERT Family

```yaml
architecture:
  type: "bert-sequence-classification"
  # Supports: bert-base, bert-large, roberta, albert, electra
```

**Works with**:
- `bert-base-uncased`
- `roberta-base`
- `albert-base-v2`
- `distilbert-base-uncased`

### 2. Sentence Transformers

```yaml
architecture:
  type: "sentence-transformer"
  pooling: "mean"  # mean, cls, max
```

**Use case**: Embedding-based similarity

### 3. Custom Classification Head

```yaml
architecture:
  type: "bert-custom-head"
  base_model: "bert-base-uncased"
  head:
    - type: "linear"
      in_features: 768
      out_features: 256
    - type: "relu"
    - type: "dropout"
      p: 0.1
    - type: "linear"
      in_features: 256
      out_features: 2
```

### 4. Lightweight Models

```yaml
architecture:
  type: "tiny-bert"
  # Or
  type: "mobile-bert"
```

---

## Model Registry Structure

### Directory Layout

```
models/
├── registry.yaml           # Model definitions
├── toxic-bert/
│   ├── model.safetensors
│   ├── config.json
│   ├── tokenizer.json
│   └── vocab.txt
├── prompt-injection/
│   └── ...
└── pii-detection/
    └── ...
```

### Registry File

```yaml
# models/registry.yaml

version: "1.0"

models:
  # Toxicity detection
  toxicity:
    source:
      type: local
      path: "./models/toxic-bert"
    architecture:
      type: bert-sequence-classification
      num_labels: 6
    inference:
      threshold: 0.5

  # Prompt injection detection
  prompt-injection:
    source:
      type: huggingface
      repo: "protectai/deberta-v3-base-prompt-injection"
    architecture:
      type: deberta-sequence-classification
      num_labels: 2
    inference:
      threshold: 0.8  # More conservative

  # PII detection (pattern-based, no ML)
  pii:
    source:
      type: builtin
      implementation: "PatternPIIDetector"
    # No architecture - uses regex patterns
```

---

## Classifier Configuration

### Link Models to Classifiers

```yaml
# classifiers.yaml

classifiers:
  toxicity:
    type: ml              # ml, pattern, api, custom
    model: "toxicity"     # References models/registry.yaml
    tier: B               # Tier B (<5ms target)

  prompt-injection:
    type: ml
    model: "prompt-injection"
    tier: B

  pii:
    type: pattern         # No ML model needed
    tier: A
    patterns:
      - type: ssn
        regex: "\\b\\d{3}-\\d{2}-\\d{4}\\b"
      - type: email
        regex: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}"
```

---

## Dynamic Model Loading (Runtime)

### 1. Download on Startup

```yaml
# config.yaml

models:
  auto_download: true      # Download missing models on startup
  cache_dir: "~/.cache/checkstream/models"

  sources:
    - type: huggingface
      token: "${HF_TOKEN}"  # Optional for private models
```

### 2. Lazy Loading

```yaml
models:
  lazy_load: true          # Load models only when first used
  preload:                 # Or specify which to preload
    - toxicity
    - prompt-injection
```

### 3. Hot Reload

```yaml
models:
  watch: true              # Watch for model updates
  reload_interval: 300     # Check every 5 minutes
```

---

## Custom Preprocessing

Define preprocessing pipelines in config:

```yaml
models:
  toxicity:
    preprocessing:
      - type: "lowercase"
      - type: "remove_urls"
      - type: "truncate"
        max_length: 512
      - type: "normalize_whitespace"
```

---

## Model Variants (A/B Testing)

```yaml
models:
  toxicity:
    variants:
      - name: "default"
        source:
          type: huggingface
          repo: "unitary/toxic-bert"
        weight: 0.9      # 90% of traffic

      - name: "experimental"
        source:
          type: local
          path: "./models/toxic-bert-v2"
        weight: 0.1      # 10% of traffic (A/B test)
```

---

## Quantization (Performance)

```yaml
models:
  toxicity:
    source:
      type: huggingface
      repo: "unitary/toxic-bert"

    quantization:
      enabled: true
      method: "dynamic"      # dynamic, static, qat
      dtype: "int8"          # int8, int4, float16

    # Expected speedup: 2-4x, minimal accuracy loss
```

---

## Implementation: Generic Model Loader

### Rust Code (One-time implementation)

```rust
// crates/checkstream-classifiers/src/model_loader.rs

pub struct GenericModelLoader {
    config: ModelConfig,
    cache_dir: PathBuf,
}

impl GenericModelLoader {
    pub async fn load(&self) -> Result<Box<dyn Classifier>> {
        match &self.config.architecture.type_ {
            "bert-sequence-classification" => {
                self.load_bert_classifier().await
            }
            "distilbert-sequence-classification" => {
                self.load_distilbert_classifier().await
            }
            "sentence-transformer" => {
                self.load_sentence_transformer().await
            }
            _ => Err(Error::UnsupportedArchitecture(
                self.config.architecture.type_.clone()
            ))
        }
    }

    async fn load_bert_classifier(&self) -> Result<Box<dyn Classifier>> {
        // Generic BERT loading (works for any BERT-based model)
        let model_path = self.resolve_model_path().await?;

        let tokenizer = Tokenizer::from_file(
            model_path.join("tokenizer.json")
        )?;

        let config: BertConfig = load_config(&model_path)?;
        let weights = load_safetensors(&model_path)?;

        let model = BertForSequenceClassification::load(
            weights,
            &config,
            self.config.architecture.num_labels,
        )?;

        Ok(Box::new(BertClassifier {
            tokenizer,
            model,
            config: self.config.clone(),
        }))
    }

    async fn resolve_model_path(&self) -> Result<PathBuf> {
        match &self.config.source.type_ {
            SourceType::Local { path } => Ok(path.clone()),
            SourceType::HuggingFace { repo, revision } => {
                self.download_from_hf(repo, revision).await
            }
        }
    }

    async fn download_from_hf(&self, repo: &str, revision: &str) -> Result<PathBuf> {
        // Use hf-hub to download
        let api = hf_hub::api::sync::Api::new()?;
        let model = api.repo(hf_hub::Repo::model(repo.to_string()));

        let model_path = self.cache_dir.join(repo.replace("/", "--"));

        // Download all files
        for file in ["config.json", "tokenizer.json", "vocab.txt", "model.safetensors"] {
            if !model_path.join(file).exists() {
                model.get(file)?;  // Downloads to cache
            }
        }

        Ok(model_path)
    }
}
```

### Usage (No Code Needed)

```rust
// Just load from config
let registry = ModelRegistry::from_file("models/registry.yaml")?;
let classifier = registry.get_classifier("toxicity").await?;

// Works with any model defined in registry.yaml
```

---

## Adding a New Model (User Workflow)

### Option 1: HuggingFace Model

```bash
# 1. Add to registry
cat >> models/registry.yaml <<EOF
  my-custom-toxicity:
    source:
      type: huggingface
      repo: "some-org/my-model"
    architecture:
      type: bert-sequence-classification
      num_labels: 2
EOF

# 2. Reference in classifier config
cat >> config/classifiers.yaml <<EOF
  my-toxicity:
    type: ml
    model: "my-custom-toxicity"
    tier: B
EOF

# 3. Done! CheckStream will auto-download and load
cargo run --features ml-models
```

### Option 2: Local Model

```bash
# 1. Place model files
mkdir -p models/my-model
cp /path/to/model.safetensors models/my-model/
cp /path/to/config.json models/my-model/
cp /path/to/tokenizer.json models/my-model/

# 2. Add to registry
cat >> models/registry.yaml <<EOF
  my-model:
    source:
      type: local
      path: "./models/my-model"
    architecture:
      type: bert-sequence-classification
      num_labels: 3
EOF

# 3. Done!
```

---

## When Do You Need Code?

You **only need to write Rust code** if:

1. **Truly custom architecture** (not BERT/RoBERTa/DistilBERT/etc.)
   - Example: Custom CNN+LSTM hybrid
   - Example: Novel attention mechanism

2. **Custom preprocessing** (beyond config-defined steps)
   - Example: Domain-specific tokenization
   - Example: Multi-modal inputs (text + images)

3. **Custom post-processing**
   - Example: Ensemble of multiple models
   - Example: Rule-based override logic

For **90% of models** (standard transformers from HuggingFace), config is enough.

---

## Roadmap

### Phase 1: Core Infrastructure (Now)
- [x] Manual model loading (toxicity.rs)
- [x] Feature flags for ML
- [ ] Generic BERT loader
- [ ] Model registry YAML parsing

### Phase 2: Dynamic Loading
- [ ] HuggingFace auto-download
- [ ] Config-driven model selection
- [ ] Lazy loading
- [ ] Model caching

### Phase 3: Advanced Features
- [ ] Quantization support
- [ ] A/B testing variants
- [ ] Hot reload
- [ ] Multi-model ensembles

---

## Benefits

### For Users
✅ **No code** for standard models
✅ **Swap models** by editing config
✅ **A/B test** model variants
✅ **Fast iteration** on model selection

### For Developers
✅ **Generic loaders** handle common architectures
✅ **Type-safe** config validation
✅ **Extensible** via custom loaders
✅ **Maintainable** - less model-specific code

---

## Example: Full Workflow

```yaml
# models/registry.yaml - Define available models
models:
  toxicity-v1:
    source: {type: huggingface, repo: "unitary/toxic-bert"}
    architecture: {type: bert-sequence-classification, num_labels: 6}

  toxicity-v2:
    source: {type: local, path: "./models/toxic-distilbert"}
    architecture: {type: distilbert-sequence-classification, num_labels: 6}

# config/classifiers.yaml - Choose which model to use
classifiers:
  toxicity:
    type: ml
    model: "toxicity-v2"  # Just change this to swap models!
    tier: B

# That's it! No code changes needed.
```

---

**Summary**: CheckStream will support **configuration-driven model loading**. Write code only for truly custom architectures. For standard transformers, just edit YAML.
