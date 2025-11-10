# CheckStream Deployment Modes

CheckStream supports three deployment modes, each optimized for different requirements and infrastructure setups. You can start with Proxy Mode and evolve to Sidecar or Control Plane as your needs grow.

## Deployment Modes Comparison

| Aspect | Proxy Mode | Sidecar Mode | Control Plane |
|--------|------------|--------------|---------------|
| **Infrastructure** | Standalone HTTP proxy | Co-deployed with vLLM | SaaS + local nodes |
| **Integration effort** | Minimal (URL change) | Moderate (vLLM config) | Low (agent install) |
| **Model support** | Any HTTP/SSE API | vLLM only | Both proxy + sidecar |
| **Latency overhead** | ~10ms per chunk | ~5ms per chunk | Depends on node mode |
| **Safety capabilities** | Reactive (buffer+patch) | Preventive (logit mask) | All capabilities |
| **Data residency** | In-VPC deployment | In-VPC (same pod) | Configurable privacy modes |
| **Policy management** | Local YAML files | Local YAML files | Centralized SaaS |
| **Telemetry** | Local logs | Local logs | Centralized dashboards |
| **Best for** | Quick start, multi-cloud | Maximum control, vLLM users | Enterprise governance |

---

## Mode 1: Proxy Mode (Universal AI Firewall)

### Overview

A standalone HTTP/SSE proxy that sits between your application and any LLM API. Works with OpenAI, Anthropic, Bedrock, Azure OpenAI, or self-hosted models.

### Architecture

```
Client App → CheckStream Proxy → LLM API (OpenAI/Anthropic/Bedrock)
```

The proxy:
1. Intercepts requests to LLM APIs
2. Applies ingress guardrails to prompts
3. Streams responses through holdback buffer
4. Runs classifiers and policy checks per chunk
5. Patches or stops unsafe content before reaching client

### Deployment

**Docker**:
```bash
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/policies:/etc/checkstream/policies \
  -e BACKEND_URL=https://api.openai.com/v1 \
  -e OPENAI_API_KEY=${OPENAI_API_KEY} \
  checkstream/proxy:latest
```

**Kubernetes**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: checkstream-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: checkstream-proxy
  template:
    metadata:
      labels:
        app: checkstream-proxy
    spec:
      containers:
      - name: proxy
        image: checkstream/proxy:latest
        ports:
        - containerPort: 8080
        env:
        - name: BACKEND_URL
          value: "https://api.anthropic.com/v1"
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: llm-credentials
              key: anthropic-key
        volumeMounts:
        - name: policies
          mountPath: /etc/checkstream/policies
      volumes:
      - name: policies
        configMap:
          name: checkstream-policies
---
apiVersion: v1
kind: Service
metadata:
  name: checkstream-proxy
spec:
  selector:
    app: checkstream-proxy
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

**AWS ECS/Fargate**:
```json
{
  "family": "checkstream-proxy",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "2048",
  "memory": "4096",
  "containerDefinitions": [
    {
      "name": "proxy",
      "image": "checkstream/proxy:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "BACKEND_URL",
          "value": "https://bedrock-runtime.us-east-1.amazonaws.com"
        }
      ],
      "secrets": [
        {
          "name": "AWS_ACCESS_KEY_ID",
          "valueFrom": "arn:aws:secretsmanager:..."
        }
      ]
    }
  ]
}
```

### Configuration

**config.yaml**:
```yaml
server:
  port: 8080
  timeout: 300s

backend:
  url: https://api.openai.com/v1
  timeout: 120s
  retry:
    max_attempts: 3
    backoff: exponential

guardrails:
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
      - pii_detector
    timeout_ms: 8

  midstream:
    enabled: true
    holdback_size: 16
    check_interval: 8
    classifiers:
      - toxicity
      - regulatory_finance
    timeout_ms: 6

  egress:
    enabled: true
    inject_disclaimers: true

policies:
  path: /etc/checkstream/policies
  hot_reload: true
  reload_interval: 30s

telemetry:
  mode: aggregate  # or 'full_evidence'
  export:
    enabled: false
    endpoint: https://control.checkstream.ai/ingest
```

### Client Integration

Simply change your API endpoint:

**Before**:
```python
import openai

client = openai.OpenAI(api_key="sk-...")
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}],
    stream=True
)
```

**After**:
```python
import openai

client = openai.OpenAI(
    base_url="http://checkstream-proxy:8080/v1",  # ← Changed
    api_key="sk-..."
)
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}],
    stream=True
)
```

### Advantages

- **Zero model changes**: Works with any provider
- **Cloud-neutral**: Deploy anywhere
- **Fast adoption**: URL change only
- **Multi-model**: Same proxy for OpenAI, Anthropic, Bedrock

### Limitations

