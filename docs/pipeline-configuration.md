# Classifier Pipeline Configuration

CheckStream's pipeline system enables sophisticated workflows by chaining classifiers sequentially, running them in parallel, or executing them conditionally based on previous results.

## Overview

Pipelines provide:
- **Parallel Execution**: Run multiple classifiers concurrently for speed
- **Sequential Chaining**: Execute classifiers in order, with each stage seeing previous results
- **Conditional Logic**: Run expensive checks only when needed
- **Result Aggregation**: Combine outputs using various strategies (max, min, unanimous, etc.)

All pipelines are configured via YAML and built at startup for zero runtime overhead.

## Quick Start

### Basic Parallel Pipeline

```yaml
pipelines:
  basic-safety:
    description: "Quick safety check"
    stages:
      - type: parallel
        name: safety-check
        classifiers:
          - toxicity
          - sentiment
        aggregation: max_score
```

### Using in Code

```rust
use checkstream_classifiers::{load_config, build_pipeline_from_config};
use std::collections::HashMap;
use std::sync::Arc;

// Load configuration
let config = load_config("./classifiers.yaml")?;

// Get pipeline spec
let pipeline_spec = config.get_pipeline("basic-safety")
    .ok_or("Pipeline not found")?;

// Build pipeline with your classifiers
let classifiers: HashMap<String, Arc<dyn Classifier>> = // ... your classifiers
let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers)?;

// Execute
let result = pipeline.execute("Check this text").await?;
println!("Score: {}, Latency: {}μs",
    result.final_decision.unwrap().score,
    result.total_latency_us);
```

## Pipeline Stages

### Single Stage

Executes one classifier.

```yaml
stages:
  - type: single
    name: toxicity-check
    classifier: toxicity
```

**Use when**: You need a straightforward, single classifier execution.

### Parallel Stage

Executes multiple classifiers concurrently.

```yaml
stages:
  - type: parallel
    name: multi-check
    classifiers:
      - toxicity
      - sentiment
      - prompt-injection
    aggregation: max_score
```

**Use when**:
- Classifiers are independent
- You want maximum speed
- Results need to be combined

**Performance**: All classifiers run simultaneously. Total latency ≈ slowest classifier.

### Sequential Stage

Executes classifiers one after another.

```yaml
stages:
  - type: sequential
    name: ordered-checks
    classifiers:
      - toxicity
      - sentiment
      - readability
```

**Use when**:
- Order matters
- Later classifiers need to see earlier results
- You want fine-grained control over execution flow

**Performance**: Total latency = sum of all classifier latencies.

### Conditional Stage

Executes classifier only if condition is met.

```yaml
stages:
  - type: single
    name: quick-check
    classifier: toxicity-distilled

  - type: conditional
    name: deep-check
    classifier: toxicity
    condition:
      any_above_threshold:
        threshold: 0.5
```

**Use when**:
- Classifier is expensive
- You want to skip work for clean inputs
- Adaptive execution based on context

**Performance**: Zero cost when condition not met.

## Aggregation Strategies

Control how parallel results are combined.

### All

Keep all results without filtering.

```yaml
aggregation: all
```

**Use**: When you need visibility into every classifier's output.

### Max Score

Return the result with the highest score.

```yaml
aggregation: max_score
```

**Use**: When any classifier triggering is significant (e.g., safety checks).

### Min Score

Return the result with the lowest score.

```yaml
aggregation: min_score
```

**Use**: When you want the most conservative/cautious result.

### First Positive

Return the first result above a threshold.

```yaml
aggregation:
  first_positive:
    threshold: 0.7
```

**Use**: Fast-fail scenarios where first detection is enough.

### Unanimous

Require all classifiers to agree (all positive or all negative).

```yaml
aggregation: unanimous
```

**Use**: High-confidence decisions requiring consensus.

### Weighted Average

Average scores across all classifiers.

```yaml
aggregation: weighted_average
```

**Use**: Balanced decision making from multiple signals.

## Conditions

Control when conditional stages execute.

### Any Above Threshold

Execute if any previous result exceeds threshold.

```yaml
condition:
  any_above_threshold:
    threshold: 0.6
```

### All Above Threshold

Execute if all previous results exceed threshold.

```yaml
condition:
  all_above_threshold:
    threshold: 0.5
```

### Classifier Triggered

Execute if specific classifier returned positive result (score > 0.5).

```yaml
condition:
  classifier_triggered:
    classifier: toxicity
```

### Always

Always execute (default behavior).

```yaml
condition: always
```

## Complete Examples

### Example 1: Progressive Depth

Start with fast check, go deeper if needed.

```yaml
pipelines:
  progressive-safety:
    description: "Fast initial check, deeper analysis if issues found"
    stages:
      # Stage 1: Quick distilled model (Tier B, <2ms)
      - type: single
        name: quick-scan
        classifier: toxicity-distilled

      # Stage 2: Full model only if quick scan triggered (Tier B, <5ms)
      - type: conditional
        name: detailed-scan
        classifier: toxicity
        condition:
          any_above_threshold:
            threshold: 0.3

      # Stage 3: Expensive checks only for confirmed issues (Tier C, <10ms)
      - type: conditional
        name: compliance-check
        classifier: financial-advice
        condition:
          any_above_threshold:
            threshold: 0.5
```

