# Pipeline Quick Start Guide

Get started with CheckStream classifier pipelines in 5 minutes.

## 1. Define Your Pipeline (YAML)

Edit `classifiers.yaml`:

```yaml
pipelines:
  my-safety-check:
    description: "Multi-stage safety validation"
    stages:
      # Stage 1: Fast parallel checks
      - type: parallel
        name: quick-scan
        classifiers:
          - toxicity
          - sentiment
        aggregation: max_score

      # Stage 2: Deep analysis if needed
      - type: conditional
        name: deep-check
        classifier: advanced-toxicity
        condition:
          any_above_threshold:
            threshold: 0.5
```

## 2. Load and Build Pipeline (Rust)

```rust
use checkstream_classifiers::{load_config, build_pipeline_from_config};
use std::collections::HashMap;
use std::sync::Arc;

// Load config
let config = load_config("./classifiers.yaml")?;

// Get your pipeline spec
let pipeline_spec = config.get_pipeline("my-safety-check")?;

// Create your classifiers
let mut classifiers = HashMap::new();
classifiers.insert("toxicity".to_string(), Arc::new(my_toxicity_classifier));
classifiers.insert("sentiment".to_string(), Arc::new(my_sentiment_classifier));

// Build the pipeline
let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers)?;
```

## 3. Execute

```rust
// Run pipeline on input text
let result = pipeline.execute("Check this message").await?;

// Check results
if let Some(decision) = result.final_decision {
    println!("Score: {}", decision.score);
    println!("Latency: {}μs", result.total_latency_us);

    if decision.score > 0.7 {
        println!("⚠️ FLAGGED");
    }
}
```

## Common Patterns

### Pattern 1: Parallel Fast Checks

```yaml
fast-check:
  stages:
    - type: parallel
      name: scan
      classifiers: [toxicity, pii, prompt-injection]
      aggregation: max_score  # Flag if ANY triggers
```

**Use when**: Multiple independent checks, need speed

**Latency**: ~max(classifier_latencies)

### Pattern 2: Progressive Depth

```yaml
progressive:
  stages:
    - type: single
      name: quick-filter
      classifier: fast-toxicity

    - type: conditional
      name: deep-analysis
      classifier: advanced-toxicity
      condition:
        any_above_threshold:
          threshold: 0.3
```

**Use when**: Want to save compute, expensive checks

**Latency**: Fast path ~2ms, deep path ~7ms

### Pattern 3: Consensus

```yaml
high-confidence:
  stages:
    - type: parallel
      name: ensemble
      classifiers: [model-a, model-b, model-c]
      aggregation: unanimous  # All must agree
```

**Use when**: High-stakes decisions

**Latency**: ~max(classifier_latencies)

## Stage Types Reference

| Type | When to Use | Example |
|------|-------------|---------|
| `single` | One classifier | Initial triage |
| `parallel` | Independent checks | Multi-model validation |
| `sequential` | Order matters | Multi-step analysis |
| `conditional` | Expensive fallback | Deep check if triggered |

## Aggregation Strategies

| Strategy | Behavior | Use Case |
|----------|----------|----------|
| `max_score` | Highest score wins | Safety (any flag = action) |
| `min_score` | Lowest score wins | Conservative approach |
| `unanimous` | All must agree | High confidence needed |
| `weighted_average` | Average scores | Balanced decision |
| `first_positive` | Stop at first hit | Fast-fail |

## Conditions Reference

```yaml
# Execute if ANY previous result > threshold
condition:
  any_above_threshold:
    threshold: 0.5

# Execute if ALL previous results > threshold
condition:
  all_above_threshold:
    threshold: 0.3

# Execute if specific classifier triggered
condition:
  classifier_triggered:
    classifier: toxicity

# Always execute
condition: always
```

## Performance Tips

### ✅ DO

```yaml
# Parallel independent checks
- type: parallel
  classifiers: [a, b, c]
  aggregation: max_score

# Fast classifiers first
- type: single
  classifier: quick-check  # <2ms
- type: conditional
  classifier: slow-check   # <10ms, only if needed
```

### ❌ DON'T

```yaml
# Sequential when parallel would work
- type: sequential
  classifiers: [a, b, c]  # Wastes time if independent

# No gating for expensive operations
- type: single
  classifier: very-expensive  # Always runs
```

## Debugging

### Print Detailed Results

```rust
let result = pipeline.execute(text).await?;

for stage in &result.results {
    println!("{}: {} ({}μs, score: {:.2})",
        stage.stage_name,
        stage.classifier_name,
        stage.stage_latency_us,
        stage.result.score
    );
}
```

### Check Total Latency

```rust
if result.total_latency_us > 10_000 {
    eprintln!("⚠️ Pipeline exceeded 10ms target!");
}
```

## Running the Example

```bash
# See full working example
cargo run --example pipeline_usage

# Output shows:
# - Pipeline configuration loading
# - Execution with different inputs
# - Latency measurements
# - Decision outcomes
```

## Next Steps

- 📖 Full guide: [`docs/pipeline-configuration.md`](pipeline-configuration.md)
- 🔧 Configuration: [`docs/classifier-configuration.md`](classifier-configuration.md)
- 💻 Example code: [`examples/pipeline_usage.rs`](../examples/pipeline_usage.rs)
- 📝 Config file: [`classifiers.yaml`](../classifiers.yaml)

## Common Issues

### "Pipeline not found"

**Problem**: `config.get_pipeline("my-pipeline")` returns None

**Solution**: Check pipeline name in YAML, ensure it's under `pipelines:` section

### "Classifier not found for stage"

**Problem**: Referenced classifier doesn't exist

**Solution**: Verify classifier is in your HashMap and name matches exactly

### High latency

**Problem**: Pipeline exceeds latency budget

**Solutions**:
1. Use `parallel` instead of `sequential` where possible
2. Add `conditional` gates for expensive classifiers
3. Use faster model variants (distilled, quantized)
4. Check individual classifier latencies

## Support

- Issues: https://github.com/yourusername/checkstream/issues
- Docs: [`docs/`](.)
- Examples: [`examples/`](../examples/)
