# CheckStream Development Roadmap

**Last Updated**: 2025-11-13
**Current Version**: 0.1.0 (Development)

---

## ✅ Completed (Phase 1)

### Core Infrastructure
- [x] Rust workspace structure (5 crates)
- [x] Core types and utilities
- [x] Error handling system
- [x] Token buffer with holdback
- [x] Apache 2.0 license
- [x] Contributing guidelines
- [x] CI/CD pipelines (basic)

### Classifier System
- [x] Three-tier classifier architecture (A/B/C)
- [x] Candle-based ML inference
- [x] Model loading (HuggingFace Hub + local)
- [x] Device support (CPU/CUDA/Metal)
- [x] Quantization support
- [x] YAML configuration system
- [x] Pattern matching (Tier A)
- [x] PII detection (Tier A)
- [x] Toxicity classifier stubs (Tier B)

### Pipeline System ⭐ NEW
- [x] Pipeline orchestration engine
- [x] 4 stage types (Single, Parallel, Sequential, Conditional)
- [x] 6 aggregation strategies
- [x] YAML-based pipeline configuration
- [x] Pipeline builder from config
- [x] Streaming context windows (last N chunks or entire buffer)
- [x] Comprehensive documentation (2,500+ lines)
- [x] Working examples

### Documentation
- [x] Architecture overview
- [x] Deployment modes
- [x] Use cases
- [x] Regulatory compliance overview
- [x] Model loading guide
- [x] Pipeline configuration guide
- [x] FCA compliance example
- [x] Three-phase architecture diagrams
- [x] Streaming context guide
- [x] Integration guide

---

## 🔄 In Progress / Next Up (Phase 2)

### Priority 1: Core Proxy Implementation (Next 2-4 weeks)

The classifier system is complete, but needs to be integrated into the actual proxy.

#### 2.1 Basic Proxy Server
**Status**: Stub exists, needs implementation
**Files**: `crates/checkstream-proxy/src/*`

- [ ] Complete HTTP/SSE proxy server
- [ ] Request interceptor
- [ ] Response streaming handler
- [ ] Integration with classifier pipelines
- [ ] Phase 1 (Ingress) implementation
- [ ] Phase 2 (Midstream) implementation
- [ ] Phase 3 (Egress) implementation

**Deliverables**:
- Working proxy that intercepts OpenAI-compatible requests
- Executes pipelines at each phase
- Blocks/modifies/logs based on results

**Why This Is Next**: We have the engine, need to connect it to the car!

---

#### 2.2 Real Classifier Implementations
**Status**: Have infrastructure, need actual models

**Tier A (Pattern-based)**:
- [x] PII detection (basic)
- [ ] Enhanced PII (more formats)
- [ ] Profanity filter (word list)
- [ ] Simple regex patterns (URLs, IPs, etc.)

**Tier B (ML-based)**:
- [ ] Real toxicity classifier (load actual model)
- [ ] Sentiment analysis (working implementation)
- [ ] Prompt injection detector
- [ ] Jailbreak attempt detector

**Tier C (Advanced)**:
- [ ] Financial advice classifier
- [ ] Medical advice classifier
- [ ] Legal advice classifier
- [ ] Custom domain classifiers

**Deliverables**:
- At least 5-8 production-ready classifiers
- Model download scripts
- Performance benchmarks
- Accuracy metrics

**Why This Is Next**: Skeleton is there, need the actual ML models loaded and tested.

---

#### 2.3 Policy Engine
**Status**: Stub exists
**Files**: `crates/checkstream-policy/src/*`

- [ ] Policy definition language
- [ ] Rule evaluation engine
- [ ] Policy-to-pipeline mapping
- [ ] YAML policy configuration
- [ ] Policy versioning
- [ ] Policy hot-reload

**Example Policy**:
```yaml
policies:
  fca-consumer-duty:
    description: "UK FCA Consumer Duty compliance"
    rules:
      - name: financial-advice-detection
        trigger: ingress
        pipeline: fca-ingress-check
        action: modify_context
        threshold: 0.7

      - name: advice-blocking
        trigger: midstream
        pipeline: fca-midstream-check
        action: redact
        threshold: 0.8
```

**Deliverables**:
- Policy engine that maps rules to pipelines
- Library of pre-built policies (FCA, FINRA, GDPR, etc.)
- Policy testing framework

**Why This Is Next**: Makes CheckStream usable by non-technical users.

---

### Priority 2: Production Readiness (Weeks 4-8)

#### 2.4 Telemetry & Observability
**Status**: Stub exists
**Files**: `crates/checkstream-telemetry/src/*`

- [ ] Prometheus metrics integration
- [ ] Structured logging (tracing)
- [ ] Performance metrics (latency histograms)
- [ ] Decision metrics (pass/block/modify rates)
- [ ] Audit trail storage
- [ ] Grafana dashboards
- [ ] Alerting rules

**Key Metrics**:
```
checkstream_pipeline_latency_us{pipeline="basic-safety",phase="ingress"}
checkstream_decisions_total{action="block",pipeline="fca-midstream"}
checkstream_classifier_executions{classifier="toxicity",tier="B"}
checkstream_errors_total{phase="midstream",type="timeout"}
```

**Deliverables**:
- Full observability stack
- Pre-built Grafana dashboards
- Alert configurations
- Performance monitoring

---

#### 2.5 Testing & Benchmarking
**Status**: Unit tests exist, need integration tests

- [ ] Integration tests for proxy
- [ ] End-to-end tests with real LLMs
- [ ] Performance benchmarks
- [ ] Load testing
- [ ] Adversarial testing (jailbreak attempts)
- [ ] Compliance test suite

