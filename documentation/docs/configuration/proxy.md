# Proxy Configuration

Complete reference for CheckStream proxy configuration.

---

## Configuration File

CheckStream uses YAML configuration. Default location: `config.yaml`

```bash
./checkstream-proxy --config /path/to/config.yaml
```

---

## Full Configuration Reference

```yaml
# =============================================================================
# Server Configuration
# =============================================================================
server:
  host: "0.0.0.0"           # Bind address
  port: 8080                 # Main proxy port
  metrics_port: 9090         # Prometheus metrics port
  max_connections: 10000     # Maximum concurrent connections
  request_timeout_ms: 60000  # Request timeout
  shutdown_timeout_ms: 30000 # Graceful shutdown timeout

# =============================================================================
# Backend Configuration
# =============================================================================
backend:
  url: "https://api.openai.com/v1"  # LLM API endpoint
  timeout_ms: 30000                  # Backend request timeout
  retry_attempts: 3                  # Retry failed requests
  retry_delay_ms: 1000               # Delay between retries

  # Optional: Backend-specific headers
  headers:
    X-Custom-Header: "value"

# =============================================================================
# Pipeline Configuration
# =============================================================================
pipeline:
  # Ingress phase (pre-generation)
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
      - pii_detector
    threshold: 0.85           # Block if any classifier exceeds
    timeout_ms: 50            # Max time for ingress phase

  # Midstream phase (during streaming)
  midstream:
    enabled: true
    token_holdback: 16        # Tokens to buffer (8-32 recommended)
    context_chunks: 3         # Previous chunks for context (0 = all)
    classifiers:
      - toxicity
    chunk_threshold: 0.75     # Redact if exceeded
    timeout_ms: 10            # Max time per chunk

  # Egress phase (post-generation)
  egress:
    enabled: true
    audit: true               # Generate audit records
    classifiers:
      - compliance_check
    inject_disclaimers: true

# =============================================================================
# Threshold Configuration
# =============================================================================
thresholds:
  safety: 0.85      # Default safety threshold
  chunk: 0.75       # Default chunk threshold
  audit: 0.3        # Threshold for audit logging

# =============================================================================
# Policy Configuration
# =============================================================================
policy:
  path: "./policies/default.yaml"  # Policy file location
  reload_interval_s: 60            # Hot reload interval (0 = disabled)

# =============================================================================
# Streaming Configuration
# =============================================================================
streaming:
  format: "openai"            # openai, anthropic, custom
  chunk_delimiter: "\n\n"     # SSE chunk delimiter
  data_prefix: "data: "       # SSE data prefix

  # For custom formats
  custom:
    content_path: "$.choices[0].delta.content"
    finish_path: "$.choices[0].finish_reason"

# =============================================================================
# Telemetry Configuration
# =============================================================================
telemetry:
  metrics:
    enabled: true
    prefix: "checkstream"

  logging:
    level: "info"             # trace, debug, info, warn, error
    format: "json"            # json, pretty

  audit:
    enabled: true
    path: "./audit"
    rotation: "daily"
    retention_days: 90
    hash_chain: true          # Enable tamper-proof chain

# =============================================================================
# TLS Configuration (Optional)
# =============================================================================
tls:
  enabled: false
  cert_path: "/path/to/cert.pem"
  key_path: "/path/to/key.pem"

# =============================================================================
# Multi-Tenant Configuration (Optional)
# =============================================================================
tenants:
  enabled: false
  resolution:
    - header: "X-Tenant-ID"
    - path_prefix: "/tenant/"
    - api_key_mapping: true
  default_tenant: "default"
```

---

## Environment Variables

Configuration values can be overridden with environment variables:

| Variable | Config Path | Example |
|----------|-------------|---------|
| `CHECKSTREAM_HOST` | `server.host` | `0.0.0.0` |
| `CHECKSTREAM_PORT` | `server.port` | `8080` |
| `CHECKSTREAM_BACKEND_URL` | `backend.url` | `https://api.openai.com/v1` |
| `CHECKSTREAM_LOG_LEVEL` | `telemetry.logging.level` | `debug` |
| `CHECKSTREAM_POLICY_PATH` | `policy.path` | `./policies/prod.yaml` |

