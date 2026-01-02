# Metrics & Telemetry Reference

Complete reference for CheckStream observability.

---

## Prometheus Metrics

All metrics use the `checkstream_` prefix by default.

### Request Metrics

```
# Total requests processed
checkstream_requests_total{tenant="default",status="success|blocked|error"} counter

# Request latency histogram
checkstream_request_latency_ms{tenant="default",quantile="0.5|0.95|0.99"} histogram

# Active connections
checkstream_connections_active{tenant="default"} gauge

# Bytes transferred
checkstream_bytes_total{tenant="default",direction="in|out"} counter
```

### Phase Metrics

```
# Phase latency
checkstream_phase_latency_ms{phase="ingress|midstream|egress",quantile="0.5|0.95|0.99"} histogram

# Phase decisions
checkstream_phase_decisions_total{phase="ingress|midstream|egress",decision="allow|block|redact"} counter

# Tokens processed (midstream)
checkstream_tokens_processed_total{tenant="default"} counter
checkstream_tokens_redacted_total{tenant="default"} counter
```

### Classifier Metrics

```
# Classifier calls
checkstream_classifier_calls_total{classifier="toxicity",result="positive|negative"} counter

# Classifier latency
checkstream_classifier_latency_ms{classifier="toxicity",tier="A|B|C",quantile="0.95"} histogram

# Classifier errors
checkstream_classifier_errors_total{classifier="toxicity",error="timeout|model_error"} counter

# Classifier scores histogram
checkstream_classifier_scores{classifier="toxicity",bucket="0.1|0.2|...|0.9|1.0"} histogram
```

### Policy Metrics

```
# Policy triggers
checkstream_policy_triggers_total{policy="default",rule="block_toxicity",action="stop|redact|log"} counter

# Policy evaluation time
checkstream_policy_eval_latency_ms{policy="default",quantile="0.95"} histogram

# Shadow mode triggers (testing)
checkstream_shadow_triggers_total{policy="default",rule="test_rule"} counter
```

### Model Metrics

```
# Model memory usage
checkstream_model_memory_bytes{model="toxicity",device="cpu|cuda"} gauge

# Model inference time
checkstream_model_inference_ms{model="toxicity",quantile="0.95"} histogram

# Model cache hits/misses
checkstream_model_cache_hits_total{model="toxicity"} counter
checkstream_model_cache_misses_total{model="toxicity"} counter
```

### Backend Metrics

```
# Backend request latency
checkstream_backend_latency_ms{backend="openai",quantile="0.95"} histogram

# Backend errors
checkstream_backend_errors_total{backend="openai",error="timeout|5xx|connection"} counter

# Backend retry attempts
checkstream_backend_retries_total{backend="openai"} counter
```

### System Metrics

```
# Process metrics
checkstream_process_cpu_seconds_total counter
checkstream_process_resident_memory_bytes gauge
checkstream_process_open_fds gauge

# Runtime metrics
checkstream_runtime_threads gauge
checkstream_runtime_tasks_active gauge
```

---

## Metric Labels

### Common Labels

| Label | Description | Values |
|-------|-------------|--------|
| `tenant` | Tenant identifier | String |
| `phase` | Pipeline phase | `ingress`, `midstream`, `egress` |
| `classifier` | Classifier name | String |
| `tier` | Classifier tier | `A`, `B`, `C` |
| `status` | Request status | `success`, `blocked`, `error` |
| `action` | Policy action | `stop`, `redact`, `log`, `inject` |

### Quantile Labels

Histogram metrics include quantile buckets:

| Quantile | Meaning |
|----------|---------|
| `0.5` | Median (50th percentile) |
| `0.9` | 90th percentile |
| `0.95` | 95th percentile |
| `0.99` | 99th percentile |

---

## Configuration

### Enable Metrics

```yaml
telemetry:
  metrics:
    enabled: true
    port: 9090
    path: "/metrics"
    prefix: "checkstream"
```

### Custom Labels

```yaml
telemetry:
  metrics:
    custom_labels:
      environment: "production"
      region: "us-east-1"
```

### Histogram Buckets

```yaml
telemetry:
  metrics:
    latency_buckets: [1, 2, 5, 10, 25, 50, 100, 250, 500, 1000]
    score_buckets: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
```

---

## Grafana Dashboards

### Overview Dashboard

Key panels:
- Requests per second (by status)
- P95 latency by phase
- Policy triggers over time
- Error rate

### Classifier Dashboard

Key panels:
- Classifier latency by tier
- Score distribution
- Positive rate over time
- Cache hit ratio

### Compliance Dashboard

Key panels:
- Blocks by regulation
- Audit records created
- Shadow mode triggers
- Policy coverage

---

## Alerting Rules

### High Latency

