# Model Loading with Candle

CheckStream uses [Candle](https://github.com/huggingface/candle), a minimalist ML framework for Rust, to load and run machine learning models for Tier B and Tier C classifiers.

## Overview

Candle provides:
- **Native Rust inference**: No Python dependencies
- **Multiple backends**: CPU, CUDA (NVIDIA), Metal (Apple Silicon)
- **SafeTensors support**: Fast and safe model loading
- **Hugging Face integration**: Easy model downloading
- **Quantization support**: For faster inference

## Model Formats Supported

1. **SafeTensors** (recommended)
   - Fast loading
   - Safe deserialization
   - Memory efficient
   - `.safetensors` extension

2. **PyTorch**
   - Standard PyTorch format
   - `.pt`, `.pth`, `.bin` extensions

## Loading Models

### From Local File

```rust
use checkstream_classifiers::{ModelConfig, DeviceType};

// Load from local SafeTensors file
let config = ModelConfig::from_local("./models/toxicity-model.safetensors")
    .with_tokenizer("./models/tokenizer.json")
    .with_device(DeviceType::Cpu);

let model = LoadedModel::load(config)?;
```

### From Hugging Face Hub

```rust
// Download and load from Hugging Face
let config = ModelConfig::from_hf(
    "unitary/toxic-bert",  // repo_id
    "model.safetensors"     // filename
)
.with_revision("main")
.with_device(DeviceType::Cpu);

let model = LoadedModel::load(config)?;
```

The model will be automatically downloaded and cached locally.

### With GPU Acceleration

```rust
// CUDA (NVIDIA)
let config = ModelConfig::from_local("./models/model.safetensors")
    .with_device(DeviceType::Cuda(0));  // GPU index 0

// Metal (Apple Silicon)
let config = ModelConfig::from_local("./models/model.safetensors")
    .with_device(DeviceType::Metal(0));
```

### With Quantization

```rust
let config = ModelConfig::from_local("./models/model.safetensors")
    .with_quantization(true)  // Enable quantization for faster inference
    .with_device(DeviceType::Cpu);
```

## Model Registry

Use `ModelRegistry` to manage multiple models:

```rust
use checkstream_classifiers::ModelRegistry;

let mut registry = ModelRegistry::new();

// Load and register multiple models
registry.load_and_register(
    "toxicity",
    ModelConfig::from_hf("unitary/toxic-bert", "model.safetensors")
)?;

registry.load_and_register(
    "sentiment",
    ModelConfig::from_local("./models/sentiment.safetensors")
)?;

// Retrieve models
let toxicity_model = registry.get("toxicity").unwrap();
let sentiment_model = registry.get("sentiment").unwrap();

// List registered models
for name in registry.model_names() {
    println!("Loaded model: {}", name);
}
```

## Building a Custom Classifier

Here's an example of building a Tier B toxicity classifier using a loaded Candle model:

```rust
use candle_core::Tensor;
use candle_transformers::models::bert::{BertModel, Config};
use checkstream_classifiers::{
    Classifier, ClassificationResult, ClassificationMetadata,
    LoadedModel, ModelConfig, DeviceType
};

pub struct CandleToxicityClassifier {
    model: Arc<LoadedModel>,
    bert: BertModel,
}

impl CandleToxicityClassifier {
    pub fn new() -> Result<Self> {
        // Load model weights
        let config = ModelConfig::from_hf(
            "unitary/toxic-bert",
            "model.safetensors"
        )
        .with_device(DeviceType::Cpu);

        let loaded = LoadedModel::load(config)?;

        // Build BERT model using loaded weights
        let bert_config = Config::default();
        let vb = loaded.var_builder();
        let bert = BertModel::load(vb, &bert_config)?;

        Ok(Self {
            model: Arc::new(loaded),
            bert,
        })
    }

    fn tokenize(&self, text: &str) -> Result<Tensor> {
        let tokenizer = self.model.tokenizer()
            .ok_or_else(|| Error::classifier("No tokenizer available"))?;

        let encoding = tokenizer.encode(text, true)
            .map_err(|e| Error::classifier(format!("Tokenization failed: {}", e)))?;

        let tokens = encoding.get_ids();
        let tensor = Tensor::new(tokens, self.model.device())?;

        Ok(tensor)
    }
}

#[async_trait]
impl Classifier for CandleToxicityClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Tokenize input
        let input_ids = self.tokenize(text)?;

        // Run model inference
        let output = self.bert.forward(&input_ids)?;

        // Extract logits and compute score
        let logits = output.squeeze(0)?;
        let probs = candle_nn::ops::softmax(&logits, 0)?;
        let score = probs.get(1)?.to_scalar::<f32>()?;  // Toxic class

        let label = if score > 0.5 { "toxic" } else { "safe" };

        Ok(ClassificationResult {
            label: label.to_string(),
            score,
            metadata: ClassificationMetadata::default(),
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        "candle_toxicity"
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}
```

## Recommended Models for CheckStream

### Tier B (<5ms target)

**Toxicity Detection:**
- `unitary/toxic-bert` - Fine-tuned BERT for toxicity
- `martin-ha/toxic-comment-model` - Distilled toxicity classifier
- Custom distilled models (768 hidden size or smaller)

**Prompt Injection:**
- `deepset/deberta-v3-base-injection` - Prompt injection detection
- Custom fine-tuned models on injection datasets

**Sentiment Analysis:**
- `distilbert-base-uncased-finetuned-sst-2-english` - Fast sentiment
- `cardiffnlp/twitter-roberta-base-sentiment` - Social media sentiment

### Tier C (<10ms target)

**Financial Advice Detection:**
- Fine-tuned BERT/RoBERTa models on financial text
- Custom models for investment advice classification

**Complexity/Readability:**
- Custom readability classifiers
- Text complexity scoring models

## Model Optimization Tips

### 1. Use Quantization
```rust
let config = ModelConfig::from_local("./models/model.safetensors")
    .with_quantization(true);  // Faster inference, lower memory
```

### 2. Batch Processing
Process multiple inputs together when possible:
```rust
// Instead of classifying one at a time
for text in texts {
    classifier.classify(text).await?;
}

// Batch them for better throughput
let results = classifier.classify_batch(&texts).await?;
```

### 3. Model Caching
Load models once and reuse:
```rust
// Good: Load once, use many times
let classifier = ToxicityClassifier::new()?;
for text in texts {
    classifier.classify(text).await?;
}

// Bad: Loading on every request
for text in texts {
    let classifier = ToxicityClassifier::new()?;  // Slow!
    classifier.classify(text).await?;
}
```

### 4. GPU Acceleration
For higher throughput deployments:
```rust
let config = ModelConfig::from_local("./models/model.safetensors")
    .with_device(DeviceType::Cuda(0));  // Use GPU
```

## Converting Models to SafeTensors

If you have a PyTorch model, convert it to SafeTensors for faster loading:

```python
from transformers import AutoModel
from safetensors.torch import save_file

# Load PyTorch model
model = AutoModel.from_pretrained("model-name")

# Save as SafeTensors
save_file(model.state_dict(), "model.safetensors")
```

## Model Directory Structure

Organize models in your project:

```
checkstream/
├── models/
│   ├── toxicity/
│   │   ├── model.safetensors
│   │   ├── tokenizer.json
│   │   └── config.json
│   ├── sentiment/
│   │   ├── model.safetensors
│   │   └── tokenizer.json
│   └── prompt-injection/
│       ├── model.safetensors
│       └── tokenizer.json
└── ...
```

## Discovering Models

Use the `discover_models` helper to find all models in a directory:

```rust
use checkstream_classifiers::discover_models;

let model_paths = discover_models("./models")?;
for path in model_paths {
    println!("Found model: {:?}", path);
}
```

## Performance Benchmarking

Always benchmark your models to ensure they meet latency targets:

```rust
use std::time::Instant;

let classifier = ToxicityClassifier::new()?;

// Warmup
for _ in 0..10 {
    classifier.classify("test text").await?;
}

// Benchmark
let start = Instant::now();
let iterations = 1000;

for _ in 0..iterations {
    classifier.classify("This is a test message").await?;
}

let avg_latency = start.elapsed() / iterations;
println!("Average latency: {:?}", avg_latency);

// Should be < 5ms for Tier B
assert!(avg_latency.as_millis() < 5);
```

## Troubleshooting

### Model Not Found
```
Error: Model file not found: ./models/model.safetensors
```
**Solution**: Ensure the file exists or use `ModelSource::HuggingFace` to auto-download.

### CUDA Out of Memory
```
Error: Failed to create CUDA device: Out of memory
```
**Solution**:
- Use smaller models
- Enable quantization
- Reduce batch size
- Fall back to CPU

### Slow Inference
**Solutions**:
- Enable quantization
- Use GPU if available
- Use smaller/distilled models
- Batch requests
- Cache tokenizer outputs

### Tokenizer Issues
```
Error: Failed to load tokenizer
```
**Solution**: Ensure tokenizer.json is in the correct format (HuggingFace tokenizers format).

## Example: Complete Integration

```rust
// Initialize model registry at startup
let mut registry = ModelRegistry::new();

registry.load_and_register(
    "toxicity",
    ModelConfig::from_hf("unitary/toxic-bert", "model.safetensors")
        .with_device(DeviceType::Cpu)
        .with_quantization(true)
)?;

// Use in classifier
pub struct TierBClassifier {
    model: Arc<LoadedModel>,
}

impl TierBClassifier {
    pub fn from_registry(registry: &ModelRegistry, name: &str) -> Result<Self> {
        let model = registry.get(name)
            .ok_or_else(|| Error::classifier(format!("Model {} not found", name)))?;

        Ok(Self { model })
    }
}

// In your application
let registry = Arc::new(registry);
let classifier = TierBClassifier::from_registry(&registry, "toxicity")?;

// Run classifications
let result = classifier.classify("input text").await?;
```

## Resources

- [Candle Documentation](https://github.com/huggingface/candle)
- [Candle Examples](https://github.com/huggingface/candle/tree/main/candle-examples)
- [Hugging Face Model Hub](https://huggingface.co/models)
- [SafeTensors Format](https://github.com/huggingface/safetensors)

---

For questions or issues with model loading, check the CheckStream issues page or refer to the Candle documentation.
