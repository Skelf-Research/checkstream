# CheckStream Control Plane

The CheckStream Control Plane is a SaaS management layer that orchestrates policies, models, and telemetry across all enforcement nodes—without touching the LLM traffic data plane.

---

## Architecture Overview

```
                   SaaS Control Plane (multi-tenant, regional)
┌──────────────────────────────────────────────────────────────────┐
│  ┌──────────┐  ┌───────────┐  ┌─────────┐  ┌─────────────────┐ │
│  │ Policy   │  │  Model    │  │  Fleet  │  │  Telemetry      │ │
│  │ Store    │  │  Registry │  │  Manager│  │  Ingest (opt)   │ │
│  └──────────┘  └───────────┘  └─────────┘  └─────────────────┘ │
│  ┌──────────┐  ┌───────────┐  ┌─────────┐  ┌─────────────────┐ │
│  │ AuthN/Z  │  │  Audit    │  │ Webhook │  │  Dashboards     │ │
│  │ & RBAC   │  │  Ledger   │  │ / SIEM  │  │  & Analytics    │ │
│  └──────────┘  └───────────┘  └─────────┘  └─────────────────┘ │
└─────────────────────┬────────────────────────────────────────────┘
                      │ (mTLS, signed bundles)
           ┌──────────┼──────────┬───────────┐
           │          │          │           │
      ┌────▼────┐ ┌──▼─────┐ ┌──▼──────┐ ┌──▼──────┐
      │ Proxy   │ │ Proxy  │ │ Sidecar │ │ Sidecar │
      │ Node 1  │ │ Node 2 │ │ Node 1  │ │ Node 2  │
      └─────────┘ └────────┘ └─────────┘ └─────────┘
           │          │          │           │
      (LLM Traffic - stays in customer VPC, never touches control plane)
```

**Key Principle**: The control plane is **out-of-band**. LLM tokens flow through enforcement nodes; only metadata flows to the control plane.

---

## Core Components

### 1. Policy Store & Compiler

**Purpose**: Centralized policy management with version control

**Features**:
- Git-backed policy storage
- Visual policy editor (web UI)
- YAML policy compiler with syntax validation
- Signed bundle generation (policy bytecode + metadata)
- Canary rollouts and staged deployments
- Approval workflows for policy changes

**Workflow**:
```
1. Author policy in UI or Git
2. Validate syntax and semantics
3. Submit for approval (optional)
4. Compiler builds signed bundle
5. Publish to CDN
6. Nodes poll and hot-reload
```

**API**:
```bash
# Create policy
curl -X POST https://control.checkstream.ai/v1/policies \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/yaml" \
  --data-binary @policy.yaml

# List policies
curl https://control.checkstream.ai/v1/policies

# Get policy
curl https://control.checkstream.ai/v1/policies/consumer-duty-v2

# Update policy
curl -X PUT https://control.checkstream.ai/v1/policies/consumer-duty-v2 \
  --data-binary @updated.yaml

# Deploy policy to fleet
curl -X POST https://control.checkstream.ai/v1/policies/consumer-duty-v2/deploy \
  -d '{"fleets": ["production-eu"], "canary_percent": 10}'
```

---

### 2. Model Registry

**Purpose**: Manage classifier models and versioning

**Features**:
- Model artifact storage (ONNX, TensorRT)
- Model cards (purpose, training data, performance metrics)
- Automatic distribution to regional CDNs
- Version pinning per fleet
- A/B testing framework

**Models Available**:
- Toxicity detector (multilingual, INT8)
- Prompt injection classifier
- PII/PHI detector
- Regulatory classifiers (FCA, FINRA, MiFID II)
- Domain-specific models (finance, healthcare, legal)

**Metadata Example**:
```json
{
  "model_id": "toxicity-detector-v2.1",
  "type": "classifier",
  "framework": "onnx",
  "precision": "int8",
  "size_mb": 45,
  "latency_p95_ms": 6.2,
  "accuracy": 0.94,
  "f1_score": 0.92,
  "training_data": {
    "sources": ["civil_comments", "jigsaw", "custom_finance"],
    "samples": 1200000,
    "languages": ["en", "es", "fr", "de"]
  },
  "created": "2024-01-10",
  "cdn_urls": {
    "us-east-1": "https://models-us-east.checkstream.ai/toxicity-v2.1.onnx",
    "eu-west-2": "https://models-eu-west.checkstream.ai/toxicity-v2.1.onnx"
  }
}
```

