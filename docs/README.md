# CheckStream Documentation

Complete documentation for CheckStream - Real-time Safety and Compliance Layer for Streaming AI.

## 📚 Documentation Index

### Getting Started

- **[Quick Start Guide](getting-started.md)** - Get CheckStream running in 5 minutes
- **[Pipeline Quick Start](QUICKSTART_PIPELINES.md)** ⭐ NEW - Start using classifier pipelines
- **[Overview](overview.md)** - Platform overview and key concepts

### Core Concepts

- **[Architecture](architecture.md)** - System architecture and components
- **[Deployment Modes](deployment-modes.md)** - Proxy, Sidecar, and Control Plane modes
- **[Use Cases](use-cases.md)** - Real-world applications and scenarios

### Classifier System

- **[Pipeline Configuration](pipeline-configuration.md)** ⭐ NEW - Complete guide to classifier pipelines
  - Parallel execution
  - Sequential chaining
  - Conditional logic
  - Result aggregation
  - Performance optimization

- **[Classifier Configuration](classifier-configuration.md)** - Model configuration reference
  - Loading models from HuggingFace or local files
  - Device configuration (CPU/CUDA/Metal)
  - Quantization and optimization
  - Configuration best practices

- **[Model Loading](model-loading.md)** - Using Candle for ML inference
  - Model formats (SafeTensors, PyTorch)
  - Device management
  - Performance tuning
  - API reference

### Policy & Compliance

- **[Policy Engine](policy-engine.md)** - Policy-as-code system
- **[Regulatory Compliance](regulatory-compliance.md)** - FCA Consumer Duty, FINRA, MiFID II
- **[Security & Privacy](security-privacy.md)** - Data protection and security features

### Advanced Topics

- **[Control Plane](control-plane.md)** - Enterprise fleet management
- **[Adversarial Robustness](adversarial-robustness.md)** - Defense against attacks
- **[Pre-Production Validation](pre-production-validation.md)** - Testing and validation
- **[API Reference](api-reference.md)** - Complete API documentation
- **[Business Positioning](business-positioning.md)** - Market positioning and value prop

## 🎯 Quick Navigation

### I want to...

**Get started quickly**
→ [Getting Started](getting-started.md)

**Use classifier pipelines** ⭐ NEW
→ [Pipeline Quick Start](QUICKSTART_PIPELINES.md)
→ [Pipeline Configuration Guide](pipeline-configuration.md)

**Configure ML models**
→ [Classifier Configuration](classifier-configuration.md)
→ [Model Loading](model-loading.md)

**Understand the architecture**
→ [Architecture](architecture.md)
→ [Deployment Modes](deployment-modes.md)

**Implement compliance**
→ [Regulatory Compliance](regulatory-compliance.md)
→ [Policy Engine](policy-engine.md)

**Deploy in production**
→ [Control Plane](control-plane.md)
→ [Security & Privacy](security-privacy.md)

## 🆕 What's New

### Classifier Pipeline System (Latest)

CheckStream now supports sophisticated classifier workflows:

- **Parallel Execution** - Run multiple classifiers concurrently
- **Sequential Chaining** - Execute classifiers in order
- **Conditional Logic** - Run expensive checks only when needed
- **Result Aggregation** - 6 strategies for combining outputs

**Quick Example:**

```yaml
pipelines:
  safety-check:
    stages:
      - type: parallel
        name: multi-check
        classifiers: [toxicity, sentiment, pii]
        aggregation: max_score
```

See [Pipeline Quick Start](QUICKSTART_PIPELINES.md) to get started.

### Candle ML Inference

Native Rust ML inference using Hugging Face Candle:

- Load models from HuggingFace Hub or local files
- Support for SafeTensors and PyTorch formats
- CPU, CUDA, and Metal device support
- Quantization for 1.5-2x speedup

See [Model Loading](model-loading.md) for details.

## 📖 Documentation Organization

```
docs/
├── README.md                          # This file
│
├── Getting Started
│   ├── getting-started.md             # Quick start guide
│   ├── QUICKSTART_PIPELINES.md        # Pipeline quick start ⭐ NEW
│   └── overview.md                    # Platform overview
│
├── Classifier System                  ⭐ NEW SECTION
│   ├── pipeline-configuration.md      # Complete pipeline guide
│   ├── classifier-configuration.md    # Model configuration
│   └── model-loading.md               # Candle ML inference
│
├── Core Concepts
│   ├── architecture.md                # System architecture
│   ├── deployment-modes.md            # Proxy/Sidecar/Control Plane
│   └── use-cases.md                   # Real-world scenarios
│
├── Policy & Compliance
│   ├── policy-engine.md               # Policy-as-code
│   ├── regulatory-compliance.md       # Regulations support
│   └── security-privacy.md            # Security features
│
└── Advanced
    ├── control-plane.md               # Fleet management
    ├── adversarial-robustness.md      # Attack defense
    ├── pre-production-validation.md   # Testing
    ├── api-reference.md               # API docs
    └── business-positioning.md        # Market positioning
```

## 🔗 External Resources

- **GitHub**: https://github.com/yourusername/checkstream
- **Examples**: [`../examples/`](../examples/)
- **Configuration**: [`../classifiers.yaml`](../classifiers.yaml)
- **Contributing**: [`../CONTRIBUTING.md`](../CONTRIBUTING.md)

## 💡 Examples

Working code examples are available in the `examples/` directory:

- **`pipeline_usage.rs`** - Classifier pipeline demonstration ⭐ NEW
- **`classifier_loading.rs`** - Model loading examples

Run examples:
```bash
cargo run --example pipeline_usage
```

## 🤝 Contributing

Found an error? Want to improve the docs?

1. Fork the repository
2. Edit the documentation
3. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## 📋 Documentation TODO

- [ ] Add more pipeline examples for different use cases
- [ ] Create video walkthrough of pipeline system
- [ ] Add benchmarking guide for latency optimization
- [ ] Expand API reference with more examples
- [ ] Add troubleshooting guide for common issues

## 📄 License

CheckStream is licensed under the Apache License 2.0. See [LICENSE](../LICENSE) for details.

---

**Last Updated**: 2025-11-13
**Version**: 0.1.0 (Development)
