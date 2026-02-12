# checkstream-classifiers-ml-plugin

External ML inference plugin for `checkstream-classifiers`.

This crate adds Candle + HuggingFace runtime model loading via the
`ModelLoaderPlugin` interface and can be injected into
`DynamicRegistryBuilder::with_loader`.

Operational guidance is documented in `plugins/checkstream-classifiers-ml-plugin/plugins.md`.

## Supported Architectures

- `bert-sequence-classification`
- `distil-bert-sequence-classification`
- `roberta-sequence-classification` (BERT-compatible checkpoint path)
- `deberta-sequence-classification`
- `xlm-roberta-sequence-classification`
- `mini-lm-sequence-classification`
- `sentence-transformer` (embedding path with `mean` or `cls` pooling)

## Usage

```rust
use checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder;
use checkstream_classifiers_ml_plugin::ExternalMlModelLoader;
use std::sync::Arc;

let loader = ExternalMlModelLoader::from_file("models/registry.yaml")?;
let registry = DynamicRegistryBuilder::new()
    .with_loader(Arc::new(loader))
    .build()
    .await?;
```

## Build

```bash
cargo check --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml
```

## Tests

Default run (deterministic, integration tests are env-gated):

```bash
cargo test --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml --offline
```

Run external-model integration tests (DistilBERT defaults, local BERT/RoBERTa when available):

```bash
CHECKSTREAM_RUN_EXTERNAL_ML_TESTS=1 cargo test --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml
```

Note: env-gated integration tests require reachable HuggingFace artifacts (or pre-cached model files).

Deterministic architecture-dispatch tests (DeBERTa/XLM-R/MiniLM/SentenceTransformer) run in normal `cargo test` and do not require network/model downloads.

## Smoke Test

```bash
cargo run --manifest-path plugins/checkstream-classifiers-ml-plugin/Cargo.toml --example external_ml_loader --offline
```