**API**:
```bash
# List models
curl https://control.checkstream.ai/v1/models

# Get model metadata
curl https://control.checkstream.ai/v1/models/toxicity-detector-v2.1

# Download model artifact
curl https://models-eu-west.checkstream.ai/toxicity-v2.1.onnx \
  -H "Authorization: Bearer ${TOKEN}" \
  -o toxicity-v2.1.onnx
```

---

### 3. Fleet Manager

**Purpose**: Track and orchestrate all enforcement nodes

**Capabilities**:
- Node registration and heartbeat monitoring
- Desired state management (policy + model versions)
- Health checks and auto-recovery
- Regional deployment tracking
- Performance metrics aggregation

**Fleet View**:
```
Organization: acme-bank
├─ Fleet: production-eu-west-2
│  ├─ Proxy Nodes: 12
│  │  ├─ checkstream-proxy-001 (v1.3.0, policy: v2.3.1, healthy)
│  │  ├─ checkstream-proxy-002 (v1.3.0, policy: v2.3.1, healthy)
│  │  └─ checkstream-proxy-003 (v1.2.9, policy: v2.3.0, ⚠️ outdated)
│  └─ Sidecar Nodes: 5
│     └─ All v1.3.0, policy v2.3.1 ✓
├─ Fleet: production-us-east-1
│  └─ Proxy Nodes: 8 (all healthy)
└─ Fleet: staging-eu-west-2
   └─ Proxy Nodes: 3 (canary: policy v2.4.0-beta)
```

**Desired State**:
```yaml
fleet: production-eu-west-2
desired_state:
  policy_bundle: consumer-duty-v2.3.1
  models:
    toxicity: v2.1.0
    advice_vs_info: v1.5.0
    pii_detector: v3.0.1
  enforcement_mode: strict
  telemetry_mode: aggregate
```

**API**:
```bash
# List fleets
curl https://control.checkstream.ai/v1/fleets

# Get fleet details
curl https://control.checkstream.ai/v1/fleets/production-eu-west-2

# Update desired state
curl -X PUT https://control.checkstream.ai/v1/fleets/production-eu-west-2/desired-state \
  -d '{
    "policy_bundle": "consumer-duty-v2.3.2",
    "rollout_strategy": "canary_10_then_full"
  }'

# View node health
curl https://control.checkstream.ai/v1/fleets/production-eu-west-2/nodes
```

---

### 4. Telemetry Ingest (Optional)

**Purpose**: Aggregate enforcement decisions for dashboards

**Privacy Modes**:

#### Aggregate Mode (default)
Only metrics, no per-request details:
```json
{
  "node_id": "proxy-eu-001",
  "interval": "2024-01-15T10:00:00Z/PT1M",
  "requests": 1523,
  "tokens": 45690,
  "decisions": {
    "allow": 1489,
    "redact": 28,
    "stop": 6
  },
  "latency_p95_ms": 8.3,
  "rules_triggered": {
    "promotional_balance": 12,
    "suitability_check": 4
  }
}
```

#### Full Evidence Mode (opt-in)
Per-decision records (PII-minimized):
```json
{
  "stream_id": "req_abc123",
  "timestamp": "2024-01-15T10:05:23.451Z",
  "node_id": "proxy-eu-001",
  "decision": {
    "rule_id": "advice_boundary_FCA",
    "action": "inject_disclaimer",
    "regulation": "FCA COBS 9A",
    "confidence": 0.87,
    "latency_ms": 6.2
  },
  "context_hash": "sha256:...",  // No raw text
  "policy_bundle": "v2.3.1",
  "hash_chain": "prev:def456,curr:abc789"
}
```

**Configuration**:
```yaml
# In node config
telemetry:
  mode: aggregate  # aggregate | full_evidence | none
  export:
    enabled: true
    endpoint: https://telemetry-eu.checkstream.ai/ingest
    batch_size: 100
    flush_interval: 60s
  privacy:
    hash_spans: true      # Hash text snippets, don't send raw
    redact_pii: true
    max_span_length: 50   # Limit snippet size
```

