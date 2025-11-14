# CheckStream Design Principles

**Core Philosophy**: CheckStream is **provider-agnostic, deployment-agnostic, and use-case-agnostic**.

---

## 1. Provider Agnosticism

### Principle: Backend Independence

CheckStream **does not care** what LLM backend you use. It works with:

- ✅ **OpenAI** (GPT-4, GPT-3.5)
- ✅ **Anthropic** (Claude)
- ✅ **Google** (Gemini)
- ✅ **AWS Bedrock**
- ✅ **Azure OpenAI**
- ✅ **Self-hosted** (vLLM, Ollama, LM Studio)
- ✅ **Custom APIs**

### How It Works

```
┌─────────────────────────────────────────────────────────┐
│                    Your Application                      │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ Standard API calls
                     │ (OpenAI format, Anthropic format, etc.)
                     ↓
┌─────────────────────────────────────────────────────────┐
│                  CheckStream Proxy                       │
│                                                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐    │
│  │  Phase 1:   │  │   Phase 2:   │  │  Phase 3:   │    │
│  │  Ingress    │→ │  Midstream   │→ │   Egress    │    │
│  │  (Validate) │  │  (Stream)    │  │  (Audit)    │    │
│  └─────────────┘  └──────────────┘  └─────────────┘    │
│                                                           │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ Forward to configured backend
                     │
        ┌────────────┼────────────┬─────────────┐
        │            │            │             │
        ↓            ↓            ↓             ↓
   ┌────────┐  ┌─────────┐  ┌─────────┐  ┌──────────┐
   │ OpenAI │  │ Claude  │  │ Gemini  │  │  vLLM    │
   │   API  │  │   API   │  │   API   │  │ (local)  │
   └────────┘  └─────────┘  └─────────┘  └──────────┘
```

### Configuration Example

**Change backend** by just updating config:

```yaml
# Use OpenAI
backend_url: "https://api.openai.com/v1"

# Use Anthropic
backend_url: "https://api.anthropic.com/v1"

# Use Azure OpenAI
backend_url: "https://your-resource.openai.azure.com"

# Use local vLLM
backend_url: "http://localhost:8000/v1"

# Use custom API
backend_url: "https://your-custom-llm.com/api"
```

**No code changes required.** CheckStream just proxies through.

### Why This Matters

1. **Vendor Lock-in Prevention**: Switch providers anytime
2. **Multi-Provider**: Use different providers for different models
3. **Cost Optimization**: Route to cheapest provider
4. **Failover**: If OpenAI is down, switch to Anthropic
5. **Testing**: Test locally with Ollama, deploy with OpenAI

### Implementation

CheckStream achieves this by:

```rust
// Generic HTTP client - works with any backend
let backend_response = state.http_client
    .post(&state.config.backend_url)  // Any URL
    .header("Authorization", auth_header)  // Pass through auth
    .json(&req)  // Forward original request
    .send()
    .await?;

// We don't parse or depend on backend-specific features
// Just proxy the response through our safety layers
```

**Key**: We operate on **text content**, not provider-specific formats.

---

## 2. Deployment Agnosticism

### Principle: Run Anywhere

CheckStream **does not care** how you deploy it:

- ✅ **Standalone binary** (single server)
- ✅ **Docker container**
- ✅ **Kubernetes cluster**
- ✅ **AWS ECS/Fargate**
- ✅ **Google Cloud Run**
- ✅ **Azure Container Apps**
- ✅ **Embedded** (library mode)
- ✅ **Sidecar** (next to your app)
- ✅ **Gateway** (centralized proxy)

### Deployment Modes

#### Mode 1: Standalone Proxy
```
┌──────────┐      ┌─────────────┐      ┌─────────┐
│   App    │ ───→ │ CheckStream │ ───→ │ OpenAI  │
└──────────┘      └─────────────┘      └─────────┘
                   (Port 8080)
```

```bash
./checkstream-proxy --config config.yaml --port 8080
```

#### Mode 2: Kubernetes Sidecar
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-app
spec:
  containers:
  # Your app
  - name: app
    image: my-app:latest
    env:
      - name: OPENAI_API_BASE
        value: "http://localhost:8080/v1"  # Use sidecar

  # CheckStream sidecar
  - name: checkstream
    image: checkstream-proxy:latest
    ports:
      - containerPort: 8080
```

**Your app talks to `localhost:8080`**, CheckStream forwards to real API.

#### Mode 3: Gateway (Shared)
```
┌──────────┐
│  App 1   │ ───┐
└──────────┘    │
                │     ┌─────────────┐      ┌─────────┐
┌──────────┐    ├───→ │ CheckStream │ ───→ │ OpenAI  │
│  App 2   │ ───┤     │  (Gateway)  │      └─────────┘
└──────────┘    │     └─────────────┘
                │
┌──────────┐    │
│  App 3   │ ───┘
└──────────┘
```

Multiple apps share one CheckStream instance.

#### Mode 4: Library (Embedded)
```rust
// Future: Use CheckStream as a library in your Rust app
use checkstream::SafetyLayer;

