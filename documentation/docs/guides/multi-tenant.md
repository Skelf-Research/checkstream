# Multi-Tenant Setup

Configure CheckStream for multiple tenants with isolated policies and backends.

---

## Overview

Multi-tenant mode allows a single CheckStream instance to serve multiple clients with:

- Separate LLM backends per tenant
- Independent policies per tenant
- Custom streaming formats
- Isolated audit trails

---

## Enabling Multi-Tenancy

```yaml
tenants:
  enabled: true
  resolution:
    - header: "X-Tenant-ID"
  default_tenant: "default"
```

---

## Tenant Resolution

CheckStream resolves tenants in priority order:

### 1. Header-Based (Recommended)

```yaml
tenants:
  resolution:
    - header: "X-Tenant-ID"
```

Client sends:
```bash
curl -H "X-Tenant-ID: acme-corp" http://localhost:8080/v1/chat/completions
```

### 2. Path Prefix

```yaml
tenants:
  resolution:
    - path_prefix: "/tenant/"
```

Client sends:
```bash
curl http://localhost:8080/tenant/acme-corp/v1/chat/completions
```

### 3. API Key Mapping

```yaml
tenants:
  resolution:
    - api_key_mapping: true

  api_key_map:
    "sk-acme-123": "acme-corp"
    "sk-beta-456": "beta-inc"
```

### 4. Combined Resolution

```yaml
tenants:
  resolution:
    - header: "X-Tenant-ID"      # Try header first
    - path_prefix: "/tenant/"     # Then path
    - api_key_mapping: true       # Then API key
  default_tenant: "default"       # Fallback
```

---

## Tenant Configuration

### Basic Tenant

```yaml
tenants:
  enabled: true
  configs:
    acme-corp:
      backend_url: "https://api.openai.com/v1"
      policy_path: "./policies/acme.yaml"
```

### Full Tenant Configuration

```yaml
tenants:
  enabled: true
  configs:
    acme-corp:
      # Backend
      backend_url: "https://api.openai.com/v1"
      backend_timeout_ms: 30000

      # Policies
      policy_path: "./policies/acme.yaml"

      # Streaming format
      streaming_format: "openai"

      # Thresholds (override global)
      thresholds:
        safety: 0.9
        chunk: 0.8

      # Pipeline (override global)
      pipeline:
        ingress:
          classifiers:
            - prompt_injection
            - custom_acme_classifier
        midstream:
          classifiers:
            - toxicity

      # Rate limiting
      rate_limit:
        requests_per_minute: 1000
        tokens_per_minute: 100000

      # Audit
      audit:
        enabled: true
        path: "./audit/acme"
```

---

## Different Backends Per Tenant

### OpenAI + Anthropic

```yaml
tenants:
  configs:
    openai-tenant:
      backend_url: "https://api.openai.com/v1"
      streaming_format: "openai"

    anthropic-tenant:
      backend_url: "https://api.anthropic.com"
      streaming_format: "anthropic"
      headers:
        anthropic-version: "2024-01-01"
```

### Self-Hosted LLMs

```yaml
tenants:
  configs:
    vllm-tenant:
      backend_url: "http://vllm-server:8000/v1"
      streaming_format: "openai"

    ollama-tenant:
      backend_url: "http://ollama:11434/v1"
      streaming_format: "openai"
```

---

## Per-Tenant Policies

### Strict Policy (Finance)

```yaml
# policies/finance.yaml
version: "1.0"
name: "finance-policy"

policies:
  - name: block_investment_advice
    trigger:
      classifier: financial_advice
      threshold: 0.7
    action: stop
    regulation: "FCA COBS 9A.2.1R"
```

### Permissive Policy (Internal)

```yaml
# policies/internal.yaml
version: "1.0"
name: "internal-policy"

policies:
  - name: log_safety
    trigger:
      classifier: toxicity
      threshold: 0.5
    action: log
    # No blocking, just logging
```

### Tenant Configuration

```yaml
tenants:
  configs:
    finance-team:
      policy_path: "./policies/finance.yaml"
      thresholds:
        safety: 0.9

    internal-team:
      policy_path: "./policies/internal.yaml"
      thresholds:
        safety: 0.5
```