---

### 5. Audit Ledger

**Purpose**: Immutable record of all policy changes and critical events

**What's Logged**:
- Policy creations, updates, deletions
- Model deployments
- Fleet configuration changes
- Admin actions (approve, reject, rollback)
- Node attestations (what's running where)

**Audit Entry**:
```json
{
  "event_id": "evt_abc123",
  "timestamp": "2024-01-15T14:30:00Z",
  "event_type": "policy_deployed",
  "actor": "risk_officer@acme-bank.com",
  "resource": "policy/consumer-duty-v2.3.1",
  "action": "deploy",
  "target": "fleet/production-eu-west-2",
  "metadata": {
    "approval_ticket": "RISK-1234",
    "approved_by": "chief_risk_officer@acme-bank.com",
    "regulation_update": "FCA PS23/6"
  },
  "signature": "sha256:...",
  "previous_event": "evt_def456"
}
```

**Export for Regulators**:
```bash
# Generate audit report
curl -X POST https://control.checkstream.ai/v1/audit/export \
  -d '{
    "start_date": "2024-01-01",
    "end_date": "2024-03-31",
    "format": "pdf",
    "include": ["policy_changes", "deployment_events", "breach_incidents"]
  }' \
  -o Q1_2024_audit_report.pdf
```

---

### 6. Dashboards & Analytics

#### Compliance Dashboard

**Consumer Duty Outcomes**:
```
┌─ Products & Services ────────────────────────┐
│ Target market mismatches prevented:      23 │
│ Unsuitable products blocked:              12 │
│ Product accuracy corrections:             45 │
└──────────────────────────────────────────────┘

┌─ Price & Value ──────────────────────────────┐
│ Price disclosures injected:             1,234│
│ Fee transparency enforced:               100%│
│ Balanced promotions:                      567│
└──────────────────────────────────────────────┘

┌─ Consumer Understanding ─────────────────────┐
│ Misleading statements redacted:           45 │
│ Risk warnings added:                      567│
│ Complexity simplifications:                89 │
└──────────────────────────────────────────────┘

┌─ Consumer Support ───────────────────────────┐
│ Vulnerability detections:                  78 │
│ Supportive tone adaptations:               56 │
│ Resource links provided:                  134 │
└──────────────────────────────────────────────┘
```

#### Security Dashboard

**Threat Detection**:
```
┌─ Prompt Injection Attempts ──────────────────┐
│ Last 24h:  127 blocked,  3 flagged for review│
│ Success rate: 97.7%                          │
│ Top patterns:                                │
│   - "Ignore previous instructions" (45)      │
│   - "System prompt override" (32)            │
│   - Indirect injection via docs (18)         │
└──────────────────────────────────────────────┘

┌─ Data Exfiltration Flags ────────────────────┐
│ PII redactions (last 7d): 234                │
│ Secret patterns detected:  12                │
│ High-risk contexts blocked: 8                │
└──────────────────────────────────────────────┘
```

#### Operational Dashboard

**Performance Metrics**:
```
┌─ Latency ────────────────────────────────────┐
│ TTFT p95:                              287ms │
│ Tokens/sec:                             52.3 │
│ Decision latency p95:                   8.1ms│
│ Holdback delay avg:                      35ms│
└──────────────────────────────────────────────┘

┌─ Fleet Health ───────────────────────────────┐
│ Total nodes: 25  (24 healthy, 1 degraded)    │
│ Policy drift: 1 node outdated                │
│ Model drift: 0                               │
│ CPU utilization avg: 34%                     │
└──────────────────────────────────────────────┘
```

---

## Setup & Onboarding

### 1. Create Organization

```bash
# Sign up at https://control.checkstream.ai

# Or via CLI
checkstream signup \
  --org "Acme Bank" \
  --email "admin@acme-bank.com"

# Receive org_id and initial admin credentials
```

### 2. Configure RBAC

**Roles**:
- **Org Admin**: Full access, user management
- **Risk Officer**: Policy approval, audit access
- **Compliance Analyst**: Dashboard read, evidence export
- **Engineer**: Node deployment, policy editing
- **Auditor**: Read-only access to all

```bash
# Invite users
checkstream users invite \
  --email risk.officer@acme-bank.com \
  --role risk_officer

# Assign roles
checkstream users assign-role \
  --user engineer@acme-bank.com \
  --role engineer
```

### 3. Deploy First Node

```bash
# Install agent on your infrastructure
curl -sSL https://install.checkstream.ai | bash

# Authenticate
checkstream login --org acme-bank

# Deploy proxy
checkstream deploy proxy \
  --fleet production-eu-west-2 \
  --backend https://api.openai.com/v1 \
  --policy-sync auto \
  --telemetry aggregate
```

Node registers with control plane automatically.

### 4. Configure Policies

```bash
# Install policy pack
checkstream policy-packs install fca-consumer-duty

# Or create custom policy
cat > custom-policy.yaml <<EOF
policies:
  - name: acme_bank_custom
    rules:
      - ...
EOF

checkstream policies create custom-policy.yaml

# Deploy to fleet
checkstream policies deploy acme_bank_custom \
  --fleet production-eu-west-2
```

### 5. Verify Operation

```bash
# Check fleet status
checkstream fleets status production-eu-west-2

# View live metrics
checkstream dashboards open

# Test enforcement
curl http://your-proxy:8080/v1/chat/completions \
  -d '{"model": "gpt-4", "messages": [...]}'
```

---

## Advanced Features

### Canary Deployments

Roll out policy changes gradually:

```bash
# Deploy to 10% of fleet
checkstream policies deploy consumer-duty-v2.4.0 \
  --fleet production-eu-west-2 \
  --canary 10

# Monitor canary metrics
checkstream policies canary-status consumer-duty-v2.4.0

# If metrics look good, promote to 50%
checkstream policies canary-promote consumer-duty-v2.4.0 --to 50

# Full rollout
checkstream policies canary-promote consumer-duty-v2.4.0 --to 100
```

### Multi-Region Orchestration

Deploy consistently across regions:

```bash
checkstream policies deploy consumer-duty-v2.4.0 \
  --fleets production-eu-west-2,production-us-east-1,production-ap-southeast-1 \
  --strategy sequential \  # or 'parallel'
  --wait-between 5m
```

### Alerting & Webhooks

```bash
# Configure Slack alerts
checkstream integrations add slack \
  --webhook https://hooks.slack.com/... \
  --events policy_drift,high_risk_spike,node_unhealthy

# SIEM export
checkstream integrations add siem \
  --type splunk \
  --endpoint https://splunk.acme-bank.com/hec \
  --token ${SPLUNK_HEC_TOKEN}
```

### Compliance Evidence Export

```bash
# Monthly Consumer Duty report
checkstream compliance export consumer-duty \
  --period 2024-01 \
  --format pdf \
  --include-samples 50 \
  --output acme_bank_consumer_duty_jan2024.pdf

# Send to regulator
```

---

## Pricing

### Tiers

| Tier | Nodes | Policies | Support | Price |
|------|-------|----------|---------|-------|
| **Developer** | 1 | Community | Email | Free |
| **Professional** | Up to 10 | Pre-built + custom | Email + Chat | $500/month base |
| **Enterprise** | Unlimited | Unlimited | SLA + Phone | Custom (from $50K/year) |

### Enterprise Pricing

**Base License**: $50K - $250K/year (depends on scale)

**Usage-Based**: $0.0008 per 1K tokens processed

**Add-Ons**:
- Dedicated support engineer: +$50K/year
- Custom model training: +$100K one-time
- On-premise control plane: +$150K/year

---

## Data Residency

Control plane deployments:

- **US**: us-east-1 (AWS)
- **EU**: eu-west-2 London (AWS), eu-central-1 Frankfurt (GCP)
- **APAC**: ap-southeast-1 Singapore (AWS)

**Enforcement nodes run in your VPC**; only metadata crosses to control plane (over mTLS).

---

## Next Steps

- **Deploy your first node**: [Getting Started](getting-started.md)
- **Configure policies**: [Policy Engine](policy-engine.md)
- **Review security model**: [Security & Privacy](security-privacy.md)
- **Explore API**: [API Reference](api-reference.md)