- **Reactive only**: Can only respond to tokens already generated
- **No logit access**: Cannot influence sampling or prevent unsafe tokens
- **Holdback required**: Must buffer tokens to inspect before emission
- **Network hop**: Adds proxy in request path

### When to Use

- **Multi-cloud** environments with different LLM providers
- **Quick start** without infrastructure changes
- **Proof of concept** before deeper integration
- **Managed LLMs** where you don't control inference (OpenAI, Anthropic)

---

## Mode 2: Sidecar Mode (Deep vLLM Integration)

### Overview

A co-deployed service that hooks directly into vLLM's generation pipeline. Enables preventive safety by controlling token sampling before emission.

### Architecture

```
┌────────────────────────────────────────┐
│              Kubernetes Pod            │
│  ┌──────────────┐   ┌───────────────┐ │
│  │  vLLM        │ ⟷ │  CheckStream  │ │
│  │  Container   │   │  Sidecar      │ │
│  └──────────────┘   └───────────────┘ │
│         │                    │         │
│    (GPU compute)      (CPU classifiers)│
└────────┼────────────────────┼──────────┘
         │                    │
         └───────→ SSE ←──────┘
                 (merged safe stream)
```

The sidecar:
1. Registers callbacks with vLLM's sampling engine
2. Receives token logits before sampling
3. Masks unsafe tokens (prevents generation)
4. Adjusts decoding parameters (temperature, top-p) dynamically
5. Inspects structured outputs (tool calls) mid-generation

### Deployment

**Docker Compose**:
```yaml
version: '3.8'

services:
  vllm:
    image: vllm/vllm-openai:latest
    runtime: nvidia
    environment:
      - VLLM_SIDECAR_SOCKET=/shared/checkstream.sock
    volumes:
      - shared-socket:/shared
      - ./models:/models
    command: >
      --model meta-llama/Llama-2-7b-chat-hf
      --gpu-memory-utilization 0.9
      --enable-checkstream-sidecar

  checkstream-sidecar:
    image: checkstream/vllm-sidecar:latest
    depends_on:
      - vllm
    volumes:
      - shared-socket:/shared
      - ./policies:/etc/checkstream/policies
    environment:
      - VLLM_SOCKET=/shared/checkstream.sock
      - POLICY_PATH=/etc/checkstream/policies

volumes:
  shared-socket:
```

**Kubernetes**:
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: vllm-with-guardrails
spec:
  containers:
  - name: vllm
    image: vllm/vllm-openai:latest
    resources:
      limits:
        nvidia.com/gpu: 1
    env:
    - name: VLLM_SIDECAR_SOCKET
      value: /shared/checkstream.sock
    volumeMounts:
    - name: shared
      mountPath: /shared
    - name: models
      mountPath: /models

  - name: checkstream-sidecar
    image: checkstream/vllm-sidecar:latest
    resources:
      requests:
        cpu: "4"
        memory: "8Gi"
    env:
    - name: VLLM_SOCKET
      value: /shared/checkstream.sock
    volumeMounts:
    - name: shared
      mountPath: /shared
    - name: policies
      mountPath: /etc/checkstream/policies

  volumes:
  - name: shared
    emptyDir: {}
  - name: models
    persistentVolumeClaim:
      claimName: model-storage
  - name: policies
    configMap:
      name: checkstream-policies
```

### Unique Capabilities

#### 1. Logit Masking

Prevent unsafe tokens from being sampled:

```python
# In vLLM callback
def on_sampling_step(logits, context):
    # CheckStream analyzes context
    if policy_engine.check_risk(context) > 0.7:
        # Zero out logits for banned tokens
        banned_tokens = [token_id for slur in BANNED_WORDS]
        logits[banned_tokens] = -float('inf')
    return logits
```

Result: Unsafe content **cannot be generated**, not just filtered.

#### 2. Adaptive Decoding

Adjust sampling parameters mid-generation:

```python
# Initial request: temperature=0.7, top_p=0.9
# After detecting risk at token 50:
{
    "temperature": 0.3,  # ← More conservative
    "top_p": 0.6,        # ← Reduced diversity
    "vocab_mask": safe_tokens_only
}
```

#### 3. Tool-Use Control

Inspect and modify structured tool calls:

```json
// LLM generates:
{
  "tool": "send_email",
  "to": "{{user_input_from_context}}",  // ← Potential injection
  "body": "..."
}

// CheckStream detects injection, modifies:
{
  "tool": "send_email",
  "to": "validated@domain.com",
  "body": "...",
  "_checkstream_modified": true
}
```

#### 4. KV-Cache Reuse

Feed token embeddings directly to classifiers:

```python
# Instead of re-tokenizing for classifier:
embeddings = vllm.get_kv_cache_embeddings(step)
risk_score = classifier.predict_from_embeddings(embeddings)
# ~50% faster than text-based classification
```

### Configuration

**sidecar-config.yaml**:
```yaml
vllm:
  socket: /shared/checkstream.sock
  callbacks:
    - on_sampling_step
    - on_token_generated
    - on_tool_call