---

## Per-Tenant Classifiers

Load different classifiers per tenant:

```yaml
tenants:
  configs:
    healthcare-tenant:
      classifiers:
        - toxicity
        - pii_detector
        - medical_advice          # Healthcare-specific

    finance-tenant:
      classifiers:
        - toxicity
        - pii_detector
        - financial_advice        # Finance-specific
```

---

## Rate Limiting

### Per-Tenant Limits

```yaml
tenants:
  configs:
    free-tier:
      rate_limit:
        requests_per_minute: 60
        tokens_per_minute: 10000

    pro-tier:
      rate_limit:
        requests_per_minute: 600
        tokens_per_minute: 100000

    enterprise:
      rate_limit:
        requests_per_minute: 6000
        tokens_per_minute: 1000000
```

### Response Headers

```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1704067200
```

---

## Isolated Audit Trails

```yaml
tenants:
  configs:
    tenant-a:
      audit:
        enabled: true
        path: "./audit/tenant-a"
        retention_days: 90

    tenant-b:
      audit:
        enabled: true
        path: "./audit/tenant-b"
        retention_days: 365       # Longer retention
```

---

## Authentication

### Pass-through (Client's Keys)

```yaml
tenants:
  configs:
    byok-tenant:                  # Bring Your Own Key
      auth_passthrough: true
      # Client provides their own API key
```

### Managed Keys

```yaml
tenants:
  configs:
    managed-tenant:
      auth:
        type: bearer
        token_env: "TENANT_A_API_KEY"
      # CheckStream uses its own key
```

---

## Metrics Per Tenant

Metrics include tenant labels:

```
checkstream_requests_total{tenant="acme-corp",status="success"} 1234
checkstream_latency_ms{tenant="acme-corp",phase="ingress",quantile="0.95"} 3.2
checkstream_policy_triggers_total{tenant="acme-corp",rule="block_toxicity"} 56
```

---

## Complete Example

```yaml
server:
  host: "0.0.0.0"
  port: 8080

tenants:
  enabled: true
  resolution:
    - header: "X-Tenant-ID"
    - api_key_mapping: true
  default_tenant: "default"

  api_key_map:
    "sk-acme-prod": "acme-corp"
    "sk-beta-test": "beta-inc"

  configs:
    default:
      backend_url: "https://api.openai.com/v1"
      policy_path: "./policies/default.yaml"
      rate_limit:
        requests_per_minute: 100

    acme-corp:
      backend_url: "https://api.openai.com/v1"
      policy_path: "./policies/acme.yaml"
      thresholds:
        safety: 0.9
        chunk: 0.85
      rate_limit:
        requests_per_minute: 1000
      audit:
        enabled: true
        path: "./audit/acme"

    beta-inc:
      backend_url: "https://api.anthropic.com"
      streaming_format: "anthropic"
      policy_path: "./policies/beta.yaml"
      headers:
        anthropic-version: "2024-01-01"
      rate_limit:
        requests_per_minute: 500

pipeline:
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
  midstream:
    enabled: true
    classifiers:
      - toxicity
```

---

## Testing Multi-Tenancy

### Test Tenant Resolution

```bash
# Header-based
curl -H "X-Tenant-ID: acme-corp" http://localhost:8080/v1/chat/completions

# Path-based
curl http://localhost:8080/tenant/acme-corp/v1/chat/completions

# API key mapping
curl -H "Authorization: Bearer sk-acme-prod" http://localhost:8080/v1/chat/completions
```

### Verify Tenant

```bash
curl http://localhost:8080/admin/tenant-info \
  -H "X-Tenant-ID: acme-corp"
```

```json
{
  "tenant": "acme-corp",
  "backend": "https://api.openai.com/v1",
  "policy": "acme.yaml",
  "rate_limit": {
    "requests_per_minute": 1000,
    "remaining": 950
  }
}
```

---

## Next Steps

- [Configuration Reference](../configuration/proxy.md) - Full configuration options
- [Policy Engine](policy-engine.md) - Write tenant-specific policies
- [Deployment](../deployment/kubernetes.md) - Scale multi-tenant deployments