let safety = SafetyLayer::new(config)?;
let response = safety.safe_complete(prompt).await?;
```

### Why This Matters

1. **Flexibility**: Choose deployment based on your needs
2. **Scale**: Start small (single server), scale up (K8s)
3. **Cost**: Sidecar = isolation, Gateway = shared resources
4. **Latency**: Sidecar = localhost speed, Gateway = network hop

---

## 3. Protocol Agnosticism

### Principle: Any Request/Response Format

CheckStream **does not care** about the API format:

- ✅ **OpenAI format** (chat completions)
- ✅ **Anthropic format** (messages API)
- ✅ **Custom formats** (adapt as needed)

### How It Works

**Phase 1 & 2 operate on extracted text**:

```rust
// Extract text from ANY format
let text = match request_format {
    Format::OpenAI => extract_from_openai(&request),
    Format::Anthropic => extract_from_anthropic(&request),
    Format::Custom => extract_from_custom(&request),
};

// Run safety checks on the TEXT (format-agnostic)
let result = pipeline.execute(&text).await?;

// Forward original request (unchanged format)
let response = forward_to_backend(original_request).await?;
```

**We only care about the content**, not the wrapper.

### Example: Supporting Multiple Formats

```rust
// OpenAI format
{
  "messages": [
    {"role": "user", "content": "Hello"}  // ← Extract this
  ]
}

// Anthropic format
{
  "messages": [
    {"role": "user", "content": "Hello"}  // ← Extract this
  ]
}

// Custom format
{
  "prompt": "Hello"  // ← Extract this
}
```

CheckStream extracts the **content**, runs checks, forwards request.

---

## 4. Use Case Agnosticism

### Principle: Any Application Domain

CheckStream **does not care** what you're building:

- ✅ **Customer support chatbots**
- ✅ **Financial advisory apps**
- ✅ **Healthcare assistants**
- ✅ **Legal document generation**
- ✅ **Education platforms**
- ✅ **Gaming (NPC dialogue)**
- ✅ **Code generation tools**
- ✅ **Content moderation**

### How It Works: Configurable Pipelines

Each use case defines **its own pipelines**:

#### Financial Services
```yaml
pipelines:
  ingress_pipeline: "financial-compliance"
  # Checks: Financial advice detection, risk disclosure

  midstream_pipeline: "financial-monitoring"
  # Checks: Personalized recommendations, regulatory violations

  egress_pipeline: "financial-audit"
  # Generates: FCA compliance audit trail
```

#### Healthcare
```yaml
pipelines:
  ingress_pipeline: "healthcare-safety"
  # Checks: Medical advice requests, PHI detection

  midstream_pipeline: "healthcare-monitoring"
  # Checks: Diagnoses, treatment recommendations

  egress_pipeline: "hipaa-compliance"
  # Generates: HIPAA audit trail
```

#### Content Moderation
```yaml
pipelines:
  ingress_pipeline: "content-safety"
  # Checks: Toxic prompts, NSFW requests

  midstream_pipeline: "content-filtering"
  # Checks: Hate speech, violence, adult content

  egress_pipeline: "content-audit"
  # Generates: Moderation logs
```

**Same system, different configurations.**

---

## 5. Model Agnosticism

### Principle: Any ML Model

CheckStream **does not care** what models you use:

- ✅ **HuggingFace models**
- ✅ **Custom fine-tuned models**
- ✅ **Regex patterns** (no ML)
- ✅ **API-based classifiers** (external services)
- ✅ **Rule-based systems**
- ✅ **Hybrid approaches**

### Example: Mix and Match

```yaml
classifiers:
  # Regex-based (no model)
  pii:
    type: pattern
    patterns:
      - "\\b\\d{3}-\\d{2}-\\d{4}\\b"  # SSN

  # ML model from HuggingFace
  toxicity:
    type: ml
    source:
      repo: "unitary/toxic-bert"

  # API-based (external service)
  custom-compliance:
    type: api
    endpoint: "https://your-api.com/classify"

  # Custom logic
  business-rules:
    type: custom
    implementation: "YourCustomClassifier"
```

**CheckStream orchestrates**, you provide the classifiers.

---

## 6. Stream Format Agnosticism

### Principle: Any Streaming Protocol

CheckStream **does not care** how the LLM streams:

- ✅ **Server-Sent Events (SSE)** (most common)
- ✅ **WebSockets**
- ✅ **gRPC streams**
- ✅ **HTTP chunked transfer**
- ✅ **Custom protocols**

### How It Works

```rust
// Abstract over streaming protocol
let stream = match backend_protocol {
    Protocol::SSE => handle_sse_stream(response),
    Protocol::WebSocket => handle_ws_stream(response),
    Protocol::GRPC => handle_grpc_stream(response),
};

// Process chunks (protocol-agnostic)
for chunk in stream {
    let result = midstream_pipeline.execute(&chunk).await?;
    if result.should_redact() {
        send("[REDACTED]");
    } else {
        send(chunk);
    }
}
```

**We operate on chunks**, not protocol-specific formats.

---

## 7. Data Residency Agnosticism

### Principle: Your Data, Your Infrastructure

CheckStream **does not send data anywhere**:

- ✅ **All processing is local** (on your servers)
- ✅ **No external API calls** (except to configured LLM)
- ✅ **No telemetry to CheckStream servers** (we don't have any!)
- ✅ **Your compliance requirements** (GDPR, HIPAA, SOC2)

### Data Flow

```
User Data
    ↓
