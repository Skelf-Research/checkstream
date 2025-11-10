# API Reference

CheckStream provides multiple APIs for different use cases:
- **LLM Proxy API**: OpenAI/Anthropic-compatible endpoints (for client applications)
- **Control Plane API**: Management and orchestration (for administrators)
- **Admin API**: Node configuration and monitoring (for operations)
- **Metrics API**: Prometheus-compatible metrics (for observability)

---

## Base URLs

```
# LLM Proxy API (runs on nodes)
http://localhost:8080/v1

# Control Plane API (SaaS)
https://control.checkstream.ai/v1

# Admin API (runs on nodes)
http://localhost:8080/admin

# Metrics API (runs on nodes)
http://localhost:8080/metrics
```

---

## Authentication

### LLM Proxy API

Uses your LLM provider's API key (pass-through):

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{...}'
```

### Control Plane API

Uses CheckStream API key or SSO token:

```bash
curl https://control.checkstream.ai/v1/policies \
  -H "Authorization: Bearer cs_key_abc123..."
```

### Admin API

Optional authentication (configure in node settings):

```bash
curl http://localhost:8080/admin/health \
  -H "X-Admin-Token: ${ADMIN_TOKEN}"
```

---

## LLM Proxy API

### POST /v1/chat/completions

OpenAI-compatible chat completions.

**Request**:
```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Tell me about investing"}
    ],
    "stream": true,
    "temperature": 0.7
  }'
```

**Response** (streaming):
```
data: {"id":"chatcmpl-abc","choices":[{"delta":{"role":"assistant"},"index":0}],"created":1234567890}

data: {"id":"chatcmpl-abc","choices":[{"delta":{"content":"Investing"},"index":0}],"created":1234567890}

data: {"id":"chatcmpl-abc","choices":[{"delta":{"content":" involves"},"index":0}],"created":1234567890}

...

data: [DONE]
```

**Guardrail Headers** (optional, in response):
```
X-CheckStream-Decision: allow
X-CheckStream-Rule-Triggered: none
X-CheckStream-Latency-Ms: 7.3
X-CheckStream-Policy-Version: v2.3.1
```

### POST /v1/messages (Anthropic-compatible)

Anthropic Claude API format.

**Request**:
```bash
curl http://localhost:8080/v1/messages \
  -H "x-api-key: ${ANTHROPIC_API_KEY}" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Explain quantum computing"}
    ],
    "stream": true
  }'
```

**Response** (streaming):
```
event: message_start
data: {"type":"message_start","message":{"id":"msg_abc"}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"text":"Quantum"}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"text":" computing"}}

...

event: message_stop
data: {"type":"message_stop"}
```

### POST /v1/completions

Legacy OpenAI completions (text, not chat).

**Request**:
```bash
curl http://localhost:8080/v1/completions \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo-instruct",
    "prompt": "Once upon a time",
    "max_tokens": 100,
    "stream": true
  }'
```

---

## Control Plane API

### Policies

#### POST /v1/policies

Create a new policy.

**Request**:
```bash
curl -X POST https://control.checkstream.ai/v1/policies \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/yaml" \
  --data-binary @policy.yaml
```

**Response**:
```json
{
  "policy_id": "pol_abc123",
  "name": "consumer_duty_v2",
  "version": "2.3.1",
  "created_at": "2024-01-15T10:00:00Z",
  "created_by": "user_xyz789",
  "status": "draft"
}
```

#### GET /v1/policies

List all policies.

**Request**:
```bash
curl https://control.checkstream.ai/v1/policies \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "policies": [
    {
      "policy_id": "pol_abc123",
      "name": "consumer_duty_v2",
      "version": "2.3.1",
      "enabled": true,
      "mode": "enforce",
      "rules_count": 12,
      "last_updated": "2024-01-15T10:00:00Z"
    },
    {
      "policy_id": "pol_def456",
      "name": "prompt_injection_defense",
      "version": "1.5.0",
      "enabled": true,
      "mode": "enforce",
      "rules_count": 8,
      "last_updated": "2024-01-10T08:30:00Z"
    }
  ],
  "total": 2
}
```

#### GET /v1/policies/{policy_id}

Get policy details.

**Request**:
```bash
curl https://control.checkstream.ai/v1/policies/pol_abc123 \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "policy_id": "pol_abc123",
  "name": "consumer_duty_v2",
  "version": "2.3.1",
  "enabled": true,
  "mode": "enforce",
  "metadata": {
    "regulation": "FCA PRIN 2A",
    "severity": "high"
  },
  "rules": [
    {
      "trigger": {
        "classifier": "promotional_balance",
        "threshold": 0.75
      },
      "action": "inject_disclaimer"
    }
  ],
  "created_at": "2024-01-15T10:00:00Z",
  "created_by": "user_xyz789",
  "git_commit": "abc123def456"
}
```

#### PUT /v1/policies/{policy_id}

Update a policy.

**Request**:
```bash
curl -X PUT https://control.checkstream.ai/v1/policies/pol_abc123 \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/yaml" \
  --data-binary @updated-policy.yaml
