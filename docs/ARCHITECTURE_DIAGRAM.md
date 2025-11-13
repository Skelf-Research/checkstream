# CheckStream Pipeline Architecture

Visual reference for the classifier pipeline system architecture.

## System Overview

```
┌────────────────────────────────────────────────────────────────┐
│                     CheckStream Proxy                          │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                  HTTP Request Handler                     │ │
│  │                                                            │ │
│  │  1. Receive Request                                       │ │
│  │  2. Extract Text                                          │ │
│  │  3. Select Pipeline ────────────────────┐                 │ │
│  │  4. Execute Pipeline                    │                 │ │
│  │  5. Check Decision                      │                 │ │
│  │  6. Block or Forward                    │                 │ │
│  └──────────────────┬───────────────────────┼────────────────┘ │
│                     │                       │                  │
│                     v                       v                  │
│  ┌──────────────────────────┐   ┌─────────────────────────┐   │
│  │    Streaming Handler     │   │   Application State     │   │
│  │                          │   │                         │   │
│  │  • Per-chunk checks      │   │  • Config              │   │
│  │  • Fast pipelines        │   │  • Classifiers         │   │
│  │  • Redaction             │   │  • Pipelines           │   │
│  └──────────────────────────┘   │  • Model Registry      │   │
│                                  └────────┬────────────────┘   │
└─────────────────────────────────────────┼─────────────────────┘
                                           │
                                           v
┌────────────────────────────────────────────────────────────────┐
│              Classifier Pipeline System (New!)                 │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                  Pipeline Executor                        │ │
│  │                                                            │ │
│  │  Input: "Check this text"                                │ │
│  │         ↓                                                  │ │
│  │  ┌─────────────────────────────────────────────┐         │ │
│  │  │  Stage 1: Parallel Execution                │         │ │
│  │  │                                               │         │ │
│  │  │    ┌──────────┐  ┌──────────┐  ┌─────────┐ │         │ │
│  │  │    │Toxicity  │  │Sentiment │  │   PII   │ │         │ │
│  │  │    │ (4ms)    │  │  (3ms)   │  │ (2ms)   │ │         │ │
│  │  │    └────┬─────┘  └────┬─────┘  └────┬────┘ │         │ │
│  │  │         └───────────┬─────────────────┘      │         │ │
│  │  │                     v                        │         │ │
│  │  │         [Aggregation: max_score]             │         │ │
│  │  │                 Result: 0.9                  │         │ │
│  │  └───────────────────┬───────────────────────────┘         │ │
│  │                      │                                      │ │
│  │  ┌─────────────────────────────────────────────┐         │ │
│  │  │  Stage 2: Conditional (if score > 0.5)      │         │ │
│  │  │                                               │         │ │
│  │  │    Condition: ✓ Triggered                    │         │ │
│  │  │         ↓                                     │         │ │
│  │  │    ┌──────────────────┐                      │         │ │
│  │  │    │ Deep Analysis    │                      │         │ │
│  │  │    │     (8ms)        │                      │         │ │
│  │  │    └─────────┬────────┘                      │         │ │
│  │  │              v                                │         │ │
│  │  │         Result: 0.95                          │         │ │
│  │  └───────────────┬───────────────────────────────┘         │ │
│  │                  │                                          │ │
│  │                  v                                          │ │
│  │  ┌─────────────────────────────────────────────┐         │ │
│  │  │  Final Decision                              │         │ │
│  │  │                                               │         │ │
│  │  │  Score: 0.95                                 │         │ │
│  │  │  Label: "positive"                           │         │ │
│  │  │  Latency: 12.1ms                             │         │ │
│  │  │  Action: BLOCK                               │         │ │
│  │  └─────────────────────────────────────────────┘         │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

## Pipeline Configuration Flow

```
┌─────────────────┐
│ classifiers.yaml│
│                 │
│ models:         │
│   toxicity: ... │
│   sentiment: ...│
│                 │
│ pipelines:      │
│   my-pipeline:  │
│     stages:     │
│       - ...     │
└────────┬────────┘
         │
         v
┌─────────────────────────┐
│  load_config()          │
│  ↓                      │
│  ClassifierConfig       │
└────────┬────────────────┘
         │
         v
┌─────────────────────────────────┐
│  init_registry_from_config()    │
│  ↓                               │
│  ModelRegistry                   │
│    - Load models from HF/local  │
│    - Initialize on device       │
└────────┬────────────────────────┘
         │
         v
