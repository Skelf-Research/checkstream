# CheckStream Quick Start Guide

Get CheckStream running in 5 minutes!

## Prerequisites

- Rust 1.75+ (`rustup update`)
- An OpenAI API key (or other LLM provider)

## Step 1: Build CheckStream

```bash
# Build the proxy
cargo build --release --package checkstream-proxy
```

This will take a few minutes on first build.

## Step 2: Run the Proxy

```bash
./target/release/checkstream-proxy --config config.yaml --verbose
```

You should see:
```
INFO Proxy listening on http://0.0.0.0:8080
```

## Step 3: Test It

```bash
# Health check
curl http://localhost:8080/health

# Test with OpenAI (requires API key)
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## What's Happening?

CheckStream is now:
1. **Phase 1 (Ingress)**: Validating prompts (~2-3ms)
2. **Phase 2 (Midstream)**: Checking streaming chunks (~1-2ms per chunk)
3. **Phase 3 (Egress)**: Running compliance checks (async)

All with sub-10ms latency!

## Next Steps

- Customize thresholds in `config.yaml`
- Add custom classifiers in `classifiers.yaml`
- See [full documentation](docs/README.md)
- Check the [proxy README](crates/checkstream-proxy/README.md)

## Learn More

- [Architecture](docs/architecture.md)
- [Pipeline Configuration](docs/pipeline-configuration.md)
- [Three-Phase System](docs/THREE_PHASE_DIAGRAM.md)
- [Roadmap](ROADMAP.md)
