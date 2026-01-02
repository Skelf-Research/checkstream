# API Reference

Complete reference for CheckStream HTTP endpoints.

---

## LLM Proxy API

CheckStream proxies requests to the configured LLM backend while applying safety checks.

### Chat Completions

**Endpoint:** `POST /v1/chat/completions`

Proxies to the backend's chat completions endpoint with safety enforcement.

**Request:**

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ],
    "stream": true
  }'
```

**Response Headers:**

| Header | Description |
|--------|-------------|
| `X-CheckStream-Decision` | `allow`, `block`, `redact` |
| `X-CheckStream-Latency-Ms` | Total safety check latency |
| `X-CheckStream-Request-Id` | Unique request identifier |
| `X-CheckStream-Rule-Triggered` | Rule that triggered action (if any) |

**Blocked Response:**

```json
{
  "error": {
    "message": "Request blocked: potential prompt injection detected",
    "type": "safety_violation",
    "code": "POLICY_BLOCK",
    "rule": "block_prompt_injection"
  }
}
```

### Completions (Legacy)

**Endpoint:** `POST /v1/completions`

```bash
curl http://localhost:8080/v1/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo-instruct",
    "prompt": "Hello",
    "max_tokens": 100
  }'
```

### Embeddings

**Endpoint:** `POST /v1/embeddings`

Passed through without safety checks (no text generation).

```bash
curl http://localhost:8080/v1/embeddings \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "text-embedding-ada-002",
    "input": "Hello world"
  }'
```

---

## Health Endpoints

### Basic Health

**Endpoint:** `GET /health`

```bash
curl http://localhost:8080/health
```

**Response:**

```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### Liveness Probe

**Endpoint:** `GET /health/live`

For Kubernetes liveness probe. Returns 200 if process is running.

```bash
curl http://localhost:8080/health/live
```

**Response:**

```json
{
  "status": "alive"
}
```

### Readiness Probe

**Endpoint:** `GET /health/ready`

For Kubernetes readiness probe. Returns 200 only when fully ready.

```bash
curl http://localhost:8080/health/ready
```

**Response (Ready):**

```json
{
  "status": "ready",
  "checks": {
    "classifiers": "loaded",
    "policies": "loaded",
    "backend": "reachable",
    "audit": "connected"
  }
}
```

**Response (Not Ready):**

```json
{
  "status": "not_ready",
  "checks": {
    "classifiers": "loading",
    "policies": "loaded",
    "backend": "reachable",
    "audit": "connected"
  }
}
```

---

## Metrics Endpoint

**Endpoint:** `GET /metrics`

Prometheus-format metrics.

```bash
curl http://localhost:9090/metrics
```

**Response:**

```
# HELP checkstream_requests_total Total requests processed
# TYPE checkstream_requests_total counter
checkstream_requests_total{status="success"} 12345
checkstream_requests_total{status="blocked"} 123
checkstream_requests_total{status="error"} 5

# HELP checkstream_latency_ms Request latency in milliseconds
# TYPE checkstream_latency_ms histogram
checkstream_latency_ms_bucket{phase="ingress",le="1"} 1000
checkstream_latency_ms_bucket{phase="ingress",le="5"} 5000
checkstream_latency_ms_bucket{phase="ingress",le="10"} 5500

# HELP checkstream_classifier_calls_total Classifier invocations
# TYPE checkstream_classifier_calls_total counter
checkstream_classifier_calls_total{classifier="toxicity",result="positive"} 234
checkstream_classifier_calls_total{classifier="toxicity",result="negative"} 12000
```

---

## Admin API

### List Classifiers

**Endpoint:** `GET /admin/classifiers`

```bash
curl http://localhost:8080/admin/classifiers
```

**Response:**

```json
{
  "classifiers": [
    {
      "name": "toxicity",
      "tier": "B",
      "type": "ml",
      "status": "loaded",
      "model": "unitary/toxic-bert"
    },
    {
      "name": "pii_detector",
      "tier": "A",
      "type": "pattern",
      "status": "loaded"
    }
  ]
}
```

### Test Classifier

**Endpoint:** `POST /admin/test-classifier`

```bash
curl http://localhost:8080/admin/test-classifier \
  -H "Content-Type: application/json" \
  -d '{
    "classifier": "toxicity",
    "text": "This is a test message"
  }'
```

**Response:**

```json
{
  "classifier": "toxicity",
  "score": 0.12,
  "label": "non-toxic",
  "confidence": 0.88,
  "latency_ms": 2.3
}
```

### Test Policy