**Latency**:
- Clean input: ~2ms (just quick-scan)
- Suspicious input: ~7ms (quick-scan + detailed-scan)
- Confirmed issue: ~17ms (all stages)

### Example 2: Parallel Fast Lane

Multiple independent checks in parallel.

```yaml
pipelines:
  parallel-safety:
    description: "Run all safety checks simultaneously"
    stages:
      - type: parallel
        name: all-checks
        classifiers:
          - toxicity
          - prompt-injection
          - sentiment
        aggregation: max_score
```

**Latency**: Max of all classifiers (~5ms if all are Tier B)

### Example 3: Multi-Stage with Aggregation

Complex workflow with multiple stages and aggregation.

```yaml
pipelines:
  comprehensive:
    description: "Comprehensive multi-stage analysis"
    stages:
      # Stage 1: Parallel quick checks
      - type: parallel
        name: tier-a-checks
        classifiers:
          - toxicity-distilled
          - prompt-injection
        aggregation: max_score

      # Stage 2: Conditional deeper analysis
      - type: conditional
        name: tier-b-checks
        classifier: toxicity
        condition:
          any_above_threshold:
            threshold: 0.4

      # Stage 3: Parallel compliance checks
      - type: parallel
        name: compliance
        classifiers:
          - financial-advice
          - readability
        aggregation: weighted_average
```

### Example 4: Unanimous Consensus

Require all classifiers to agree.

```yaml
pipelines:
  high-confidence:
    description: "All classifiers must agree"
    stages:
      - type: parallel
        name: consensus-check
        classifiers:
          - toxicity
          - toxicity-distilled
          - sentiment
        aggregation: unanimous
```

**Use**: High-stakes decisions requiring multiple models to agree.

## Best Practices

### 1. Tier-Aware Design

Place faster classifiers (Tier A/B) before slower ones (Tier C).

```yaml
stages:
  - type: single
    name: fast-check
    classifier: toxicity-distilled  # Tier B: <2ms

  - type: conditional
    name: slow-check
    classifier: financial-advice     # Tier C: <10ms
    condition:
      any_above_threshold:
        threshold: 0.5
```

### 2. Parallelize Independent Checks

If classifiers don't depend on each other, run them in parallel.

```yaml
# Good: Parallel
- type: parallel
  name: independent-checks
  classifiers:
    - toxicity
    - prompt-injection
  aggregation: max_score

# Less efficient: Sequential (unless order matters)
- type: sequential
  name: independent-checks
  classifiers:
    - toxicity
    - prompt-injection
```

### 3. Use Conditionals for Expensive Operations

Skip expensive work when possible.

```yaml
# Initial cheap check
- type: single
  name: quick-filter
  classifier: pattern-matcher  # Regex, <1ms

# Expensive ML only if pattern matched
- type: conditional
  name: ml-analysis
  classifier: advanced-model    # Large model, <20ms
  condition:
    any_above_threshold:
      threshold: 0.1
```

### 4. Choose Aggregation Carefully

- **Safety checks**: Use `max_score` (any positive = flag)
- **Quality scoring**: Use `weighted_average` (balanced view)
- **High confidence**: Use `unanimous` (all must agree)
- **Fast detection**: Use `first_positive` (stop at first hit)

### 5. Name Stages Descriptively

```yaml
# Good
- type: parallel
  name: initial-safety-screening

# Less clear
- type: parallel
  name: stage1
```

## Performance Considerations

### Latency Budgets

Target total pipeline latency based on use case:

- **Streaming chat**: <5ms per chunk
- **Request validation**: <10ms
- **Content moderation**: <20ms
- **Batch analysis**: <100ms

### Optimization Strategies

1. **Conditional Execution**: Skip expensive checks for clean inputs
   ```yaml
   - type: conditional
     condition:
       any_above_threshold:
         threshold: 0.3
   ```

2. **Parallel Execution**: Run independent classifiers concurrently
   ```yaml
   - type: parallel
     classifiers: [a, b, c]
   ```

3. **Early Exit**: Use `first_positive` to stop at first detection
   ```yaml
   aggregation:
     first_positive:
       threshold: 0.8
   ```

4. **Tier Progression**: Start with fast models, escalate as needed
   ```yaml
   stages:
     - single: tier-a-model    # <2ms
     - conditional: tier-b     # <5ms if needed
     - conditional: tier-c     # <10ms if really needed
   ```

### Measuring Performance

Pipeline results include detailed timing:

```rust
let result = pipeline.execute(text).await?;

println!("Total latency: {}μs", result.total_latency_us);

for stage_result in result.results {
    println!("Stage '{}': {}μs",
        stage_result.stage_name,
        stage_result.stage_latency_us);
}
```