```

**Response**:
```json
{
  "policy_id": "pol_abc123",
  "version": "2.3.2",
  "status": "pending_approval",
  "approval_required": true,
  "approvers": ["risk_officer", "chief_risk_officer"]
}
```

#### DELETE /v1/policies/{policy_id}

Delete a policy.

**Request**:
```bash
curl -X DELETE https://control.checkstream.ai/v1/policies/pol_abc123 \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "deleted": true,
  "policy_id": "pol_abc123"
}
```

#### POST /v1/policies/{policy_id}/deploy

Deploy policy to fleet.

**Request**:
```bash
curl -X POST https://control.checkstream.ai/v1/policies/pol_abc123/deploy \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "fleets": ["production-eu-west-2"],
    "canary_percent": 10,
    "wait_for_health": true
  }'
```

**Response**:
```json
{
  "deployment_id": "dep_xyz789",
  "status": "in_progress",
  "fleets": ["production-eu-west-2"],
  "canary_percent": 10,
  "nodes_targeted": 12,
  "nodes_updated": 1,
  "started_at": "2024-01-15T11:00:00Z"
}
```

### Models

#### GET /v1/models

List available classifier models.

**Request**:
```bash
curl https://control.checkstream.ai/v1/models \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "models": [
    {
      "model_id": "toxicity-detector-v2.1",
      "name": "Toxicity Detector",
      "version": "2.1.0",
      "type": "classifier",
      "languages": ["en", "es", "fr", "de"],
      "latency_p95_ms": 6.2,
      "accuracy": 0.94,
      "size_mb": 45
    },
    {
      "model_id": "advice-vs-info-v1.5",
      "name": "Financial Advice Classifier",
      "version": "1.5.0",
      "type": "classifier",
      "languages": ["en"],
      "latency_p95_ms": 7.8,
      "accuracy": 0.91,
      "size_mb": 62
    }
  ]
}
```

#### GET /v1/models/{model_id}

Get model metadata and download URL.

**Request**:
```bash
curl https://control.checkstream.ai/v1/models/toxicity-detector-v2.1 \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "model_id": "toxicity-detector-v2.1",
  "version": "2.1.0",
  "framework": "onnx",
  "precision": "int8",
  "cdn_urls": {
    "eu-west-2": "https://models-eu-west.checkstream.ai/toxicity-v2.1.onnx"
  },
  "checksum": "sha256:abc123def456...",
  "signature": "-----BEGIN PGP SIGNATURE-----..."
}
```

### Fleets

#### GET /v1/fleets

List all fleets.

**Request**:
```bash
curl https://control.checkstream.ai/v1/fleets \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "fleets": [
    {
      "fleet_id": "flt_prod_eu",
      "name": "production-eu-west-2",
      "region": "eu-west-2",
      "nodes": 12,
      "healthy_nodes": 11,
      "policy_version": "v2.3.1",
      "last_updated": "2024-01-15T10:00:00Z"
    }
  ]
}
```

#### GET /v1/fleets/{fleet_id}

Get fleet details.

**Request**:
```bash
curl https://control.checkstream.ai/v1/fleets/flt_prod_eu \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "fleet_id": "flt_prod_eu",
  "name": "production-eu-west-2",
  "region": "eu-west-2",
  "nodes": [
    {
      "node_id": "node_abc123",
      "type": "proxy",
      "version": "1.3.0",
      "policy_version": "v2.3.1",
      "status": "healthy",
      "last_heartbeat": "2024-01-15T11:05:00Z",
      "metrics": {
        "requests_per_sec": 23.4,
        "latency_p95_ms": 8.1,
        "cpu_percent": 34
      }
    }
  ]
}
```

#### PUT /v1/fleets/{fleet_id}/desired-state

Update fleet desired state.

**Request**:
```bash
curl -X PUT https://control.checkstream.ai/v1/fleets/flt_prod_eu/desired-state \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "policy_bundle": "consumer-duty-v2.3.2",
    "models": {
      "toxicity": "v2.1.0",
      "advice_vs_info": "v1.5.0"
    }
  }'
