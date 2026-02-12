# Plugin Operations Guide

This guide covers operating `checkstream-classifiers-ml-plugin` in development and production.

## 1. Runtime Wiring

Use the plugin via `DynamicRegistryBuilder::with_loader`.

```rust
use checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder;
use checkstream_classifiers_ml_plugin::ExternalMlModelLoader;
use std::sync::Arc;

let loader = ExternalMlModelLoader::from_file("models/registry.yaml")?;

let registry = DynamicRegistryBuilder::new()
    .with_loader(Arc::new(loader))
    .preload("toxicity")
    .build()
    .await?;
```

`with_loader` overrides the default in-crate loader for dynamic model-backed classifiers.

## 2. Model Registry Requirements

The plugin reads `ModelRegistry` entries from `models/registry.yaml`.

Required fields per model:
- `source.type`: `local` or `huggingface`
- `architecture.type`: one of:
  - `bert-sequence-classification`
  - `distil-bert-sequence-classification`
  - `roberta-sequence-classification`
  - `deberta-sequence-classification`
  - `xlm-roberta-sequence-classification`
  - `mini-lm-sequence-classification`
  - `sentence-transformer` (requires `pooling: mean|cls`)
- `inference.device`: `cpu`, `cuda`, `cuda:0`, `metal`, or `mps`

For local sources, ensure the path contains:
- `config.json`
- `model.safetensors` (or compatible weights)
- tokenizer files (`tokenizer.json` preferred, `vocab.txt` fallback)

## 3. Device Configuration

Device selection is controlled by `inference.device` in model config:
- `cpu`: safest default
- `cuda`/`cuda:0`: NVIDIA GPU path
- `metal`/`mps`: Apple Silicon path

If GPU initialization fails, the loader returns an error. It does not auto-fallback to CPU.

## 4. HuggingFace Download and Cache

For `source.type: huggingface`, the plugin uses `hf-hub` to download model files.

Practical notes:
- first load downloads model/tokenizer assets
- subsequent loads reuse local cache when available
- cache location is managed by `hf-hub` (typically under user cache directories)

Operational recommendation:
- warm up the service by preloading critical classifiers at startup (`.preload("...")`)

## 5. Fallback Strategy

The plugin itself is strict: load failures return errors.

Recommended fallback options:
- register core built-ins for critical safety classifiers (`with_builtin(...)`)
- preload required models so startup fails fast rather than failing on first request
- implement a custom composite `ModelLoaderPlugin` if you need primary-then-fallback loader behavior

## 6. Smoke Test

Run the included example:

```bash
cargo run --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml --example external_ml_loader --offline
```

Expected behavior:
- logs model loading
- prints a classification result with `label`, `score`, and `latency_us`

## 7. Test Strategy

The plugin includes default-model integration tests in:
- `plugins/checkstream-classifiers-ml-plugin/tests/default_model_tests.rs`

To keep CI deterministic, tests are env-gated:

```bash
CHECKSTREAM_RUN_EXTERNAL_ML_TESTS=1 cargo test --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml
```

These env-gated integration tests require network access to HuggingFace (unless the required assets are already cached locally).

Covered paths:
- DistilBERT default sentiment config (positive/negative/metadata checks)
- Local BERT toxicity model load and score-shape checks (when `./models/toxic-bert` exists)
- Local RoBERTa architecture dispatch path with toxicity score-shape checks
- Deterministic architecture dispatch checks for:
  - DeBERTa sequence classification
  - XLM-RoBERTa sequence classification
  - MiniLM sequence classification
  - SentenceTransformer embedding path

## 8. Security Posture

Core workspace remains hardened because the plugin crate is excluded from workspace membership.

Plugin lockfile may still report unmaintained transitive warnings from the current Candle/tokenizers stack; these are isolated to the plugin path.