## Common Patterns

### Pattern: Fast Triage

Quick classification with minimal latency.

```yaml
fast-triage:
  stages:
    - type: parallel
      name: quick-scan
      classifiers: [toxicity-distilled, prompt-injection]
      aggregation:
        first_positive:
          threshold: 0.7
```

### Pattern: Progressive Depth

Start shallow, go deeper as needed.

```yaml
progressive:
  stages:
    - type: single
      name: tier-a
      classifier: pattern-matcher
    - type: conditional
      name: tier-b
      classifier: quantized-model
      condition: { any_above_threshold: { threshold: 0.3 } }
    - type: conditional
      name: tier-c
      classifier: full-model
      condition: { any_above_threshold: { threshold: 0.5 } }
```

### Pattern: Ensemble

Multiple models voting.

```yaml
ensemble:
  stages:
    - type: parallel
      name: vote
      classifiers: [model-a, model-b, model-c]
      aggregation: weighted_average
```

### Pattern: Gating

Use cheap check to gate expensive operation.

```yaml
gated:
  stages:
    - type: single
      name: gate
      classifier: cheap-filter
    - type: conditional
      name: expensive
      classifier: large-model
      condition: { any_above_threshold: { threshold: 0.1 } }
```

## Testing Pipelines

### Unit Testing

```rust
#[tokio::test]
async fn test_pipeline_execution() {
    let config = ClassifierConfig::from_yaml(r#"
        pipelines:
          test:
            stages:
              - type: single
                name: test-stage
                classifier: mock-classifier
    "#).unwrap();

    let pipeline_spec = config.get_pipeline("test").unwrap();

    let mut classifiers = HashMap::new();
    classifiers.insert("mock-classifier".to_string(),
        Arc::new(MockClassifier { score: 0.8 }) as Arc<dyn Classifier>);

    let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers).unwrap();
    let result = pipeline.execute("test").await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert!(result.total_latency_us > 0);
}
```

### Integration Testing

Test pipelines with real classifiers in development:

```rust
#[tokio::test]
#[ignore] // Requires models
async fn test_real_pipeline() {
    let config = load_config("./classifiers.yaml").unwrap();
    let registry = init_registry_from_config(&config).unwrap();

    // Build classifiers map from registry
    // ...

    let pipeline_spec = config.get_pipeline("basic-safety").unwrap();
    let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers).unwrap();

    let result = pipeline.execute("This is a test").await.unwrap();
    assert!(result.total_latency_us < 10_000); // <10ms
}
```

## Troubleshooting

### Pipeline Not Found

```
Error: Pipeline 'my-pipeline' not found in config
```

**Solution**: Check pipeline name in YAML and ensure it's in the `pipelines:` section.

### Classifier Not Found

```
Error: Classifier 'toxicity' not found for stage 'safety-check'
```

**Solution**: Ensure the classifier is:
1. Defined in the `models:` section
2. Loaded and available in the classifiers map
3. Name matches exactly (case-sensitive)

### Aggregation Type Mismatch

```
Error: failed to deserialize aggregation strategy
```

**Solution**: Check aggregation syntax:
```yaml
# Correct
aggregation: max_score

# Also correct
aggregation:
  first_positive:
    threshold: 0.7

# Incorrect
aggregation: maxScore  # Wrong case
```

### Condition Not Triggering

If a conditional stage isn't executing:

1. Check threshold values
2. Verify previous stages are producing results
3. Add logging to see actual scores

```rust
for result in &pipeline_result.results {
    println!("Stage: {}, Score: {}",
        result.stage_name, result.result.score);
}
```

## API Reference

### Building Pipelines

```rust
pub fn build_pipeline_from_config(
    config: &PipelineConfigSpec,
    classifiers: &HashMap<String, Arc<dyn Classifier>>,
) -> Result<ClassifierPipeline>
```

Creates a pipeline from configuration.

### Executing Pipelines

```rust
impl ClassifierPipeline {
    pub async fn execute(&self, text: &str) -> Result<PipelineExecutionResult>
}
```

### Results

```rust
pub struct PipelineExecutionResult {
    pub results: Vec<PipelineResult>,
    pub total_latency_us: u64,
    pub final_decision: Option<ClassificationResult>,
}

pub struct PipelineResult {
    pub stage_name: String,
    pub classifier_name: String,
    pub result: ClassificationResult,
    pub stage_latency_us: u64,
}
```

## Examples Repository

See complete working examples in:
- `examples/pipeline_basic.rs` - Simple pipeline usage
- `examples/pipeline_conditional.rs` - Conditional execution
- `examples/pipeline_parallel.rs` - Parallel execution with aggregation

## Next Steps

- [Model Loading](model-loading.md) - Configure and load ML models
- [Classifier Configuration](classifier-configuration.md) - Model configuration reference
- [Creating Custom Classifiers](custom-classifiers.md) - Implement your own classifiers