```

**Response**:
```json
{
  "fleet_id": "flt_prod_eu",
  "desired_state_updated": true,
  "rollout_started": true,
  "estimated_completion": "2024-01-15T11:15:00Z"
}
```

### Telemetry

#### POST /v1/telemetry/ingest

Ingest telemetry from nodes (internal use).

**Request** (from node):
```bash
curl -X POST https://telemetry-eu.checkstream.ai/v1/telemetry/ingest \
  -H "Authorization: Bearer node_token_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "node_abc123",
    "interval": "2024-01-15T10:00:00Z/PT1M",
    "metrics": {...}
  }'
```

#### GET /v1/telemetry/query

Query telemetry data.

**Request**:
```bash
curl "https://control.checkstream.ai/v1/telemetry/query?start=2024-01-15T00:00:00Z&end=2024-01-15T23:59:59Z&fleet=flt_prod_eu&metric=decisions" \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "metric": "decisions",
  "data": [
    {
      "timestamp": "2024-01-15T10:00:00Z",
      "allow": 1489,
      "redact": 28,
      "stop": 6
    },
    {
      "timestamp": "2024-01-15T10:01:00Z",
      "allow": 1502,
      "redact": 31,
      "stop": 4
    }
  ]
}
```

### Audit

#### GET /v1/audit/events

Query audit log.

**Request**:
```bash
curl "https://control.checkstream.ai/v1/audit/events?start=2024-01-01&end=2024-01-31&event_type=policy_deployed" \
  -H "Authorization: Bearer cs_key_abc123"
```

**Response**:
```json
{
  "events": [
    {
      "event_id": "evt_abc123",
      "timestamp": "2024-01-15T14:30:00Z",
      "event_type": "policy_deployed",
      "actor": "risk_officer@acme-bank.com",
      "resource": "policy/consumer-duty-v2.3.1",
      "action": "deploy",
      "outcome": "success"
    }
  ],
  "total": 1,
  "integrity_verified": true
}
```

#### POST /v1/audit/export

Export audit report.

**Request**:
```bash
curl -X POST https://control.checkstream.ai/v1/audit/export \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2024-01-01",
    "end_date": "2024-03-31",
    "format": "pdf",
    "include": ["policy_changes", "deployment_events"]
  }' \
  --output Q1_2024_audit.pdf
```

---

## Admin API (Node)

### GET /admin/health

Health check endpoint.

**Request**:
```bash
curl http://localhost:8080/admin/health
```

**Response**:
```json
{
  "status": "healthy",
  "version": "1.3.0",
  "uptime_seconds": 86400,
  "policy_version": "v2.3.1",
  "models_loaded": 3,
  "backend_reachable": true
}
```

### GET /admin/ready

Readiness probe (Kubernetes).

**Request**:
```bash
curl http://localhost:8080/admin/ready
```

**Response**:
```
200 OK (if ready)
503 Service Unavailable (if not ready)
```

### GET /admin/live

Liveness probe (Kubernetes).

**Request**:
```bash
curl http://localhost:8080/admin/live
```

**Response**:
```
200 OK (if alive)
```

### POST /admin/reload-policies

Hot-reload policies.

**Request**:
```bash
curl -X POST http://localhost:8080/admin/reload-policies \
  -H "X-Admin-Token: ${ADMIN_TOKEN}"
```

**Response**:
```json
{
  "reloaded": true,
  "previous_version": "v2.3.0",
  "new_version": "v2.3.1",
  "reload_time_ms": 234
}
```

### GET /admin/policies

List currently loaded policies.

**Request**:
```bash
curl http://localhost:8080/admin/policies
```

**Response**:
```json
{
  "policies": [
    {
      "name": "consumer_duty",
      "version": "v2.3.1",
      "enabled": true,
      "mode": "enforce",
      "rules": 12
    }
  ]
}
```

### GET /admin/logs

Stream recent logs (last 100 by default).

**Request**:
```bash
curl http://localhost:8080/admin/logs?limit=50
```

**Response** (JSON lines):
```
{"timestamp":"2024-01-15T10:00:00Z","level":"info","message":"Policy reloaded"}
{"timestamp":"2024-01-15T10:01:23Z","level":"info","stream_id":"req_abc","decision":"allow"}
```

---

## Metrics API (Prometheus)

### GET /metrics

Prometheus metrics endpoint.

**Request**:
```bash
curl http://localhost:8080/metrics
```

**Response** (Prometheus format):
```
# HELP checkstream_requests_total Total number of requests processed
# TYPE checkstream_requests_total counter
checkstream_requests_total{status="allowed"} 12345
checkstream_requests_total{status="redacted"} 234
checkstream_requests_total{status="blocked"} 56