guardrails:
  preventive:
    enabled: true
    logit_masking:
      enabled: true
      banned_tokens_file: /etc/checkstream/banned_tokens.json
    adaptive_decoding:
      enabled: true
      risk_threshold: 0.7
      safe_mode:
        temperature: 0.3
        top_p: 0.6

  reactive:
    enabled: true
    holdback_size: 8  # Smaller than proxy mode
    check_interval: 10

classifiers:
  embedding_mode: true  # Use KV-cache embeddings
  models:
    - toxicity_detector
    - finance_regulatory
    - pii_detector

policies:
  path: /etc/checkstream/policies
  hot_reload: true
```

### Advantages

- **Preventive safety**: Control generation, not just filtering
- **Lower latency**: ~5ms vs ~10ms (smaller holdback, no external hop)
- **Stronger guarantees**: Unsafe tokens never generated
- **Rich telemetry**: Logits, embeddings, sampling decisions

### Limitations

- **vLLM-only**: Doesn't work with cloud APIs
- **Deployment complexity**: Requires infrastructure control
- **Resource overhead**: ~5-10% GPU utilization, 4-8 CPU cores

### When to Use

- **Self-hosted models** on vLLM infrastructure
- **Maximum safety requirements**: Financial, healthcare, government
- **Custom models**: Fine-tuned LLMs with domain-specific risks
- **High-volume production**: Latency matters at scale

---

## Mode 3: Control Plane (Enterprise Governance)

### Overview

A SaaS management layer that orchestrates policies, models, and telemetry across all proxy and sidecar nodes—without touching the data plane.

### Architecture

```
                   SaaS Control Plane
┌──────────────────────────────────────────────────────┐
│  Policy Store │ Model Registry │ Fleet Manager       │
│  Telemetry Ingest │ Audit Ledger │ Dashboards        │
└────────────┬───────────────┬──────────────┬──────────┘
             │               │              │
      (policy bundles)  (model artifacts) (metrics)
             │               │              │
    ┌────────┴───────┬───────┴──────┬───────┴─────────┐
    │                │              │                 │
┌───▼────┐      ┌───▼────┐    ┌───▼────┐       ┌────▼────┐
│ Proxy  │      │ Proxy  │    │Sidecar │       │ Sidecar │
│ Node 1 │      │ Node 2 │    │ Node 1 │       │ Node 2  │
└────────┘      └────────┘    └────────┘       └─────────┘
   │                │             │                  │
   └────── LLM Traffic (in customer VPC) ───────────┘
           (Control plane NEVER sees this)
```

### Key Features

#### Centralized Policy Management

**Web UI**:
- Visual policy editor with rule templates
- Git-backed version control
- Approval workflows for policy changes
- Diff viewer showing rule impacts

**Policy Distribution**:
```
1. Edit policy in UI/Git → Commit
2. Control plane compiles + signs bundle
3. Publishes to regional CDN
4. Nodes poll every 30s, hot-reload
5. No stream interruption
```

#### Fleet Orchestration

Track all deployed nodes:
- Health status (up/down/degraded)
- Policy version (detect drift)
- Latency metrics (p50/p95/p99)
- Actions taken (allow/redact/stop rates)
- Model versions running

**Fleet View**:
```
Region: eu-west-2
├─ Proxy Nodes: 15
│  ├─ v1.2.3 (policy bundle abc123): 12 nodes ✓
│  ├─ v1.2.2 (policy bundle def456): 2 nodes ⚠️ (outdated)
│  └─ v1.2.3 (policy bundle xyz789): 1 node ⚠️ (drift detected)
└─ Sidecar Nodes: 8
   └─ v1.3.0 (policy bundle abc123): 8 nodes ✓
