# Changelog

All notable changes to CheckStream will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - Production Hardening (2025-12-12)

**Major Updates: Working ML Models, Comprehensive Testing, Production Features**

#### ML Classifier Fixes (Critical)
- **Attention Mask Inversion**: Fixed Candle's inverted attention mask semantics
  - Candle uses: 0 = attend, 1 = mask (opposite of HuggingFace)
  - Now correctly converts attention masks for proper model inference
- **Pre-classifier Layer**: Added missing `pre_classifier` with ReLU activation
  - DistilBERT sequence classification requires: pre_classifier → ReLU → classifier
  - Without this layer, outputs were non-discriminative (~0.5 for all classes)
- **Input Type Conversion**: Fixed i64 type requirement for transformer input_ids
  - Candle transformers expect i64 tensors, not u32

#### Working HuggingFace Integration
- **Live Model Demo**: `cargo run --example test_hf_model --features ml-models`
  - Downloads DistilBERT sentiment model from HuggingFace
  - Runs inference with >99% accuracy on test cases
  - "I love this movie!" → positive (1.000)
  - "This is terrible." → negative (1.000)

#### Production Features
- **Graceful Shutdown**: SIGTERM/SIGINT signal handling
  - Clean connection draining on shutdown
  - AtomicBool shutdown flag for coordinated termination
- **Kubernetes Health Probes**:
  - `GET /health/live` - Liveness probe (always returns healthy)
  - `GET /health/ready` - Readiness probe (checks classifiers, policies, audit service)

#### Comprehensive Testing (122 tests)
- **ML Classifier Tests** (12 tests):
  - Positive/negative sentiment classification
  - Batch classification accuracy
  - Metadata and all_scores verification
  - Latency measurement (<1 second on CPU)
  - Edge cases: empty input, long input truncation, special characters
  - Error handling: non-existent models, missing registry entries
- **Integration Tests** (10 tests):
  - Policy pattern matching (case-sensitive/insensitive)
  - Classifier trigger evaluation with thresholds
  - Action execution (Stop, Redact, Log, Audit)
  - Composite triggers (AND, OR operators)
  - Multiple rules in single policy
  - Disabled rule handling

### Changed

#### Documentation Cleanup
- Consolidated documentation to single README.md + docs/ folder
- Removed 25+ redundant documentation files
- Updated README.md with accurate current status
- All documentation links now point to valid files

#### Performance Metrics (Actual)
- Pattern classifier: ~0.5ms (Tier A)
- ML classifier (CPU): ~30-50ms (Tier B)
- ML classifier (GPU, estimated): ~2-10ms
- Policy evaluation: ~0.2ms
- Total overhead (patterns only): ~5-8ms

### Fixed
- NaN values in ML model hidden states output
- Non-discriminative classification scores
- Integration test API compatibility
- All 122 tests now passing

## [0.1.0] - 2025-11-15

### Added - Dynamic Model Loading

**Major Feature: Configuration-Driven ML Model Management**

Users can now add, swap, and manage ML models without writing Rust code - just by editing YAML configuration files.

#### Core Implementation
- **GenericModelLoader** - Universal loader for BERT-family architectures
  - Support for BERT, RoBERTa, DistilBERT, DeBERTa, ALBERT
  - Auto-download from HuggingFace Hub
  - Device selection (CPU, CUDA, MPS)
  - Memory-mapped SafeTensors loading

- **DynamicClassifierRegistry** - Runtime classifier management
  - Lazy loading - models load on first use
  - Automatic caching (~5µs subsequent access)
  - Mix ML and pattern-based classifiers seamlessly
  - Thread-safe with RwLock

- **ModelConfig** - Type-safe YAML configuration
  - Multiple model sources (HuggingFace, local, builtin)
  - Inference configuration (device, max_length, threshold)
  - Preprocessing pipeline configuration

### Added - Classifier Pipeline System (2025-11-13)

#### Core Pipeline Implementation
- **Classifier Pipeline System** with four stage types:
  - Single, Parallel, Sequential, Conditional
- **Six aggregation strategies**:
  - All, MaxScore, MinScore, FirstPositive, Unanimous, WeightedAverage
- Async execution with Tokio parallelism
- Microsecond-precision latency tracking

#### Configuration System
- YAML-based pipeline configuration
- Configuration-driven pipeline builder
- Validates classifier references at build time

### Added - Initial Rust Implementation

#### Core Infrastructure
- Workspace structure with 5 crates:
  - `checkstream-core` - Core types and utilities
  - `checkstream-proxy` - HTTP/SSE proxy server
  - `checkstream-classifiers` - ML classifier system
  - `checkstream-policy` - Policy engine
  - `checkstream-telemetry` - Observability

#### Three-Phase Pipeline
- **Ingress**: Validate prompts before LLM (~3ms)
- **Midstream**: Check streaming chunks (~2ms/chunk)
- **Egress**: Final compliance check (async)

#### Policy Engine
- Pattern-based triggers (regex, substring)
- Classifier triggers with thresholds
- Composite triggers (AND, OR operators)
- Actions: Stop, Redact, Log, Audit

#### Action Executor
- Stop with custom message and status code
- Redact with replacement text
- Log with configurable level
- Audit with category and severity

#### Audit Trail
- Hash-chained tamper-proof logging
- Query API endpoint
- Async persistence

## Performance Metrics

### Classifier Tiers
| Tier | Target | Actual | Use Case |
|------|--------|--------|----------|
| A | <2ms | ~0.5ms | Pattern matching, PII |
| B | <50ms | ~30-50ms | ML models (CPU) |
| B | <10ms | ~2-10ms | ML models (GPU) |

### Pipeline Overhead
- Parallel stages: max(classifier_latencies) + ~50μs
- Sequential stages: sum(classifier_latencies) + ~30μs per classifier
- Conditional stages: ~5μs when skipped
- Aggregation: <10μs for all strategies

## Breaking Changes

None - initial release.

## Links

- [Documentation](docs/)
- [Architecture](docs/architecture.md)
- [Getting Started](docs/getting-started.md)
- [Model Loading](docs/model-loading.md)
- [Pipeline Configuration](docs/pipeline-configuration.md)
- [Policy Engine](docs/policy-engine.md)
- [API Reference](docs/api-reference.md)

---

**Built for trust at the speed of generation.**
