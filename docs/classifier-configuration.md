# Classifier Configuration

CheckStream provides a comprehensive configuration system for managing ML models used in Tier B and Tier C classifiers.

## Configuration File

Create a `classifiers.yaml` file to configure your models:

```yaml
# Default settings
default_device: cpu          # cpu, cuda, or metal
default_quantize: true       # Enable quantization by default
models_dir: ./models         # Directory for local models

# Model configurations
models:
  toxicity:
    repo_id: unitary/toxic-bert
    filename: model.safetensors
    device: cpu
    quantize: true
    tier: B

  sentiment:
    path: ./models/sentiment.safetensors
    tokenizer: ./models/tokenizer.json
    device: cpu
    tier: B
```

## Configuration Schema

### Top-Level Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_device` | string | `cpu` | Default device for all models |
| `default_quantize` | boolean | `false` | Enable quantization by default |
| `models_dir` | path | `./models` | Directory for local model files |
| `models` | object | `{}` | Map of model names to configurations |

### Model Configuration

Each model in the `models` section can be configured with:

#### Source Options

**Local File:**
```yaml
model_name:
  path: ./models/model.safetensors
```

**Hugging Face Hub:**
```yaml
model_name:
  repo_id: organization/model-name
  filename: model.safetensors
  revision: main  # optional, defaults to "main"
```

#### Device Options

**CPU (default):**
```yaml
device: cpu
```

**NVIDIA GPU (CUDA):**
```yaml
device:
  cuda:
    index: 0  # GPU index, optional (defaults to 0)
```

**Apple Silicon (Metal):**
```yaml
device:
  metal:
    index: 0  # optional
```

#### Other Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tokenizer` | path | auto | Path to tokenizer file (optional) |
| `format` | string | `safetensors` | Model format: `safetensors` or `pytorch` |
| `quantize` | boolean | inherited | Enable quantization for this model |
| `tier` | string | - | Performance tier (A, B, or C) for documentation |

## Complete Example

```yaml
# CheckStream Classifiers Configuration

default_device: cpu
default_quantize: true
models_dir: ./models

models:
  # Tier B: Fast toxicity detection
  toxicity:
    repo_id: unitary/toxic-bert
    filename: model.safetensors
    revision: main
    device: cpu
    quantize: true
    tier: B

  # Tier B: Distilled for extra speed
  toxicity-fast:
    repo_id: martin-ha/toxic-comment-model
    filename: pytorch_model.bin
    format: pytorch
    device: cpu
    quantize: true
    tier: B

  # Tier B: Sentiment analysis
  sentiment:
    repo_id: distilbert-base-uncased-finetuned-sst-2-english
    filename: model.safetensors
    device: cpu
    quantize: true
    tier: B

  # Tier B: Prompt injection detection
  prompt-injection:
    repo_id: deepset/deberta-v3-base-injection
    filename: model.safetensors
    device: cpu
    quantize: true
    tier: B

  # Tier C: Custom financial advice model (local)
  financial-advice:
    path: ./models/financial-advice/model.safetensors
    tokenizer: ./models/financial-advice/tokenizer.json
    device: cpu
    quantize: false
    tier: C

  # GPU-accelerated model
  toxicity-gpu:
    repo_id: unitary/toxic-bert
    filename: model.safetensors
    device:
      cuda:
        index: 0
    quantize: false  # Less needed with GPU
    tier: B
```

## Loading Configuration in Code

### Basic Loading

```rust
use checkstream_classifiers::{load_config, init_registry_from_file};

// Load configuration
let config = load_config("./classifiers.yaml")?;

// Initialize registry from config
let registry = init_registry_from_file("./classifiers.yaml")?;
```

### Advanced Usage

```rust
use checkstream_classifiers::{
    load_config, init_registry_from_config, SharedRegistry
};

// Load config
let config = load_config("./classifiers.yaml")?;

// Inspect before loading
println!("Models to load: {:?}", config.model_names());

// Initialize registry
let registry = init_registry_from_config(&config)?;

// Create shared registry for thread-safe access
let shared = SharedRegistry::new(registry);

// Use across application
let arc_registry = shared.clone_arc();
```

### Integration with Proxy

In `main.rs`:

```rust
use checkstream_classifiers::init_registry_from_file;

#[tokio::main]
async fn main() -> Result<()> {
    // Load classifier configuration
    let classifiers_path = config.classifiers_config;
    let model_registry = init_registry_from_file(classifiers_path)?;

    // Pass to application state
    let app_state = AppState {
        model_registry: Arc::new(model_registry),
        // ... other state
    };

    // Use in handlers
    let model = app_state.model_registry.get("toxicity").unwrap();
}
```

