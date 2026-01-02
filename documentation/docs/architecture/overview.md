# Architecture Overview

CheckStream is designed as a high-performance, modular guardrail platform for real-time LLM safety enforcement.

---

## Design Principles

1. **Sub-10ms Latency**: Safety checks must not degrade user experience
2. **Streaming-First**: Work with token streams, not just complete responses
3. **Configuration-Driven**: Change behavior via YAML, not code
4. **Defense in Depth**: Multiple layers of protection
5. **Production-Ready**: Observable, reliable, and scalable

---

## System Architecture

```
                                    CheckStream Proxy
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌──────────┐   │
│  │   Ingress   │     │  Midstream  │     │   Egress    │     │  Audit   │   │
│  │   Phase     │────▶│   Phase     │────▶│   Phase     │────▶│  Trail   │   │
│  │  (Pre-Gen)  │     │ (Streaming) │     │ (Post-Gen)  │     │          │   │
│  └─────────────┘     └─────────────┘     └─────────────┘     └──────────┘   │
│         │                   │                   │                  │        │
│         ▼                   ▼                   ▼                  ▼        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      Classifier Pipeline                             │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐               │   │
│  │  │ Tier A  │  │ Tier B  │  │ Tier C  │  │ Custom  │               │   │
│  │  │ Pattern │  │   ML    │  │  Full   │  │         │               │   │
│  │  │  <2ms   │  │  <5ms   │  │  <10ms  │  │         │               │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                        Policy Engine                                 │   │
│  │  Rules → Triggers → Actions → Decisions                             │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
         │                          │                           │
         ▼                          ▼                           ▼
    ┌─────────┐              ┌─────────────┐             ┌───────────┐
    │ Metrics │              │  LLM Backend │             │   Logs    │
    │Prometheus│              │   (OpenAI)   │             │  (JSON)   │
    └─────────┘              └─────────────┘             └───────────┘
```

---

## Crate Structure

CheckStream is organized as a Rust workspace with five crates:

| Crate | Purpose |
|-------|---------|
| `checkstream-core` | Core types, traits, and utilities |
| `checkstream-proxy` | HTTP/SSE proxy server and pipeline orchestration |
| `checkstream-classifiers` | ML and pattern-based classifiers |
| `checkstream-policy` | Policy engine and rule evaluation |
| `checkstream-telemetry` | Metrics, logging, and audit trail |

### Dependency Graph

```
checkstream-proxy
    ├── checkstream-core
    ├── checkstream-classifiers
    │       └── checkstream-core
    ├── checkstream-policy
    │       └── checkstream-core
    └── checkstream-telemetry
            └── checkstream-core
```

---

## Request Flow

### 1. Request Arrives

```
Client Request → Proxy Server → Route Handler
```

### 2. Ingress Phase

```
Prompt → Classifier Pipeline → Policy Engine → Decision
                                                  │
                                    ┌─────────────┴─────────────┐
                                    ▼                           ▼
                                 ALLOW                        BLOCK
                                    │                           │
                                    ▼                           ▼
                            Forward to LLM              Return Error
```

### 3. Midstream Phase (Streaming)

```
Token Stream from LLM
        │
        ▼
┌─────────────────┐
│ Holdback Buffer │◀──── 8-32 tokens buffered
│   (16 tokens)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Classifiers   │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
  SAFE     UNSAFE
    │         │
    ▼         ▼
 Release   Redact
 tokens   content
```

### 4. Egress Phase

```
Complete Response → Full Analysis → Compliance Check → Audit Record
                                          │
                                          ▼
                                    Add Disclaimers
                                    (if required)
```

---

## Key Components

### Classifier Pipeline

The classifier system is tiered for optimal latency/accuracy tradeoff:

- **Tier A** (Pattern): Regex/DFA matching in <2ms
- **Tier B** (ML): Quantized models in <5ms
- **Tier C** (Full): Large models in <10ms

See [Classifier System](classifiers.md) for details.

### Policy Engine

Declarative rules that map classifier outputs to actions:

```yaml
trigger → condition → action → result
```

Supports complex boolean logic, threshold ranges, and regulatory mapping.

### Audit Trail

Hash-chained, tamper-proof logging for compliance:

```
Record N: { data, hash(Record N-1) }
```

Enables regulatory proof of safety enforcement.

---

## Performance Architecture

### Async Everything

Built on Tokio for non-blocking I/O:

- Connection handling
- Classifier inference
- Backend requests
- Metrics collection

### Memory Efficiency

- Token streaming avoids full response buffering
- Model weights shared across requests
- Lazy classifier loading

### Concurrency

- Thread pool for CPU-bound classification
- Async task pool for I/O operations
- Lock-free metrics collection

---

## Deployment Models

### Proxy Mode (Recommended)

```
Client → CheckStream → LLM Backend
```

Centralized safety enforcement for all clients.

### Sidecar Mode

```
┌─────────────────────────┐
│       Pod               │
│  ┌──────┐  ┌─────────┐  │
│  │ vLLM │──│CheckStream│ │
│  └──────┘  └─────────┘  │
└─────────────────────────┘
```

Co-located with LLM for minimal latency.

---

## Next Steps

- [Three-Phase Pipeline](pipeline.md) - Deep dive into each phase
- [Classifier System](classifiers.md) - How classification works
- [Configuration](../configuration/proxy.md) - Configure for your use case