**Deliverables**:
- 80%+ test coverage
- Performance benchmarks proving <10ms targets
- Load test results (requests/sec)
- Adversarial robustness metrics

---

#### 2.6 Configuration Management
**Status**: YAML configs exist, need validation & tooling

- [ ] Configuration validation
- [ ] Config migration tools
- [ ] Default configurations for common use cases
- [ ] Config generator/wizard
- [ ] Hot-reload support
- [ ] Config versioning

**Deliverables**:
- Validated configs that catch errors at startup
- Library of pre-built configs
- CLI tool for config management

---

### Priority 3: Advanced Features (Weeks 8-12)

#### 2.7 Streaming Optimizations
**Status**: Context windows done, need more optimization

- [ ] Chunk batching (process N chunks at once)
- [ ] Adaptive context windows (expand if suspicious)
- [ ] Result caching (deduplicate similar chunks)
- [ ] Parallel chunk processing (multiple classifiers concurrently)
- [ ] Token-level streaming (not just chunks)

---

#### 2.8 Multi-Model Support
**Status**: Designed for extensibility

- [ ] Anthropic Claude support
- [ ] Google Gemini support
- [ ] AWS Bedrock support
- [ ] Azure OpenAI support
- [ ] Self-hosted models (vLLM, Ollama)

---

#### 2.9 Advanced Pipeline Features
**Status**: Core done, room for enhancement

- [ ] Pipeline composition (pipelines calling pipelines)
- [ ] Dynamic pipeline selection (choose based on request)
- [ ] Pipeline hot-reload (update without restart)
- [ ] Pipeline debugging tools
- [ ] Pipeline metrics per stage
- [ ] Circuit breakers (fail gracefully)
- [ ] Retry logic with exponential backoff

---

#### 2.10 Sidecar Mode (vLLM Integration)
**Status**: Designed but not implemented

- [ ] vLLM integration
- [ ] Logit masking
- [ ] Adaptive decoding (temperature adjustment)
- [ ] Token-level intervention
- [ ] Streaming optimization for sidecar

**Why Later**: More complex, needs proxy working first.

---

## 🔮 Future (Phase 3 - Q1 2025+)

### Control Plane (Enterprise)
- [ ] SaaS policy management
- [ ] Fleet orchestration
- [ ] Centralized telemetry
- [ ] Policy distribution
- [ ] A/B testing for policies

### ML Improvements
- [ ] ML-optimized scheduling (learn optimal order)
- [ ] Adaptive aggregation (adjust based on load)
- [ ] Model fine-tuning pipeline
- [ ] Continuous learning from false positives

### Compliance Features
- [ ] Cryptographic audit trail (blockchain/ledger)
- [ ] Regulatory report generation
- [ ] Compliance dashboards
- [ ] Evidence vault
- [ ] Multi-jurisdiction support

### Developer Experience
- [ ] Web UI for configuration
- [ ] Visual pipeline builder
- [ ] Real-time testing playground
- [ ] Documentation site
- [ ] Video tutorials

---

## 🎯 Recommended Next Steps

### This Week (Immediate)

1. **Get Proxy Working** (3-5 days)
   - Implement basic HTTP proxy
   - Add Phase 1 (ingress) integration
   - Add Phase 2 (midstream) with streaming context
   - Test with OpenAI API

2. **Load Real Models** (2-3 days)
   - Download toxicity model from HuggingFace
   - Test inference performance
   - Verify <5ms latency on CPU

3. **Basic Telemetry** (1-2 days)
   - Add Prometheus metrics
   - Log pipeline decisions
   - Create simple dashboard

### Next 2 Weeks

4. **Integration Testing**
   - End-to-end test with real LLM
   - Performance benchmarks
   - Load testing

5. **Policy Engine MVP**
   - Basic policy evaluation
   - 2-3 example policies (FCA, GDPR)

6. **Documentation**
   - Deployment guide
   - Operations runbook
   - Troubleshooting guide

### Next Month

7. **Production Readiness**
   - Complete test coverage
   - Performance optimization
   - Error handling hardening
   - Docker images
   - Helm charts

8. **More Classifiers**
   - 5+ production-ready classifiers
   - Benchmark suite
   - Model repository

---

## 📊 Success Metrics

### Technical Metrics
- **Latency**: 95th percentile < 10ms total pipeline
- **Throughput**: 1000+ requests/sec per instance
- **Availability**: 99.9% uptime
- **Test Coverage**: >80%
- **Memory**: <500MB per instance

### Business Metrics
- **False Positive Rate**: <5%
- **False Negative Rate**: <2%
- **Detection Accuracy**: >95%
- **Compliance Pass**: 100% (for implemented regulations)

---

## 🚫 Not on Roadmap (Explicitly Out of Scope)

- **Building LLMs**: CheckStream guards existing LLMs, doesn't create them
- **Fine-tuning Services**: We detect, not train
- **General-purpose Proxy**: Focused on safety/compliance, not performance routing
- **Chat UI**: Backend only, UIs are separate
- **Model Hosting**: We integrate with existing providers

---

## 🤝 How to Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

**High Priority Contributions Needed**:
1. Real classifier implementations (Tier B/C)
2. Policy examples for different regulations
3. Performance benchmarks
4. Integration tests
5. Documentation improvements

---

## 📝 Notes

- This roadmap is a living document
- Priorities may shift based on user feedback
- Dates are estimates, not commitments
- Focus is on production readiness before adding features

---

## 🔗 Related Documents

- [Architecture](docs/architecture.md)
- [Pipeline Configuration](docs/pipeline-configuration.md)
- [Integration Guide](docs/INTEGRATION_GUIDE.md)
- [Delivery Summary](DELIVERY_SUMMARY.md)
- [Changelog](CHANGELOG.md)

---

**Questions? Open an issue on GitHub.**