## Configuration Best Practices

### 1. Use SafeTensors Format

```yaml
# Preferred
toxicity:
  repo_id: unitary/toxic-bert
  filename: model.safetensors  # ✓ Fast and safe

# Avoid if possible
toxicity:
  filename: pytorch_model.bin   # ✗ Slower to load
  format: pytorch
```

### 2. Enable Quantization for CPU

```yaml
# Good for CPU inference
toxicity:
  device: cpu
  quantize: true  # ✓ 1.5-2x speedup

# GPU inference
toxicity-gpu:
  device:
    cuda:
      index: 0
  quantize: false  # Not as beneficial on GPU
```

### 3. Organize by Tier

```yaml
models:
  # Tier B models (<5ms)
  toxicity:
    # ... distilled or quantized model
    tier: B

  # Tier C models (<10ms)
  financial-advice:
    # ... full-size model acceptable
    tier: C
```

### 4. Local Models for Production

```yaml
# Development: Use HF for convenience
toxicity:
  repo_id: unitary/toxic-bert
  filename: model.safetensors

# Production: Use local for reliability
toxicity:
  path: /opt/checkstream/models/toxicity/model.safetensors
  tokenizer: /opt/checkstream/models/toxicity/tokenizer.json
```

### 5. Document Tier and Purpose

```yaml
toxicity:
  repo_id: unitary/toxic-bert
  filename: model.safetensors
  tier: B  # Latency target: <5ms
  # Purpose: Detect toxic/harmful content in real-time
```

## Environment-Specific Configs

### Development

```yaml
# classifiers.dev.yaml
default_device: cpu
default_quantize: true

models:
  toxicity:
    repo_id: martin-ha/toxic-comment-model  # Smaller model
    filename: pytorch_model.bin
```

### Production

```yaml
# classifiers.prod.yaml
default_device:
  cuda:
    index: 0
default_quantize: false

models:
  toxicity:
    path: /opt/models/toxicity.safetensors  # Pre-downloaded
    tokenizer: /opt/models/toxicity-tokenizer.json
```

## Validation

The configuration system validates:
- File paths exist (for local models)
- Required fields are present
- Device specifications are valid
- Format options are recognized

Errors are reported at startup with helpful messages:

```
Error: Model file not found: ./models/missing.safetensors
Error: Invalid device specification: invalid_device
Error: Model 'toxicity' missing required field: filename
```

## Hot Reloading

To reload models without restart:

```rust
// Not currently supported - restart required
// Future enhancement: watch config file for changes
```

## Default Models

CheckStream can ship with a default configuration:

```yaml
# Built-in defaults (can be overridden)
models:
  toxicity:
    repo_id: unitary/toxic-bert
    filename: model.safetensors
    tier: B

  # Users can override or add models
```

## Troubleshooting

### Model Download Fails

```
Error: Failed to download model from HF: Network error
```

**Solution:** Check internet connection or use local models

### Device Not Available

```
Error: Failed to create CUDA device: CUDA not available
```

**Solution:** Fall back to CPU in config:

```yaml
device: cpu  # Change from cuda
```

### Out of Memory

```
Error: Out of memory when loading model
```

**Solutions:**
- Enable quantization
- Use smaller model
- Increase system memory
- Use GPU if available

### Tokenizer Not Found

```
Warning: Tokenizer not found, attempting auto-download
```

**Solution:** Specify explicit tokenizer path or ensure HF download succeeds

## Reference Configuration

See `classifiers.yaml` in the repository root for a complete reference configuration with all available models and options.

## Pipelines

CheckStream supports classifier pipelines for chaining and parallel execution. See [pipeline-configuration.md](pipeline-configuration.md) for comprehensive documentation on:

- Creating multi-stage pipelines
- Parallel and sequential execution
- Conditional logic
- Result aggregation strategies
- Performance optimization patterns

Quick example:

```yaml
pipelines:
  basic-safety:
    description: "Quick safety check"
    stages:
      - type: parallel
        name: safety-check
        classifiers:
          - toxicity
          - sentiment
        aggregation: max_score
```

## API Reference

See [model-loading.md](model-loading.md) for detailed API documentation on using loaded models programmatically.

See [pipeline-configuration.md](pipeline-configuration.md) for pipeline configuration and usage.