---

## Server Options

### Connection Limits

```yaml
server:
  max_connections: 10000      # Total connections
  max_connections_per_ip: 100 # Per-IP limit
  keepalive_timeout_ms: 60000 # HTTP keepalive
```

### Timeouts

```yaml
server:
  request_timeout_ms: 60000   # Total request time
  header_timeout_ms: 5000     # Time to receive headers
  body_timeout_ms: 30000      # Time to receive body
  shutdown_timeout_ms: 30000  # Graceful shutdown wait
```

---

## Backend Options

### Authentication Passthrough

```yaml
backend:
  url: "https://api.openai.com/v1"
  auth_passthrough: true      # Forward client's auth header
```

### Custom Authentication

```yaml
backend:
  url: "https://api.openai.com/v1"
  auth:
    type: bearer
    token_env: "OPENAI_API_KEY"  # Read from environment
```

### Multiple Backends (with Tenants)

```yaml
tenants:
  enabled: true
  configs:
    openai:
      backend_url: "https://api.openai.com/v1"
      policy_path: "./policies/openai.yaml"
    anthropic:
      backend_url: "https://api.anthropic.com"
      policy_path: "./policies/anthropic.yaml"
      streaming_format: "anthropic"
```

---

## Pipeline Options

### Classifier Selection

```yaml
pipeline:
  ingress:
    classifiers:
      - prompt_injection     # By name
      - type: pattern        # By type (all pattern classifiers)
      - tier: A              # By tier (all Tier A)
```

### Phase-Specific Thresholds

```yaml
pipeline:
  ingress:
    threshold: 0.9           # Strict for ingress
  midstream:
    chunk_threshold: 0.7     # More permissive for streaming
```

### Disabling Phases

```yaml
pipeline:
  ingress:
    enabled: true
  midstream:
    enabled: false           # Skip midstream checks
  egress:
    enabled: true
```

---

## Streaming Options

### OpenAI Format (Default)

```yaml
streaming:
  format: "openai"
  # Expects: data: {"choices":[{"delta":{"content":"..."}}]}
```

### Anthropic Format

```yaml
streaming:
  format: "anthropic"
  # Expects: event: content_block_delta
  #          data: {"delta":{"text":"..."}}
```

### Custom Format

```yaml
streaming:
  format: "custom"
  custom:
    content_path: "$.result.text"
    finish_path: "$.result.done"
    id_path: "$.id"
```

---

## Logging Options

### JSON Logging (Production)

```yaml
telemetry:
  logging:
    level: "info"
    format: "json"
    output: "stdout"
```

Output:
```json
{"timestamp":"2024-01-15T10:30:00Z","level":"INFO","message":"Request processed","request_id":"abc123","latency_ms":5}
```

### Pretty Logging (Development)

```yaml
telemetry:
  logging:
    level: "debug"
    format: "pretty"
```

Output:
```
2024-01-15 10:30:00 INFO  Request processed request_id=abc123 latency_ms=5
```

---

## Health Endpoints

Automatically enabled endpoints:

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Basic health check |
| `GET /health/live` | Kubernetes liveness probe |
| `GET /health/ready` | Kubernetes readiness probe |
| `GET /metrics` | Prometheus metrics |

---

## Configuration Validation

Validate configuration before starting:

```bash
./checkstream-proxy --config config.yaml --validate
```

---

## Hot Reload

Enable automatic policy reload:

```yaml
policy:
  path: "./policies/default.yaml"
  reload_interval_s: 60      # Check every 60 seconds
```

Or trigger manually:

```bash
curl -X POST http://localhost:8080/admin/reload
```

---

## Next Steps

- [Classifier Configuration](classifiers.md) - Configure ML models
- [Pipeline Configuration](pipelines.md) - Advanced pipeline setup
- [Deployment Guide](../deployment/docker.md) - Production deployment
