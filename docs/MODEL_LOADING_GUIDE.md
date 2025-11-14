# Model Loading Guide

Guide for loading real ML models into CheckStream classifiers.

## Overview

CheckStream uses [Candle](https://github.com/huggingface/candle), a Rust-native ML framework from Hugging Face, for model inference. This guide shows how to load real models for production use.

## Supported Model Formats

- **SafeTensors** (Recommended): Fast, safe format from Hugging Face
- **PyTorch**: Traditional `.pt` or `.pth` files
- **ONNX**: Not currently supported (use Candle-compatible models)

## Quick Start: Loading a Toxicity Model

### 1. Choose a Model

Recommended models for toxicity detection:

**Option A: DistilBERT Toxicity** (Fast, ~130MB)
- Model: `unitary/toxic-bert`
- Speed: ~3-5ms on CPU
- Good for: Phase 2 midstream checks

**Option B: RoBERTa Hate Speech** (Accurate, ~500MB)
- Model: `facebook/roberta-hate-speech-dynabench-r4-target`
- Speed: ~8-12ms on CPU
- Good for: Phase 1 ingress or Phase 3 egress

### 2. Download Model Files

Using `hf-hub` (already a dependency):

```rust
use hf_hub::api::sync::Api;
use std::path::PathBuf;

fn download_toxicity_model() -> anyhow::Result<PathBuf> {
    let api = Api::new()?;
    let repo = api.model("unitary/toxic-bert".to_string());

    // Download model files
    let model_file = repo.get("model.safetensors")?;
    let config_file = repo.get("config.json")?;
    let tokenizer_file = repo.get("tokenizer.json")?;

    Ok(model_file.parent().unwrap().to_path_buf())
}
```

Or manually from Hugging Face:

```bash
# Install huggingface-cli
pip install huggingface-hub

# Download model
huggingface-cli download unitary/toxic-bert \
  --local-dir ./models/toxic-bert \
  --include "*.safetensors" "*.json"
```

### 3. Update Configuration

Edit `classifiers.yaml`:

```yaml
models:
  toxicity-bert:
    name: "DistilBERT Toxicity Classifier"
    tier: B
    source:
      type: huggingface
      repo: "unitary/toxic-bert"
      revision: "main"
    # OR for local files:
    # source:
    #   type: local
    #   path: "./models/toxic-bert"
    device: cpu  # or cuda, metal
    quantization: none  # or int8 for 1.5-2x speedup
```

### 4. Implement the Classifier

Update `crates/checkstream-classifiers/src/toxicity.rs`:

```rust
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;

pub struct ToxicityClassifier {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl ToxicityClassifier {
    pub fn new() -> Result<Self> {
        // Load model from config
        let model_dir = std::path::Path::new("./models/toxic-bert");

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(
            model_dir.join("tokenizer.json")
        )?;

        // Load config
        let config_path = model_dir.join("config.json");
        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(config_path)?
        )?;

        // Set device
        let device = Device::Cpu;

        // Load model weights
        let vb = VarBuilder::from_safetensors(
            vec![model_dir.join("model.safetensors")],
            candle_core::DType::F32,
            &device
        )?;

        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }
}

#[async_trait::async_trait]
impl Classifier for ToxicityClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| Error::classifier(format!("Tokenization failed: {}", e)))?;

        let tokens = encoding.get_ids();
        let token_ids = Tensor::new(tokens, &self.device)?
            .unsqueeze(0)?;  // Add batch dimension

        // Run inference
        let outputs = self.model.forward(&token_ids)?;

        // Get logits and convert to probabilities
        let logits = outputs.last_hidden_state()?;
        let pooled = logits.mean(1)?;  // Mean pooling

        // Simple threshold for now (TODO: add classification head)
        let values = pooled.to_vec2::<f32>()?;
        let score = values[0][0].abs().min(1.0);  // Simplified

        let latency = start.elapsed();

        Ok(ClassificationResult {
            score,
            decision: if score > 0.7 {
                ClassificationDecision::Block
            } else {
                ClassificationDecision::Allow
            },
            tier: ClassifierTier::B,
            latency_us: latency.as_micros() as u64,
            metadata: HashMap::from([
                ("model".to_string(), "toxic-bert".to_string()),
                ("confidence".to_string(), format!("{:.3}", score)),
            ]),
        })
    }

    fn name(&self) -> &str {
        "toxicity-bert"
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}
```

### 5. Test the Model

Create `examples/test_toxicity_model.rs`:

```rust
use checkstream_classifiers::ToxicityClassifier;
use checkstream_classifiers::Classifier;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading toxicity model...");
    let classifier = ToxicityClassifier::new()?;

    // Test cases
    let test_cases = vec![
        "Hello, how are you?",                    // Should be safe
        "I hate you and hope you die!",          // Should be toxic
        "This is amazing work!",                  // Should be safe
        "You're an idiot and worthless",         // Should be toxic
    ];

    for text in test_cases {
        println!("\nInput: {}", text);

        let result = classifier.classify(text).await?;

        println!("  Score: {:.3}", result.score);
        println!("  Decision: {:?}", result.decision);
        println!("  Latency: {}µs", result.latency_us);
    }

    Ok(())
}
```

Run it:

```bash
cargo run --example test_toxicity_model
```

## Model Types by Use Case

### Toxicity Detection

**Fast (Tier B, <5ms)**:
- `unitary/toxic-bert` - DistilBERT fine-tuned on toxic comments
- `unitary/unbiased-toxic-roberta` - Debiased version

**Accurate (Tier C, <10ms)**:
- `martin-ha/toxic-comment-model` - Multi-label toxicity
- `facebook/roberta-hate-speech-dynabench-r4-target` - Hate speech

### Sentiment Analysis

**Fast**:
- `distilbert-base-uncased-finetuned-sst-2-english` - Binary sentiment
- `cardiffnlp/twitter-roberta-base-sentiment` - 3-class sentiment

**Accurate**:
- `nlptown/bert-base-multilingual-uncased-sentiment` - 5-star ratings

### Prompt Injection Detection

**Specialized**:
- `deepset/deberta-v3-base-injection` - Prompt injection detection
- `protectai/deberta-v3-base-prompt-injection-v2` - Latest version

### PII Detection

Currently using regex patterns (Tier A). For ML-based:
- Custom NER model trained on PII datasets
- `dslim/bert-base-NER` - General NER (adaptable for PII)

## Performance Optimization

### 1. Quantization

Use INT8 quantization for 1.5-2x speedup:

```yaml
models:
  toxicity-bert:
    quantization: int8  # or int4 for even faster (less accurate)
```

In code:

```rust
let vb = VarBuilder::from_safetensors(
    vec![model_path],
    candle_core::DType::U8,  // INT8
    &device
)?;
```

### 2. Model Caching

Keep models loaded in memory:

```rust
lazy_static! {
    static ref TOXICITY_MODEL: ToxicityClassifier =
        ToxicityClassifier::new().expect("Failed to load model");
}
```

### 3. Batch Processing

Process multiple texts together:

```rust
pub async fn classify_batch(&self, texts: &[&str]) -> Result<Vec<ClassificationResult>> {
    // Tokenize all at once
    let encodings: Vec<_> = texts.iter()
        .map(|t| self.tokenizer.encode(*t, true))
        .collect::<Result<_>>()?;

    // Create batch tensor
    let batch = stack_tensors(encodings)?;

    // Single forward pass
    let outputs = self.model.forward(&batch)?;

    // Process results
    outputs.iter().map(|o| process_output(o)).collect()
}
```

### 4. GPU Acceleration

Use CUDA or Metal for faster inference:

```yaml
models:
  toxicity-bert:
    device: cuda  # or metal on macOS
```

```rust
let device = if cfg!(feature = "cuda") {
    Device::new_cuda(0)?
} else if cfg!(feature = "metal") {
    Device::new_metal(0)?
} else {
    Device::Cpu
};
```

## Troubleshooting

### "Model file not found"

Check the model was downloaded:

```bash
ls -la ./models/toxic-bert/
```

Should see:
- `model.safetensors`
- `config.json`
- `tokenizer.json`

### "Out of memory"

Try quantization:

```yaml
quantization: int8  # Reduces memory by ~4x
```

Or use a smaller model:

```yaml
source:
  repo: "unitary/toxic-bert"  # Instead of roberta-large
```

### "Slow inference (>10ms)"

1. Check device is correct (GPU if available)
2. Enable quantization
3. Use smaller model for Phase 2
4. Batch requests if possible

### "Low accuracy"

1. Use larger model (Tier C instead of Tier B)
2. Disable quantization
3. Fine-tune model on your data
4. Adjust classification threshold

## Best Practices

### Model Selection

**Phase 1 (Ingress)**:
- Use fast models (Tier B, <5ms)
- Focus on blocking clearly unsafe content
- Higher precision (fewer false positives)

**Phase 2 (Midstream)**:
- Use fastest models (Tier A/B, <3ms)
- Can have higher false positive rate (just redacts)
- Consider using quantized versions

**Phase 3 (Egress)**:
- Can use slower, more accurate models (Tier C, <50ms)
- Runs async, no latency impact
- Focus on compliance and audit trail

### Pipeline Design

```yaml
pipelines:
  ingress:
    stages:
      - type: parallel
        classifiers: [pii, toxicity-fast]  # Fast checks only
        aggregation: max_score

  midstream:
    stages:
      - type: single
        classifier: toxicity-distilled  # Ultra-fast, quantized

  egress:
    stages:
      - type: sequential
        classifiers:
          - toxicity-full     # Full model
          - sentiment
          - prompt-injection
          - custom-compliance
```

### Testing

Always benchmark models before production:

```bash
# Create benchmark
cargo bench --bench classifier_bench

# Expected output:
# toxicity-bert/cpu      3.2ms  ± 0.4ms
# toxicity-bert/cuda     0.8ms  ± 0.1ms
# toxicity-bert/int8     2.1ms  ± 0.3ms
```

## Next Steps

1. **Load your first model**: Follow the Quick Start above
2. **Benchmark performance**: Ensure <5ms for Tier B models
3. **Test accuracy**: Validate on your use cases
4. **Optimize**: Try quantization, GPU, batching
5. **Deploy**: Update proxy configuration and restart

## Resources

- [Candle Documentation](https://github.com/huggingface/candle)
- [Hugging Face Hub](https://huggingface.co/models)
- [Model Quantization Guide](https://huggingface.co/docs/transformers/quantization)
- [CheckStream Architecture](architecture.md)

---

**Questions?** Open an issue on GitHub or check the [troubleshooting](#troubleshooting) section.
