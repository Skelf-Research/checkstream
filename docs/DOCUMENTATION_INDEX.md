# CheckStream Documentation Index

**Complete guide to CheckStream documentation**

---

## 🚀 Quick Start

**New to CheckStream?** Start here:

1. **[README.md](../README.md)** - Project overview and quick start
2. **[Adding Models Guide](ADDING_MODELS_GUIDE.md)** - Add your first ML model in 2 minutes
3. **[examples/full_dynamic_pipeline.rs](../examples/full_dynamic_pipeline.rs)** - See it in action

---

## 📚 Core Documentation

### Getting Started

- **[README.md](../README.md)** - Main project documentation
  - Overview and philosophy
  - Key capabilities
  - Quick start guide
  - Building from source
  - Adding new models

### Architecture & Design

- **[Architecture](architecture.md)** - Technical design and components
  - Three-phase pipeline (Ingress, Midstream, Egress)
  - Streaming architecture
  - Component overview

- **[Design Principles](DESIGN_PRINCIPLES.md)** - Core philosophy
  - Provider agnosticism
  - Deployment agnosticism
  - Use case agnosticism
  - Configuration-driven design

- **[Deployment Modes](deployment-modes.md)** - How to deploy
  - Proxy mode (universal)
  - Sidecar mode (advanced)
  - Control plane (enterprise)

---

## ⚡ Dynamic Model Loading (NEW)

### Essential Reading

1. **[Adding Models Guide](ADDING_MODELS_GUIDE.md)** ⭐ **START HERE**
   - Step-by-step guide for adding models
   - Real-world examples
   - Troubleshooting
   - **Time to add a model**: 2 minutes

2. **[Dynamic Model Loading](DYNAMIC_MODEL_LOADING.md)** - Full specification
   - How it works
   - Configuration reference
   - Supported architectures
   - Preprocessing and output options

3. **[Model Loading Summary](MODEL_LOADING_SUMMARY.md)** - Quick reference
   - Current status
   - Before vs after comparison
   - When you need code
   - Roadmap

4. **[Vision Complete](VISION_COMPLETE.md)** - Achievement summary
   - What we built
   - Benefits achieved
   - Performance benchmarks
   - Next steps

### Related Documentation

- **[Agent Integration](AGENT_INTEGRATION.md)** - Using with agent frameworks
  - LangChain integration
  - AutoGen integration
  - Custom agents
  - Multi-step workflows

- **[models/README.md](../models/README.md)** - Model directory guide
  - Registry structure
  - Supported architectures
  - Examples
  - Cache management

---

## 🛡️ Safety & Compliance

### Classifier System

- **[Adversarial Robustness](adversarial-robustness.md)** - Security considerations
  - Classifier training
  - Evasion detection
  - Red teaming

- **[Pre-Production Validation](pre-production-validation.md)** - Testing and validation
  - Testing strategy
  - Risk assessment
  - Compliance sign-off

### Regulatory Compliance

- **[Regulatory Compliance](regulatory-compliance.md)** - Compliance frameworks
  - FCA Consumer Duty
  - FINRA
  - MiFID II
  - GDPR, HIPAA

- **[Use Cases](use-cases.md)** - Industry scenarios
  - Financial services
  - Healthcare
  - Legal services
  - Government

---

## 🔧 Implementation

### Configuration

- **[Policy Engine](policy-engine.md)** - Policy-as-code reference
  - Policy syntax
  - Rule definition
  - Action configuration

- **Pipeline Configuration** (docs/pipeline-configuration.md)
  - Pipeline types
  - Aggregation strategies
  - Conditional execution

### Integration

- **[API Reference](api-reference.md)** - REST API documentation
  - Endpoints
  - Request/response formats
  - Streaming protocol

- **[Security & Privacy](security-privacy.md)** - Data handling
  - Data residency
  - Audit model
  - Encryption

- **[Control Plane](control-plane.md)** - SaaS management
  - Fleet orchestration
  - Policy distribution
  - Telemetry

---

## 📖 Examples

### Working Code Examples

Located in `examples/` directory:

1. **[classifier_loading.rs](../examples/classifier_loading.rs)**
   - Load classifiers from config
   - Basic usage

2. **[pipeline_usage.rs](../examples/pipeline_usage.rs)**
   - Build custom pipelines
   - Parallel and sequential execution

3. **[streaming_context.rs](../examples/streaming_context.rs)**
   - Streaming buffer usage
   - Context window configuration

4. **[model_registry_usage.rs](../examples/model_registry_usage.rs)** ⚡ NEW
   - Parse model registry
   - List available models

5. **[dynamic_model_loading.rs](../examples/dynamic_model_loading.rs)** ⚡ NEW
   - Load models from YAML
   - Auto-download from HuggingFace

6. **[full_dynamic_pipeline.rs](../examples/full_dynamic_pipeline.rs)** ⚡ NEW ⭐
   - Complete example with mixed classifiers
   - Lazy loading and caching
   - Built-in + ML classifiers

7. **[test_ml_model.rs](../examples/test_ml_model.rs)**
   - Test ML model inference
   - Performance benchmarking

### Running Examples

```bash
# List all examples
cargo run --example

# Run specific example
cargo run --example full_dynamic_pipeline --features ml-models

# With logging
RUST_LOG=info cargo run --example dynamic_model_loading --features ml-models
```

---

## 🗂️ File Organization