# HELP checkstream_latency_ms Request latency in milliseconds
# TYPE checkstream_latency_ms histogram
checkstream_latency_ms_bucket{stage="ingress",le="5"} 890
checkstream_latency_ms_bucket{stage="ingress",le="10"} 980
checkstream_latency_ms_bucket{stage="ingress",le="+Inf"} 1000
checkstream_latency_ms_sum{stage="ingress"} 6234
checkstream_latency_ms_count{stage="ingress"} 1000

# HELP checkstream_policy_triggers_total Policy rule trigger counts
# TYPE checkstream_policy_triggers_total counter
checkstream_policy_triggers_total{rule="promotional_balance"} 234
checkstream_policy_triggers_total{rule="suitability_check"} 89

# HELP checkstream_classifier_latency_ms Classifier inference time
# TYPE checkstream_classifier_latency_ms histogram
checkstream_classifier_latency_ms_bucket{classifier="toxicity",le="5"} 450
checkstream_classifier_latency_ms_bucket{classifier="toxicity",le="10"} 490
```

---

## SDK Examples

### Python

```python
from checkstream import CheckStreamClient

# Initialize client
client = CheckStreamClient(
    control_plane="https://control.checkstream.ai",
    api_key="cs_key_abc123"
)

# Create policy
policy = client.policies.create(
    name="my_custom_policy",
    yaml_file="./policy.yaml"
)

# Deploy to fleet
deployment = client.policies.deploy(
    policy_id=policy.id,
    fleets=["production-eu-west-2"],
    canary_percent=10
)

# Query telemetry
metrics = client.telemetry.query(
    start="2024-01-15T00:00:00Z",
    end="2024-01-15T23:59:59Z",
    metric="decisions"
)

print(f"Total requests: {sum(m['allow'] + m['redact'] + m['stop'] for m in metrics.data)}")
```

### JavaScript/TypeScript

```typescript
import { CheckStreamClient } from '@checkstream/sdk';

const client = new CheckStreamClient({
  controlPlane: 'https://control.checkstream.ai',
  apiKey: 'cs_key_abc123'
});

// List policies
const policies = await client.policies.list();

// Get fleet status
const fleet = await client.fleets.get('production-eu-west-2');
console.log(`Fleet health: ${fleet.healthy_nodes}/${fleet.nodes} nodes healthy`);

// Export audit report
const report = await client.audit.export({
  startDate: '2024-01-01',
  endDate: '2024-03-31',
  format: 'pdf'
});
```

---

## Error Codes

| Code | Status | Description |
|------|--------|-------------|
| `auth_invalid` | 401 | Invalid API key or token |
| `forbidden` | 403 | Insufficient permissions |
| `not_found` | 404 | Resource not found |
| `validation_error` | 400 | Invalid request parameters |
| `rate_limit_exceeded` | 429 | Too many requests |
| `internal_error` | 500 | Server error |
| `backend_unavailable` | 502 | LLM backend unreachable |
| `timeout` | 504 | Request timeout |

**Example Error Response**:
```json
{
  "error": {
    "code": "validation_error",
    "message": "Invalid policy syntax: missing 'trigger' field in rule 3",
    "details": {
      "line": 42,
      "field": "rules[2].trigger"
    }
  }
}
```

---

## Rate Limits

| Endpoint | Limit | Scope |
|----------|-------|-------|
| **LLM Proxy** | Unlimited | (backend limits apply) |
| **Control Plane API** | 1000 req/min | Per API key |
| **Telemetry Ingest** | 10,000 req/min | Per node |
| **Audit Export** | 10 req/hour | Per organization |

**Rate Limit Headers**:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 742
X-RateLimit-Reset: 1705329600
```

---

## Webhooks

Configure webhooks for real-time notifications.

### Setup

```bash
curl -X POST https://control.checkstream.ai/v1/webhooks \
  -H "Authorization: Bearer cs_key_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-app.com/webhooks/checkstream",
    "events": ["policy_deployed", "high_risk_spike", "node_unhealthy"],
    "secret": "whsec_abc123..."
  }'
```

### Webhook Payload

```json
{
  "event_id": "evt_abc123",
  "event_type": "high_risk_spike",
  "timestamp": "2024-01-15T11:00:00Z",
  "data": {
    "fleet": "production-eu-west-2",
    "rule": "suitability_check",
    "spike_factor": 5.2,
    "threshold": 3.0
  },
  "signature": "sha256=..."
}
```

### Verify Signature

```python
import hmac
import hashlib

def verify_webhook(payload, signature, secret):
    expected = hmac.new(
        secret.encode(),
        payload.encode(),
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)
```

---

## Next Steps

- **Start integration**: [Getting Started](getting-started.md)
- **Configure policies**: [Policy Engine](policy-engine.md)
- **Set up control plane**: [Control Plane](control-plane.md)
- **Review security**: [Security & Privacy](security-privacy.md)