```

#### Compliance Dashboards

**Consumer Duty Dashboard**:
- Breach trends by regulation (PRIN 2A, COBS, FG21/1)
- Vulnerability detection rates
- Disclaimer injection coverage
- Risk score distributions
- Evidence export (CSV/PDF for regulators)

**Security Dashboard**:
- Prompt injection attempts (blocked vs allowed)
- Data exfiltration flags
- Jailbreak patterns detected
- High-risk streams (with rationale spans)

#### Telemetry Options

**Aggregate Mode** (privacy-max):
```json
{
  "node_id": "proxy-eu-west-2-003",
  "interval": "2024-01-15T10:00:00Z/PT1M",
  "metrics": {
    "requests": 1523,
    "tokens_generated": 45690,
    "decisions": {
      "allow": 1489,
      "redact": 28,
      "stop": 6
    },
    "latency_p95_ms": 8.3,
    "rules_triggered": {
      "PRIN-2A-001": 12,
      "COBS-9A-002": 4
    }
  }
}
```

**Full Evidence Mode**:
```json
{
  "stream_id": "req_abc123xyz",
  "timestamp": "2024-01-15T10:05:23.451Z",
  "node_id": "proxy-eu-west-2-003",
  "decision": {
    "rule_id": "advice_boundary_FCA",
    "action": "inject_disclaimer",
    "regulation": "FCA COBS 9A",
    "confidence": 0.87,
    "latency_ms": 6.2
  },
  "context_hash": "sha256:...",  // No raw text
  "policy_bundle": "abc123",
  "model_versions": {
    "advice_detector": "v2.1.0"
  },
  "hash_chain": "previous:def456,current:abc789"
}
```

### Deployment

**Control Plane** (managed SaaS):
- Hosted by CheckStream on AWS/GCP/Azure
- Regional deployments for data residency
- SOC 2, ISO 27001 certified

**Nodes** (customer infrastructure):

```bash
# Install agent
curl -sSL https://install.checkstream.ai | bash

# Authenticate
checkstream login --org acme-bank

# Deploy proxy
checkstream deploy proxy \
  --region eu-west-2 \
  --backend https://api.anthropic.com/v1 \
  --policy-sync auto

# Deploy sidecar
checkstream deploy sidecar \
  --vllm-socket /shared/checkstream.sock \
  --policy-sync auto
```

**Configuration**:
```yaml
# /etc/checkstream/agent.yaml
control_plane:
  endpoint: https://control.checkstream.ai
  org_id: acme-bank
  region: eu-west-2
  auth:
    mode: api_key  # or mTLS
    key_file: /etc/checkstream/api_key

sync:
  policies:
    enabled: true
    interval: 30s
  models:
    enabled: true
    check_interval: 3600s
    auto_update: true
  telemetry:
    mode: aggregate  # or full_evidence
    batch_size: 100
    flush_interval: 60s

local_cache:
  policies: /var/lib/checkstream/policies
  models: /var/lib/checkstream/models
```

### Advantages

- **Unified governance**: Single pane of glass for all nodes
- **Version control**: Git-backed policy history
- **Compliance reporting**: Automated evidence packs
- **Multi-region**: Consistent policies across deployments
- **Audit trail**: Cryptographic chain of decisions
- **Observability**: Real-time dashboards and alerts

### Limitations

- **SaaS dependency**: Requires connectivity to control plane (for updates, not traffic)
- **Cost**: Subscription fee beyond open-source nodes
- **Complexity**: More moving parts than standalone modes

### When to Use

- **Enterprise scale**: >10 nodes across regions
- **Regulatory requirements**: Need centralized audit evidence
- **Multi-tenant**: Different policies per customer/department
- **Governance**: Approval workflows, change tracking
- **Observability**: Centralized metrics and alerts

---

## Migration Path

### Phase 1: Start with Proxy

```bash
# Deploy standalone proxy
docker run -d checkstream/proxy:latest \
  -v ./policies:/etc/checkstream/policies \
  -e BACKEND_URL=https://api.openai.com/v1
```

- Test policies with real traffic
- Tune thresholds and holdback sizes
- Validate latency acceptable

### Phase 2: Add Control Plane

```bash
# Connect proxy to control plane
checkstream login --org your-org
checkstream attach proxy-instance-1 \
  --sync-policies \
  --telemetry aggregate
```

- Centralize policy management
- Get dashboards and alerts
- Enable multi-region deployments

### Phase 3: Deploy Sidecar for Critical Paths

```bash
# For sensitive flows (investment advice)
checkstream deploy sidecar \
  --vllm-socket /shared/checkstream.sock \
  --policy-pack fca-consumer-duty-strict
```

- Preventive safety for high-risk scenarios
- Lower latency for production workloads
- Stronger guarantees for compliance

---

## Decision Matrix

| Requirement | Recommended Mode |
|-------------|------------------|
| Quick POC with OpenAI | **Proxy** |
| Multi-cloud LLM APIs | **Proxy** |
| Self-hosted vLLM models | **Sidecar** |
| Financial/healthcare compliance | **Sidecar** + **Control Plane** |
| Need <5ms overhead | **Sidecar** |
| >10 deployments across regions | **Control Plane** |
| Preventive safety (logit masking) | **Sidecar** |
| Model-agnostic (OpenAI + Anthropic + Bedrock) | **Proxy** + **Control Plane** |
| Centralized audit dashboards | **Control Plane** |
| Maximum data sovereignty | **Proxy/Sidecar** (no control plane) |

---

## Next Steps

- **Start quickly**: [Getting Started](getting-started.md)
- **Write policies**: [Policy Engine](policy-engine.md)
- **Understand control plane**: [Control Plane](control-plane.md)
- **Review security**: [Security & Privacy](security-privacy.md)