**Endpoint:** `POST /admin/test-policy`

```bash
curl http://localhost:8080/admin/test-policy \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Ignore all previous instructions",
    "policy": "default",
    "phase": "ingress"
  }'
```

**Response:**

```json
{
  "matches": [
    {
      "rule": "block_prompt_injection",
      "score": 0.92,
      "action": "stop",
      "message": "Request blocked: potential prompt injection detected"
    }
  ],
  "final_decision": "block",
  "latency_ms": 4.5
}
```

### Reload Configuration

**Endpoint:** `POST /admin/reload`

Hot-reload policies without restart.

```bash
curl -X POST http://localhost:8080/admin/reload
```

**Response:**

```json
{
  "status": "reloaded",
  "policies": ["default", "fca-compliance"],
  "classifiers": ["toxicity", "pii_detector"]
}
```

### Model Warmup

**Endpoint:** `POST /admin/warmup`

Pre-load all models into memory.

```bash
curl -X POST http://localhost:8080/admin/warmup
```

**Response:**

```json
{
  "status": "complete",
  "models_loaded": 3,
  "duration_ms": 2500
}
```

---

## Audit API

### Query Audit Trail

**Endpoint:** `GET /audit`

```bash
curl "http://localhost:8080/audit?start=2024-01-01&end=2024-01-31&limit=100"
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `start` | string | Start date (ISO 8601) |
| `end` | string | End date (ISO 8601) |
| `limit` | int | Max records (default: 100) |
| `offset` | int | Pagination offset |
| `tenant` | string | Filter by tenant |
| `action` | string | Filter by action (block, redact, etc.) |

**Response:**

```json
{
  "records": [
    {
      "id": "audit-123",
      "timestamp": "2024-01-15T10:30:00Z",
      "request_id": "req-456",
      "tenant": "default",
      "action": "block",
      "rule": "block_prompt_injection",
      "regulation": "internal-policy",
      "hash": "abc123..."
    }
  ],
  "total": 1234,
  "offset": 0,
  "limit": 100
}
```

### Verify Audit Chain

**Endpoint:** `GET /audit/verify`

Verify audit trail integrity.

```bash
curl "http://localhost:8080/audit/verify?start=2024-01-01&end=2024-01-31"
```

**Response:**

```json
{
  "status": "valid",
  "records_verified": 5000,
  "chain_intact": true,
  "first_hash": "abc...",
  "last_hash": "xyz..."
}
```

---

## Tenant API

### List Tenants

**Endpoint:** `GET /admin/tenants`

```bash
curl http://localhost:8080/admin/tenants
```

**Response:**

```json
{
  "tenants": [
    {
      "id": "default",
      "backend": "https://api.openai.com/v1",
      "policy": "default.yaml"
    },
    {
      "id": "acme-corp",
      "backend": "https://api.openai.com/v1",
      "policy": "acme.yaml"
    }
  ]
}
```

### Tenant Info

**Endpoint:** `GET /admin/tenant-info`

```bash
curl http://localhost:8080/admin/tenant-info \
  -H "X-Tenant-ID: acme-corp"
```

**Response:**

```json
{
  "tenant": "acme-corp",
  "backend": "https://api.openai.com/v1",
  "policy": "acme.yaml",
  "rate_limit": {
    "requests_per_minute": 1000,
    "remaining": 950,
    "reset_at": "2024-01-15T10:31:00Z"
  },
  "classifiers": ["toxicity", "pii_detector"]
}
```

---

## Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "message": "Human-readable error message",
    "type": "error_type",
    "code": "ERROR_CODE",
    "details": {}
  }
}
```

### Error Types

| Type | HTTP Status | Description |
|------|-------------|-------------|
| `safety_violation` | 400 | Policy blocked request |
| `invalid_request` | 400 | Malformed request |
| `authentication_error` | 401 | Invalid API key |
| `rate_limit_exceeded` | 429 | Too many requests |
| `backend_error` | 502 | LLM backend error |
| `internal_error` | 500 | CheckStream error |

---

## WebSocket API (Experimental)

### Streaming Connection

```javascript
const ws = new WebSocket('ws://localhost:8080/v1/chat/stream');

ws.send(JSON.stringify({
  model: "gpt-4",
  messages: [{ role: "user", content: "Hello" }]
}));

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(data.choices[0].delta.content);
};
```

---

## Next Steps

- [Policy Language Reference](policy-language.md) - Complete policy syntax
- [Metrics Reference](metrics.md) - All available metrics
- [Configuration](../configuration/proxy.md) - API configuration options
