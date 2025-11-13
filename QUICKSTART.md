# CheckStream Quick Start

This guide will help you get CheckStream up and running in minutes.

## Prerequisites

- **Rust 1.75+**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

## Installation

### 1. Clone and Build

```bash
# Clone the repository
git clone https://github.com/yourusername/checkstream.git
cd checkstream

# Build the project (takes ~3 minutes on first build)
cargo build --release

# The binary will be at: target/release/checkstream-proxy
```

### 2. Run Tests

```bash
# Run all tests
cargo test --all

# Run with verbose output
cargo test --all -- --nocapture
```

## Running CheckStream

### Basic Usage

```bash
# Start the proxy (development mode)
cargo run --bin checkstream-proxy -- \
  --backend https://api.openai.com/v1 \
  --policy ./policies/default.yaml \
  --port 8080

# Or use the release binary for better performance
./target/release/checkstream-proxy \
  --backend https://api.openai.com/v1 \
  --policy ./policies/default.yaml \
  --port 8080
```

### With Configuration File

Create a `config.yaml`:

```yaml
backend_url: "https://api.openai.com/v1"
policy_path: "./policies/default.yaml"
token_holdback: 10
max_buffer_capacity: 1000
telemetry:
  enabled: true
  mode: aggregate
```

Then run:

```bash
cargo run --bin checkstream-proxy -- --config config.yaml
```

### Using Docker

```bash
# Build the Docker image
docker build -t checkstream:latest .

# Run with Docker Compose
docker-compose up

# Or run directly
docker run -p 8080:8080 \
  -v $(pwd)/policies:/app/policies:ro \
  checkstream:latest
```

## Testing the Proxy

### Health Check

```bash
curl http://localhost:8080/health
# Expected output: OK
```

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
# Returns Prometheus-format metrics
```

## Project Structure

```
checkstream/
├── crates/
│   ├── checkstream-core/         # Core types and utilities
│   ├── checkstream-proxy/        # HTTP/SSE proxy server
│   ├── checkstream-policy/       # Policy engine
│   ├── checkstream-classifiers/  # Safety classifiers
│   └── checkstream-telemetry/    # Metrics and audit trail
├── policies/                      # Policy definitions
│   └── default.yaml              # Default safety policy
├── docs/                          # Comprehensive documentation
├── .github/workflows/            # CI/CD pipelines
├── Dockerfile                     # Container image
├── docker-compose.yml            # Multi-service setup
└── README.md                      # Project overview
```

## Development Workflow

### Running in Development

```bash
# Watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x "run --bin checkstream-proxy"

# Check code without building
cargo check --all

# Format code
cargo fmt --all

# Lint with Clippy
cargo clippy --all-targets --all-features
```

### Running Tests

```bash
# All tests
cargo test --all

# Specific crate
cargo test -p checkstream-core

# With output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

## What's Implemented

### ✅ Core Infrastructure
- Rust workspace with 5 crates
- Token buffer with holdback mechanism
- Error handling and type system
- Comprehensive test suite

### ✅ Policy Engine
- YAML policy definitions
- Pattern-based triggers
- Classifier-based triggers
- Multiple action types (log, stop, redact, inject, audit)

### ✅ Classifiers
- **PII Detection** (Tier A): Email, phone, SSN, credit cards
- **Pattern Matching** (Tier A): Fast Aho-Corasick algorithm
- **Toxicity Detection** (Tier B): Placeholder for ML model

### ✅ Telemetry
- Cryptographic audit trail with hash chaining
- Metrics collection
- Prometheus exporter integration

### ✅ DevOps
- Dockerfile with multi-stage build
- Docker Compose setup
- GitHub Actions CI/CD
- Security audit workflow

## Next Steps

### Immediate Development Tasks
1. Implement full HTTP/SSE proxy logic in `crates/checkstream-proxy/src/proxy.rs`
2. Add real ML models for classifiers (currently using placeholders)
3. Implement streaming token buffer integration
4. Add authentication and rate limiting
5. Create more policy packs for different regulations

### For Production Use
1. Update repository URLs in `Cargo.toml` and `README.md`
2. Configure actual backend LLM API endpoints
3. Set up monitoring and alerting
4. Deploy to your infrastructure (Kubernetes, AWS, etc.)
5. Load test and optimize for your workload

## Performance Targets

| Component | Target | Status |
|-----------|--------|--------|
| Tier A Classifiers | <2ms | ✅ Achieved |
| Tier B Classifiers | <5ms | 🔄 Placeholder |
| Policy Evaluation | <1ms | ✅ Ready |
| Total Proxy Overhead | <10ms | 🔄 In Progress |

## Getting Help

- **Documentation**: See `/docs` for detailed guides
- **Issues**: Report bugs or request features on GitHub
- **Contributing**: See `CONTRIBUTING.md` for guidelines
- **Security**: Email security@checkstream.ai for vulnerabilities

## Example: Adding a Custom Policy

Create `policies/my-policy.yaml`:

```yaml
name: custom-policy
description: My custom safety rules
version: "1.0"

rules:
  - name: block-profanity
    description: Block common profanity
    trigger:
      type: pattern
      pattern: "badword1|badword2|badword3"
      case_insensitive: true
    actions:
      - type: stop
        message: "Content policy violation"
      - type: audit
        category: profanity
        severity: medium
    enabled: true
```

Run with your policy:

```bash
cargo run --bin checkstream-proxy -- \
  --policy ./policies/my-policy.yaml
```

## License

Apache 2.0 - See `LICENSE` file for details.