```
checkstream/
├── README.md                           # Main documentation
├── docs/
│   ├── DOCUMENTATION_INDEX.md          # This file
│   ├── ADDING_MODELS_GUIDE.md          # ⭐ Quick start for models
│   ├── DYNAMIC_MODEL_LOADING.md        # Full specification
│   ├── MODEL_LOADING_SUMMARY.md        # Quick reference
│   ├── VISION_COMPLETE.md              # Achievement summary
│   ├── AGENT_INTEGRATION.md            # Agent framework integration
│   ├── DESIGN_PRINCIPLES.md            # Core philosophy
│   ├── architecture.md                 # System architecture
│   ├── deployment-modes.md             # Deployment options
│   ├── adversarial-robustness.md       # Security
│   ├── pre-production-validation.md    # Testing
│   ├── regulatory-compliance.md        # Compliance
│   ├── use-cases.md                    # Industry scenarios
│   ├── policy-engine.md                # Policy configuration
│   ├── api-reference.md                # API docs
│   ├── security-privacy.md             # Data handling
│   └── control-plane.md                # SaaS management
├── models/
│   ├── README.md                       # Model directory guide
│   └── registry.yaml                   # Model definitions
├── examples/
│   ├── classifier_loading.rs
│   ├── pipeline_usage.rs
│   ├── streaming_context.rs
│   ├── model_registry_usage.rs         # ⚡ NEW
│   ├── dynamic_model_loading.rs        # ⚡ NEW
│   ├── full_dynamic_pipeline.rs        # ⚡ NEW
│   └── test_ml_model.rs
└── scripts/
    ├── download_models.sh              # Download models
    └── build_tokenizer.py              # Build tokenizer.json
```

---

## 🎯 Documentation by Use Case

### I want to...

#### Add a new ML model
1. **[Adding Models Guide](ADDING_MODELS_GUIDE.md)** ⭐
2. **[models/registry.yaml](../models/registry.yaml)** - Examples
3. **[examples/dynamic_model_loading.rs](../examples/dynamic_model_loading.rs)** - Test it

#### Understand the architecture
1. **[Design Principles](DESIGN_PRINCIPLES.md)** - Philosophy
2. **[Architecture](architecture.md)** - Technical design
3. **[README.md](../README.md)** - Overview

#### Deploy to production
1. **[Deployment Modes](deployment-modes.md)** - Choose deployment
2. **[Security & Privacy](security-privacy.md)** - Security considerations
3. **[Pre-Production Validation](pre-production-validation.md)** - Testing

#### Integrate with my agent framework
1. **[Agent Integration](AGENT_INTEGRATION.md)** - Integration patterns
2. **[examples/full_dynamic_pipeline.rs](../examples/full_dynamic_pipeline.rs)** - Example
3. **[Design Principles](DESIGN_PRINCIPLES.md)** - Agnostic design

#### Meet regulatory requirements
1. **[Regulatory Compliance](regulatory-compliance.md)** - Compliance frameworks
2. **[Use Cases](use-cases.md)** - Industry examples
3. **[Policy Engine](policy-engine.md)** - Policy configuration

#### Build custom classifiers
1. **[Adding Models Guide](ADDING_MODELS_GUIDE.md)** - For standard models
2. **[Dynamic Model Loading](DYNAMIC_MODEL_LOADING.md)** - When you need code
3. **[examples/classifier_loading.rs](../examples/classifier_loading.rs)** - Example

---

## 📊 Documentation Status

| Category | Status | Notes |
|----------|--------|-------|
| **Quick Start** | ✅ Complete | README.md updated |
| **Dynamic Model Loading** | ✅ Complete | 4 new comprehensive docs |
| **Architecture** | ✅ Complete | Design principles documented |
| **Examples** | ✅ Complete | 7 working examples |
| **Model Registry** | ✅ Complete | YAML spec and examples |
| **Agent Integration** | ✅ Complete | Framework-specific guides |
| **Deployment** | ✅ Complete | Multiple modes documented |
| **Compliance** | ✅ Complete | FCA, FINRA, etc. |
| **API Reference** | 🚧 Planned | Coming soon |
| **Policy Engine** | 🚧 Planned | Coming soon |

---

## 🔄 Recently Updated

**November 2025** - Dynamic Model Loading Release

- ✅ README.md - Added dynamic model loading section
- ✅ New: ADDING_MODELS_GUIDE.md (400+ lines)
- ✅ New: DYNAMIC_MODEL_LOADING.md (400+ lines)
- ✅ New: MODEL_LOADING_SUMMARY.md (300+ lines)
- ✅ New: VISION_COMPLETE.md (300+ lines)
- ✅ New: models/README.md
- ✅ Updated: Roadmap with Phase 1.5 completion
- ✅ New: 3 working examples demonstrating dynamic loading

---

## 💡 Tips for Reading

### For New Users
1. Start with **README.md**
2. Try **Adding Models Guide** (2 min to add a model)
3. Run **full_dynamic_pipeline** example
4. Explore other examples

### For Developers
1. Read **Design Principles** (understand philosophy)
2. Study **Dynamic Model Loading** (full spec)
3. Review **Architecture** (technical details)
4. Check **examples/** for code patterns

### For Operators
1. Read **Deployment Modes** (choose deployment)
2. Study **Security & Privacy** (data handling)
3. Review **Pre-Production Validation** (testing)
4. Explore **Regulatory Compliance** (requirements)

---

## 🤝 Contributing to Documentation

Documentation improvements are welcome!

### Reporting Issues
- Unclear explanations
- Missing examples
- Outdated information

### Suggesting Improvements
- Additional examples
- Better explanations
- Missing use cases

Open an issue on GitHub or submit a PR.

---

## 📞 Getting Help

- **Examples**: See `examples/` directory
- **Issues**: GitHub Issues
- **Questions**: GitHub Discussions
- **Enterprise**: contact@checkstream.ai

---

**Last Updated**: November 2025
**Documentation Version**: 1.0.0 (Dynamic Model Loading Release)