CheckStream (YOUR infrastructure)
    ├─ Classifiers run locally
    ├─ Metrics stored locally (Prometheus)
    └─ Audit logs stored locally
    ↓
Configured LLM Backend (YOUR choice)
    ↓
Response
    ↓
CheckStream (YOUR infrastructure)
    ↓
User
```

**Your data never leaves your control** (except to the LLM you chose).

---

## 8. Configuration Agnosticism

### Principle: Configure, Don't Code

CheckStream **does not require code changes** for:

- ✅ Changing providers
- ✅ Updating thresholds
- ✅ Adding classifiers
- ✅ Modifying pipelines
- ✅ Adjusting policies

### Everything is YAML

```yaml
# config.yaml
backend_url: "https://api.openai.com/v1"  # Change provider here

pipelines:
  safety_threshold: 0.7   # Adjust threshold here
  chunk_threshold: 0.8

  ingress_pipeline: "custom-pipeline"  # Reference your pipeline

# classifiers.yaml
pipelines:
  custom-pipeline:  # Define your pipeline here
    stages:
      - type: parallel
        classifiers: [your-classifier]
```

**Configuration-driven**, not code-driven.

---

## Summary: The Agnostic Architecture

### What CheckStream IS

✅ **A safety layer** that sits between your app and any LLM
✅ **A pipeline executor** that runs configurable classifiers
✅ **A streaming proxy** that works with any protocol
✅ **A compliance engine** that generates audit trails

### What CheckStream IS NOT

❌ **Not tied to OpenAI** (works with any provider)
❌ **Not a SaaS** (runs on your infrastructure)
❌ **Not a specific use case** (configurable for any domain)
❌ **Not opinionated about models** (use any classifiers)
❌ **Not locked to a deployment** (run anywhere)

### The Core Abstraction

```
CheckStream operates on THREE primitives:

1. TEXT (input)
   ↓
2. CLASSIFICATION (score + decision)
   ↓
3. ACTION (allow/block/redact)
```

**Everything else is configuration.**

### Why This Matters

1. **Future-proof**: New LLM providers? Just update config.
2. **Flexible**: Different use cases? Different pipelines.
3. **Portable**: Deploy anywhere (cloud, on-prem, edge).
4. **Compliant**: Your data stays in your infrastructure.
5. **Cost-effective**: Switch providers for best pricing.
6. **Vendor-independent**: No lock-in to any provider.

### Design Mantra

> **"CheckStream doesn't care about your backend, deployment, or use case.
> It just makes your LLM applications safer, regardless of how you build them."**

---

## Practical Implications

### For Developers

**You can**:
- Use CheckStream with OpenAI today
- Switch to Anthropic tomorrow
- Deploy on AWS, then move to GCP
- Start with toxicity, add compliance later
- Run in Docker, migrate to K8s

**Without**:
- Changing CheckStream code
- Rewriting your application
- Losing audit history
- Breaking compliance

### For Organizations

**You can**:
- Meet compliance requirements (GDPR, HIPAA, FCA)
- Use preferred LLM vendors
- Deploy in required regions (data residency)
- Customize for your industry
- Scale from prototype to production

**Without**:
- Vendor lock-in
- Infrastructure constraints
- Compliance violations
- Massive refactoring

---

## Future Agnosticism

As CheckStream evolves, we maintain agnosticism:

### Planned Features (Still Agnostic)

✅ **Multi-provider support** - Route to cheapest provider
✅ **Policy engine** - Map business rules to any pipeline
✅ **Control plane** - Manage fleet, any deployment
✅ **More protocols** - gRPC, WebSocket, etc.

### NOT Planned (Would Break Agnosticism)

❌ **OpenAI-specific optimizations** - Stay generic
❌ **Vendor-specific features** - No lock-in
❌ **SaaS-only mode** - Always self-hostable
❌ **Hardcoded use cases** - Always configurable

---

## Verification Checklist

Before any feature is added, we ask:

- [ ] **Provider Agnostic?** Works with any LLM backend?
- [ ] **Deployment Agnostic?** Runs in any environment?
- [ ] **Use Case Agnostic?** Configurable for any domain?
- [ ] **Model Agnostic?** Accepts any classifier?
- [ ] **Protocol Agnostic?** Handles any streaming format?
- [ ] **Data Agnostic?** Processes locally, no external calls?
- [ ] **Config-Driven?** No code changes required?

**If any answer is "No", reconsider the design.**

---

## See Also

- [Architecture Overview](architecture.md) - System architecture
- [Deployment Modes](deployment-modes.md) - How to deploy
- [Pipeline Configuration](pipeline-configuration.md) - How to configure
- [Provider Integration](PROVIDER_INTEGRATION.md) - Supporting new providers

---

**Last Updated**: 2025-11-14
**Version**: 0.1.0
