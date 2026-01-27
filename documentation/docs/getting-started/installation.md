# Installation

This guide covers installing CheckStream on your system.

---

## Prerequisites

- **Rust 1.70+** (for building from source)
- **Git** (for cloning the repository)
- **8GB+ RAM** recommended for ML models

---

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/Skelf-Research/checkstream
cd checkstream
```

### 2. Build with ML Support

For full ML classifier support (recommended):

```bash
cargo build --release --features ml-models
```

For pattern-only classifiers (smaller binary, faster build):

```bash
cargo build --release
```

### 3. Download Models

Download pre-trained models from HuggingFace:

```bash
./scripts/download_models.sh
```

This downloads:
- DistilBERT sentiment classifier
- Toxicity detection model
- Prompt injection detector

Models are cached in `~/.cache/huggingface/`.

---

## Docker Installation

Pull and run the official Docker image:

```bash
docker pull checkstream/checkstream:latest

docker run -d \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/app/config.yaml \
  -v $(pwd)/policies:/app/policies \
  checkstream/checkstream:latest
```

### Docker Compose

```yaml
version: '3.8'
services:
  checkstream:
    image: checkstream/checkstream:latest
    ports:
      - "8080:8080"
      - "9090:9090"  # Metrics
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./policies:/app/policies
    environment:
      - RUST_LOG=info
      - CHECKSTREAM_BACKEND_URL=https://api.openai.com/v1
```

---

## Verifying Installation

Check that CheckStream is running:

```bash
# Health check
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","version":"0.1.0"}
```

Check readiness (ensures classifiers are loaded):

```bash
curl http://localhost:8080/health/ready
```

---

## Configuration Files

CheckStream uses YAML configuration files:

| File | Purpose |
|------|---------|
| `config.yaml` | Main proxy configuration |
| `classifiers.yaml` | ML model definitions |
| `policies/*.yaml` | Safety policies |

See [Configuration](../configuration/proxy.md) for detailed options.

---

## Next Steps

- [Quick Start](quickstart.md) - Run your first request through CheckStream
- [Your First Policy](first-policy.md) - Create a custom safety policy