┌─────────────────────────────────┐
│  build_classifiers()            │
│  ↓                               │
│  HashMap<String, Classifier>    │
│    - toxicity                   │
│    - sentiment                  │
│    - pii                        │
│    - ...                        │
└────────┬────────────────────────┘
         │
         v
┌──────────────────────────────────┐
│  build_pipeline_from_config()   │
│  ↓                                │
│  ClassifierPipeline              │
│    - Stages configured           │
│    - Classifiers connected       │
│    - Ready to execute            │
└──────────────────────────────────┘
```

## Stage Type Architectures

### Single Stage
```
Input Text
    ↓
┌──────────────┐
│  Classifier  │
└──────┬───────┘
       │
       v
    Result
```

### Parallel Stage
```
                Input Text
                    ↓
        ┌───────────┼───────────┐
        v           v           v
   ┌─────────┐ ┌─────────┐ ┌─────────┐
   │Classify │ │Classify │ │Classify │
   │   A     │ │   B     │ │   C     │
   └────┬────┘ └────┬────┘ └────┬────┘
        │           │           │
        └───────────┼───────────┘
                    v
            ┌───────────────┐
            │  Aggregation  │
            │  max_score    │
            └───────┬───────┘
                    v
                 Result
```

### Sequential Stage
```
Input Text
    ↓
┌──────────────┐
│ Classifier A │
└──────┬───────┘
       │
       v
┌──────────────┐
│ Classifier B │
└──────┬───────┘
       │
       v
┌──────────────┐
│ Classifier C │
└──────┬───────┘
       │
       v
  [Results A, B, C]
```

### Conditional Stage
```
Previous Results
    ↓
┌──────────────────┐
│ Check Condition  │
│ (score > 0.5?)   │
└────┬─────────┬───┘
     │         │
  Yes│         │No
     v         v
┌──────────┐  Skip (5μs)
│Classifier│
└────┬─────┘
     │
     v
  Result
```

## Aggregation Strategies

### Max Score
```
Results: [0.3, 0.9, 0.5]
         ↓
  Find Maximum
         ↓
    Return 0.9
```

### Unanimous
```
Results: [0.8, 0.9, 0.7]
Threshold: 0.5
         ↓
All > 0.5? ✓ Yes
         ↓
Return: Positive
```

### Weighted Average
```
Results: [0.3, 0.9, 0.6]
         ↓
  Sum / Count
         ↓
  0.6 average
```

## Data Flow Example

```
1. Request arrives
   ↓
2. Extract: "This is toxic content"
   ↓
3. Load pipeline: "basic-safety"
   ↓
4. Execute Stage 1 (Parallel)
   ├─ Toxicity:  0.95 (4ms)
   ├─ Sentiment: 0.85 (3ms)
   └─ PII:       0.10 (2ms)
   ↓
5. Aggregate (max_score)
   → 0.95
   ↓
6. Check threshold (0.7)
   → 0.95 > 0.7 ✓
   ↓
7. Decision: BLOCK
   ↓
8. Return 400 error to client
```

## Performance Flow

```
┌─────────────────────────────────────────┐
│            Latency Budget               │
│                                         │
│  Target: < 10ms total                   │
│                                         │
│  ┌────────────────────────────────┐    │
│  │ Parallel Stage                 │    │
│  │ Latency = max(4ms, 3ms, 2ms)  │    │
│  │         = 4ms                  │    │
│  └────────────────────────────────┘    │
│             + 0.05ms (overhead)         │
│                                         │
│  ┌────────────────────────────────┐    │
│  │ Conditional Stage (if needed)  │    │
│  │ Latency = 0.005ms (skip)       │    │
│  │       or 8ms (execute)         │    │
│  └────────────────────────────────┘    │
│                                         │
│  Total: ~4.055ms (fast path) ✓         │
│      or ~12.055ms (deep path) ⚠        │
│                                         │
└─────────────────────────────────────────┘
```

## Classifier Tiers

```
┌──────────────────────────────────────────┐
│            Classifier Tiers              │
│                                          │
│  Tier A (<2ms) - Pattern Matching       │
│  ┌────────────────────────────────────┐ │
│  │ • Regex patterns                    │ │
│  │ • PII detection (Aho-Corasick)     │ │
│  │ • Simple string matching            │ │
│  └────────────────────────────────────┘ │
│                                          │
│  Tier B (<5ms) - Quantized ML           │
│  ┌────────────────────────────────────┐ │
│  │ • Toxicity (quantized BERT)        │ │
│  │ • Sentiment (DistilBERT)           │ │
│  │ • Prompt injection detection       │ │
│  └────────────────────────────────────┘ │
│                                          │
│  Tier C (<10ms) - Full Models           │
│  ┌────────────────────────────────────┐ │
│  │ • Financial advice detection       │ │
│  │ • Nuanced classification           │ │
│  │ • Custom domain models             │ │
│  └────────────────────────────────────┘ │
└──────────────────────────────────────────┘
```

## Error Handling Flow

```
Execute Pipeline
       ↓
   Try Main
       ↓
    ┌──┴──┐
    │     │