```yaml
# Prometheus alerting rule
groups:
  - name: checkstream
    rules:
      - alert: HighIngressLatency
        expr: histogram_quantile(0.95, checkstream_phase_latency_ms{phase="ingress"}) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High ingress latency detected"
```

### High Error Rate

```yaml
- alert: HighErrorRate
  expr: |
    sum(rate(checkstream_requests_total{status="error"}[5m])) /
    sum(rate(checkstream_requests_total[5m])) > 0.01
  for: 5m
  labels:
    severity: critical
```

### Classifier Failure

```yaml
- alert: ClassifierFailure
  expr: rate(checkstream_classifier_errors_total[5m]) > 0
  for: 1m
  labels:
    severity: critical
```

### Backend Unavailable

```yaml
- alert: BackendUnavailable
  expr: rate(checkstream_backend_errors_total{error="connection"}[1m]) > 5
  for: 1m
  labels:
    severity: critical
```

---

## Structured Logging

### Log Format

```yaml
telemetry:
  logging:
    level: info
    format: json
```

### Log Fields

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "message": "Request processed",
  "request_id": "req-abc123",
  "tenant": "default",
  "phase": "ingress",
  "latency_ms": 3.5,
  "decision": "allow",
  "classifiers": {
    "toxicity": 0.12,
    "prompt_injection": 0.05
  }
}
```

### Log Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `error` | Errors requiring attention | Failures, exceptions |
| `warn` | Warning conditions | Policy triggers, slow queries |
| `info` | Normal operations | Requests, decisions |
| `debug` | Detailed debugging | Classifier scores, timing |
| `trace` | Very detailed tracing | Token-level processing |

---

## Audit Trail

### Configuration

```yaml
telemetry:
  audit:
    enabled: true
    path: "./audit"
    rotation: daily
    retention_days: 90
    hash_chain: true
    include:
      - request_id
      - timestamp
      - tenant
      - input_hash
      - output_hash
      - classifiers
      - actions
      - regulations
```

### Audit Record Format

```json
{
  "id": "audit-2024011510300001",
  "timestamp": "2024-01-15T10:30:00.123Z",
  "previous_hash": "sha256:abc123...",
  "hash": "sha256:def456...",
  "request_id": "req-abc123",
  "tenant": "acme-corp",
  "input_hash": "sha256:input...",
  "output_hash": "sha256:output...",
  "classifiers": [
    {"name": "toxicity", "score": 0.82, "tier": "B"}
  ],
  "actions": [
    {"type": "redact", "rule": "redact_toxic", "target": "midstream"}
  ],
  "regulations": ["internal-policy"],
  "latency_ms": 5.2
}
```

### Hash Chain Verification

```bash
curl http://localhost:8080/audit/verify?start=2024-01-01&end=2024-01-31
```

```json
{
  "status": "valid",
  "records_verified": 50000,
  "chain_intact": true,
  "gaps": [],
  "first_record": "2024-01-01T00:00:01Z",
  "last_record": "2024-01-31T23:59:59Z"
}
```

---

## OpenTelemetry Integration

### Configuration

```yaml
telemetry:
  opentelemetry:
    enabled: true
    endpoint: "http://otel-collector:4317"
    service_name: "checkstream"
    traces:
      enabled: true
      sampling_rate: 0.1
    metrics:
      enabled: true
      interval_seconds: 60
```

### Trace Spans

```
checkstream.request
├── checkstream.ingress
│   ├── checkstream.classifier.toxicity
│   └── checkstream.classifier.prompt_injection
├── checkstream.backend.request
├── checkstream.midstream
│   └── checkstream.classifier.toxicity (per chunk)
└── checkstream.egress
    └── checkstream.audit
```

---

## Health Metrics

### Endpoint

```bash
curl http://localhost:8080/health/ready
```

### Response

```json
{
  "status": "ready",
  "uptime_seconds": 86400,
  "checks": {
    "classifiers": {
      "status": "healthy",
      "loaded": 5,
      "failed": 0
    },
    "policies": {
      "status": "healthy",
      "loaded": 3,
      "rules": 25
    },
    "backend": {
      "status": "healthy",
      "latency_ms": 150
    },
    "audit": {
      "status": "healthy",
      "records_today": 12500
    }
  }
}
```

---

## Best Practices

1. **Set appropriate retention** - Balance storage vs compliance needs
2. **Use sampling** - Reduce trace volume in high-traffic systems
3. **Alert on P95, not P50** - Catch tail latency issues
4. **Monitor classifier accuracy** - Track false positive rates
5. **Hash chain verification** - Regular integrity checks
6. **Dashboard for each audience** - Ops, compliance, developers

---

## Next Steps

- [Deployment Guide](../deployment/kubernetes.md) - Production monitoring setup
- [API Reference](api.md) - Query metrics and audit data
- [Compliance Guide](../guides/compliance.md) - Regulatory audit requirements
