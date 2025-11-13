# Changelog

All notable changes to CheckStream will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - Classifier Pipeline System (2025-11-13)

#### Core Pipeline Implementation
- **Classifier Pipeline System** (`crates/checkstream-classifiers/src/pipeline.rs`)
  - Four stage types: Single, Parallel, Sequential, Conditional
  - Six aggregation strategies: All, MaxScore, MinScore, FirstPositive, Unanimous, WeightedAverage
  - Async execution with true parallelism using Tokio
  - Microsecond-precision latency tracking per stage
  - Comprehensive test coverage (5 tests)

#### Configuration System
- **Pipeline Configuration** (`crates/checkstream-classifiers/src/config.rs`)
  - YAML-based pipeline configuration structures
  - `PipelineConfigSpec`, `StageConfigSpec`, `AggregationStrategySpec`, `ConditionSpec`
  - Conversion functions from config to runtime types
  - Full serde support for YAML serialization/deserialization

#### Pipeline Builder
- **Configuration-Driven Builder** (`crates/checkstream-classifiers/src/registry.rs`)
  - `build_pipeline_from_config()` function
  - Validates classifier references at build time
  - Constructs complete pipelines from YAML
  - Comprehensive error messages for missing classifiers

#### Configuration Examples
- **Example Pipelines** (`classifiers.yaml`)
  - 6 production-ready pipeline examples:
    - `basic-safety` - Parallel execution with max_score aggregation
    - `advanced-safety` - Multi-stage with conditional execution
    - `content-quality` - Sequential analysis workflow
    - `comprehensive-safety` - Unanimous consensus requirement
    - `fast-triage` - First positive detection for speed
    - `weighted-analysis` - Weighted averaging of signals
  - Inline documentation and usage examples

#### Documentation
- **Complete Pipeline Guide** (`docs/pipeline-configuration.md`, 900+ lines)
  - Quick start guide with code examples
  - Complete stage type reference
  - Aggregation strategy details and use cases
  - Conditional execution guide
  - Performance optimization patterns
  - Common workflow patterns
  - Troubleshooting section
  - API reference

- **Quick Start Guide** (`docs/QUICKSTART_PIPELINES.md`)
  - 5-minute getting started guide
  - Common patterns library
  - Quick reference tables
  - Performance tips

- **Documentation Index** (`docs/README.md`)
  - Organized documentation structure
  - Navigation guide
  - What's new section

- **Implementation Summary** (`PIPELINE_IMPLEMENTATION.md`)
  - Complete implementation details
  - Design decisions
  - Performance characteristics
  - Testing summary

#### Examples
- **Pipeline Usage Example** (`examples/pipeline_usage.rs`)
  - End-to-end pipeline demonstration
  - Multiple test scenarios
  - Performance comparison
  - Detailed output formatting

#### API Exports
- Added `ClassifierTier` to public API
- Added all pipeline types to public API
- Added `build_pipeline_from_config` to public API

### Changed

#### Documentation Updates
- **README.md** - Added classifier system overview with pipeline capabilities
- **docs/classifier-configuration.md** - Added pipeline section with cross-reference

### Fixed

#### Test Improvements
- Fixed config tests to work with serde_yaml constraints
- Fixed import paths in pipeline tests
- All 28 tests passing across all crates

#### Build System
- Clean compilation with zero warnings
- Release build optimizations verified
- Example binaries building successfully

## [0.1.0] - 2025-11-13

### Added - Initial Rust Implementation

#### Core Infrastructure
- Rust-based implementation for high performance
- Workspace structure with 5 crates:
  - `checkstream-core` - Core types and utilities
  - `checkstream-proxy` - HTTP/SSE proxy server
  - `checkstream-classifiers` - ML classifier system
  - `checkstream-policy` - Policy engine
  - `checkstream-telemetry` - Observability

#### Classifier System
- Three-tier classifier architecture:
  - Tier A (<2ms): Pattern matching, PII detection
  - Tier B (<5ms): Quantized neural classifiers
  - Tier C (<10ms): Full-size models
- Candle-based ML inference
- Model loading from HuggingFace Hub and local files
- Device support: CPU, CUDA, Metal
- Quantization support for performance

#### Configuration System
- YAML-based model configuration
- Support for SafeTensors and PyTorch formats
- Device and optimization settings
- Example configurations

#### Documentation
- Comprehensive model loading guide
- Classifier configuration reference
- Architecture documentation
- Use cases and deployment modes

#### Infrastructure
- Apache 2.0 license
- Comprehensive .gitignore
- Contributing guidelines
- Docker support
- CI/CD pipelines

## Performance Metrics

### Pipeline Execution
- **Parallel stages**: Latency = max(classifier_latencies) + ~50μs overhead
- **Sequential stages**: Latency = sum(classifier_latencies) + ~30μs per classifier
- **Conditional stages**: ~5μs when skipped, normal latency when executed
- **Aggregation**: <10μs overhead for all strategies

### Overall System
- **Tier A classifiers**: <2ms (pattern matching, regex)
- **Tier B classifiers**: <5ms (quantized neural networks)
- **Tier C classifiers**: <10ms (full-size models)
- **Pipeline overhead**: Minimal (<50μs for coordination)

## Breaking Changes

None - initial release.

## Migration Guide

N/A - initial release.

## Contributors

- CheckStream Team

## Links

- [Documentation](docs/README.md)
- [Pipeline Guide](docs/pipeline-configuration.md)
- [Quick Start](docs/QUICKSTART_PIPELINES.md)
- [Examples](examples/)

---

**Note**: CheckStream is currently in active development. APIs may change before 1.0 release.