Success  Timeout/Error
    │     │
    │     v
    │  Try Fallback Pipeline
    │     │
    │  ┌──┴──┐
    │  │     │
    │ Success Error
    │  │     │
    │  │     v
    │  │  Try Simple Classifier
    │  │     │
    │  │  ┌──┴──┐
    │  │  │     │
    │  │ Success Error
    │  │  │     │
    └──┴──┘     v
       │    Check fail_open
       │        │
       │     ┌──┴──┐
       │  Yes│   │No
       │     │   │
       v     v   v
    Return  Pass Block
```

## Metrics Collection

```
Pipeline Execution
       ↓
   Record Start
       ↓
  Execute Stages
       ↓
  ┌────┴────┐
  │         │
Stage 1   Stage 2
  │         │
  v         v
Record    Record
Metrics   Metrics
  │         │
  └────┬────┘
       v
Record Total Latency
       ↓
Record Final Decision
       ↓
Export to Prometheus
       ↓
   ┌──────────────────┐
   │ Dashboards       │
   │ • Latency p50/99 │
   │ • Decision rates │
   │ • Error rates    │
   └──────────────────┘
```

## Configuration Inheritance

```
classifiers.yaml
┌─────────────────────────────────┐
│ default_device: cpu             │───┐
│ default_quantize: true          │   │
│                                 │   │ Inherited by
│ models:                         │   │ all models
│   toxicity:                     │◄──┘
│     device: cpu (inherited)     │
│     quantize: true (inherited)  │
│                                 │
│   sentiment:                    │
│     device: cuda (override)     │
│     quantize: false (override)  │
└─────────────────────────────────┘
```

## Complete System Diagram

```
┌───────────────────────────────────────────────────────────────┐
│                     CheckStream Platform                       │
│                                                                │
│  ┌──────────────┐                                             │
│  │   Client     │                                             │
│  │  (OpenAI     │                                             │
│  │  SDK format) │                                             │
│  └──────┬───────┘                                             │
│         │ HTTP/SSE                                            │
│         v                                                      │
│  ┌──────────────────────────────────────────────────────────┐│
│  │              CheckStream Proxy                           ││
│  │  ┌────────────────────────────────────────────────────┐ ││
│  │  │ Request Handler                                     │ ││
│  │  │ • Extract prompt                                    │ ││
│  │  │ • Select pipeline                                   │ ││
│  │  │ • Execute checks ──────────┐                        │ ││
│  │  └────────────────────────────┼────────────────────────┘ ││
│  │                                │                          ││
│  │  ┌────────────────────────────┼────────────────────────┐ ││
│  │  │ Pipeline System (New!)     │                        │ ││
│  │  │                            v                        │ ││
│  │  │  ┌─────────────────────────────────┐               │ ││
│  │  │  │  Parallel: [Toxicity, PII]     │               │ ││
│  │  │  │  Conditional: Deep Analysis     │               │ ││
│  │  │  │  Aggregation: max_score         │               │ ││
│  │  │  └────────────┬────────────────────┘               │ ││
│  │  │               v                                     │ ││
│  │  │         Decision: 0.85                             │ ││
│  │  └──────────────────┬──────────────────────────────────┘ ││
│  │                     │                                    ││
│  │                ┌────┴────┐                              ││
│  │                │         │                              ││
│  │              PASS      BLOCK                            ││
│  │                │         │                              ││
│  └────────────────┼─────────┼──────────────────────────────┘│
│                   │         │                               │
│                   v         v                               │
│            ┌────────┐   ┌────────┐                         │
│            │Forward │   │Return  │                         │
│            │to LLM  │   │ 400    │                         │
│            └────────┘   └────────┘                         │
└───────────────────────────────────────────────────────────────┘
```

---

For more details, see:
- [Pipeline Configuration](pipeline-configuration.md)
- [Integration Guide](INTEGRATION_GUIDE.md)
- [Quick Start](QUICKSTART_PIPELINES.md)
