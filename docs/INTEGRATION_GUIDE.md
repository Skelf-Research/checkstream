# Pipeline Integration Guide

This guide shows how to integrate the classifier pipeline system into CheckStream's proxy for real-world usage.

## Overview

The pipeline system is now ready for integration into the main proxy. This guide provides the integration path for using pipelines in request/response handling.

## Architecture Integration

```
┌─────────────┐
│   Request   │
└──────┬──────┘
       │
       v
┌─────────────────┐
│  Load Config    │  classifiers.yaml
└────────┬────────┘
         │
         v
┌─────────────────┐
│ Build Pipeline  │  build_pipeline_from_config()
└────────┬────────┘
         │
         v
┌─────────────────┐
│ Execute on Text │  pipeline.execute()
└────────┬────────┘
         │
         v
┌─────────────────┐
│ Check Decision  │  if score > threshold
└────────┬────────┘
         │
    ┌────┴────┐
    v         v
 PASS      BLOCK
```

## Step 1: Application State Setup

Add pipeline infrastructure to your application state:

```rust
// In crates/checkstream-proxy/src/main.rs or state.rs

use checkstream_classifiers::{
    ClassifierConfig, SharedRegistry, Classifier,
    build_pipeline_from_config, ClassifierPipeline,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct AppState {
    // Existing fields
    pub config: Arc<Config>,

    // New: Model registry for ML models
    pub model_registry: Arc<ModelRegistry>,

    // New: Classifier implementations
    pub classifiers: Arc<HashMap<String, Arc<dyn Classifier>>>,

    // New: Pre-built pipelines by name
    pub pipelines: Arc<HashMap<String, ClassifierPipeline>>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        // Load classifier configuration
        let classifier_config = load_config(&config.classifiers_config_path)?;

        // Initialize model registry
        let model_registry = init_registry_from_config(&classifier_config)?;

        // Build classifier implementations
        let classifiers = Self::build_classifiers(&model_registry, &classifier_config)?;

        // Pre-build all pipelines from config
        let pipelines = Self::build_all_pipelines(&classifier_config, &classifiers)?;

        Ok(Self {
            config: Arc::new(config),
            model_registry: Arc::new(model_registry),
            classifiers: Arc::new(classifiers),
            pipelines: Arc::new(pipelines),
        })
    }

    fn build_classifiers(
        registry: &ModelRegistry,
        config: &ClassifierConfig,
    ) -> Result<HashMap<String, Arc<dyn Classifier>>> {
        let mut classifiers = HashMap::new();

        // Add Tier A classifiers (pattern-based, no ML)
        classifiers.insert(
            "pii".to_string(),
            Arc::new(PiiClassifier::new()) as Arc<dyn Classifier>
        );
        classifiers.insert(
            "patterns".to_string(),
            Arc::new(PatternClassifier::new())
        );

        // Add Tier B/C classifiers (ML-based)
        for model_name in config.model_names() {
            if let Some(model) = registry.get(&model_name) {
                // Create classifier from loaded model
                let classifier = create_classifier_from_model(&model_name, model)?;
                classifiers.insert(model_name.clone(), classifier);
            }
        }

        Ok(classifiers)
    }

    fn build_all_pipelines(
        config: &ClassifierConfig,
        classifiers: &HashMap<String, Arc<dyn Classifier>>,
    ) -> Result<HashMap<String, ClassifierPipeline>> {
        let mut pipelines = HashMap::new();

        for pipeline_name in config.pipeline_names() {
            if let Some(pipeline_spec) = config.get_pipeline(&pipeline_name) {
                let pipeline = build_pipeline_from_config(pipeline_spec, classifiers)?;
                pipelines.insert(pipeline_name, pipeline);
            }
        }

        Ok(pipelines)
    }
}
```

## Step 2: Request Handler Integration

Integrate pipelines into your request handlers:

```rust
// In crates/checkstream-proxy/src/handlers/chat.rs

use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub model: String,
    #[serde(default)]
    pub stream: bool,
}

pub async fn handle_chat_completion(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Response> {
    // Step 1: Extract text to check
    let text_to_check = extract_user_prompt(&request.messages);

    // Step 2: Select pipeline based on config or request
    let pipeline_name = state.config.default_pipeline.as_str(); // e.g., "basic-safety"
    let pipeline = state.pipelines.get(pipeline_name)
        .ok_or_else(|| Error::pipeline_not_found(pipeline_name))?;

    // Step 3: Execute pipeline
    let start = Instant::now();
    let result = pipeline.execute(&text_to_check).await?;
    let check_latency = start.elapsed();

    // Step 4: Log results
    info!(
        pipeline = pipeline_name,
        latency_us = result.total_latency_us,
        stages = result.results.len(),
        "Pipeline execution complete"
    );

    // Log individual stage results
    for stage_result in &result.results {
        debug!(
            stage = %stage_result.stage_name,
            classifier = %stage_result.classifier_name,
            score = stage_result.result.score,
            latency_us = stage_result.stage_latency_us,
            "Stage result"
        );
    }

    // Step 5: Make decision based on final result
    if let Some(decision) = result.final_decision {
        if decision.score > state.config.safety_threshold {
            // BLOCK: Safety check failed
            warn!(
                score = decision.score,
                threshold = state.config.safety_threshold,
                "Request blocked by safety check"
            );

            return Ok(Response::blocked(
                "Content policy violation detected",
                decision.score,
            ));
        }
    }

    // Step 6: PASS - Forward to upstream LLM
    info!("Request passed safety checks, forwarding to LLM");
    forward_to_upstream(state, request).await
}
```

## Step 3: Streaming Response Integration

For streaming responses, check each chunk:

```rust
// In crates/checkstream-proxy/src/handlers/stream.rs

pub async fn handle_streaming_chat(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Response> {
    // Initial prompt check (same as above)
    check_prompt_with_pipeline(&state, &request).await?;

    // Create SSE stream
    let stream = stream_llm_response(state.clone(), request).await?;

    // Wrap stream with per-chunk safety checks
    let checked_stream = stream.then(move |chunk| {
        let state = state.clone();
        async move {
            match chunk {
                Ok(chunk_text) => {
                    // Use fast pipeline for per-chunk checks
                    if let Some(pipeline) = state.pipelines.get("fast-triage") {
                        match pipeline.execute(&chunk_text).await {
                            Ok(result) => {
                                if let Some(decision) = result.final_decision {
                                    if decision.score > state.config.chunk_threshold {
                                        // Block this chunk
                                        return Ok("[REDACTED]".to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Chunk check failed: {}", e);
                                // Fail open or closed based on config
                            }
                        }
                    }
                    Ok(chunk_text)
                }
                Err(e) => Err(e),
            }
        }
    });

    Ok(Response::stream(checked_stream))
}
```

## Step 4: Configuration

Update your main config to include pipeline settings:

```yaml
# config.yaml

# Proxy settings
proxy:
  port: 8080
  backend_url: https://api.openai.com/v1

# Classifier configuration
classifiers:
  config_path: ./classifiers.yaml

  # Default pipeline for request validation
  default_pipeline: basic-safety

  # Fast pipeline for per-chunk streaming checks
  streaming_pipeline: fast-triage

  # Safety thresholds
  safety_threshold: 0.7      # Block if score > 0.7
  chunk_threshold: 0.8       # More permissive for chunks

  # Performance settings
  timeout_ms: 10             # Max pipeline execution time
  cache_results: true        # Cache recent checks
  cache_ttl_seconds: 60
```

## Step 5: Metrics Integration

Add Prometheus metrics for pipeline performance:

```rust
// In crates/checkstream-telemetry/src/metrics.rs

use metrics::{counter, histogram, gauge};

pub fn record_pipeline_execution(
    pipeline_name: &str,
    latency_us: u64,
    decision: &str,
    score: f32,
) {
    // Latency histogram
    histogram!(
        "checkstream_pipeline_latency_us",
        latency_us as f64,
        "pipeline" => pipeline_name.to_string()
    );

    // Execution counter
    counter!(
        "checkstream_pipeline_executions_total",
        1,
        "pipeline" => pipeline_name.to_string(),
        "decision" => decision.to_string()
    );

    // Score distribution
    histogram!(
        "checkstream_pipeline_score",
        score as f64,
        "pipeline" => pipeline_name.to_string()
    );
}

pub fn record_stage_execution(
    pipeline_name: &str,
    stage_name: &str,
    classifier_name: &str,
    latency_us: u64,
) {
    histogram!(
        "checkstream_stage_latency_us",
        latency_us as f64,
        "pipeline" => pipeline_name.to_string(),
        "stage" => stage_name.to_string(),
        "classifier" => classifier_name.to_string()
    );
}
```

## Step 6: Error Handling

Handle pipeline errors gracefully:

```rust
pub async fn execute_pipeline_with_fallback(
    state: &AppState,
    pipeline_name: &str,
    text: &str,
) -> Result<PipelineExecutionResult> {
    // Try primary pipeline
    if let Some(pipeline) = state.pipelines.get(pipeline_name) {
        match timeout(
            Duration::from_millis(state.config.timeout_ms),
            pipeline.execute(text)
        ).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                warn!("Pipeline {} failed: {}", pipeline_name, e);
            }
            Err(_) => {
                warn!("Pipeline {} timed out", pipeline_name);
            }
        }
    }

    // Fallback to simple pattern-based check
    if let Some(fallback) = state.classifiers.get("patterns") {
        let result = fallback.classify(text).await?;
        return Ok(PipelineExecutionResult {
            results: vec![PipelineResult {
                stage_name: "fallback".to_string(),
                classifier_name: "patterns".to_string(),
                result,
                stage_latency_us: 0,
            }],
            total_latency_us: 0,
            final_decision: Some(result),
        });
    }

    // Final fallback: fail open or closed based on config
    if state.config.fail_open {
        Ok(create_pass_result())
    } else {
        Err(Error::all_pipelines_failed())
    }
}
```

## Step 7: Testing Integration

Create integration tests:

```rust
// In crates/checkstream-proxy/tests/integration_test.rs

#[tokio::test]
async fn test_pipeline_integration() {
    // Setup test app
    let app = create_test_app().await;

    // Test 1: Clean request should pass
    let response = app
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello world"}]
        }))
        .send()
        .await;

    assert_eq!(response.status(), 200);

    // Test 2: Toxic request should be blocked
    let response = app
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Very toxic content here"}]
        }))
        .send()
        .await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await;
    assert!(body["error"]["message"].as_str().unwrap().contains("policy violation"));

    // Test 3: Check metrics were recorded
    let metrics = app.get_metrics().await;
    assert!(metrics.contains("checkstream_pipeline_executions_total"));
}
```

## Step 8: Performance Optimization

Optimize for production:

```rust
// Pre-warm classifiers at startup
pub async fn prewarm_classifiers(state: &AppState) -> Result<()> {
    let warmup_text = "warmup";

    for (name, pipeline) in state.pipelines.iter() {
        info!("Pre-warming pipeline: {}", name);
        match pipeline.execute(warmup_text).await {
            Ok(_) => info!("Pipeline {} warmed up", name),
            Err(e) => warn!("Failed to warm up pipeline {}: {}", name, e),
        }
    }

    Ok(())
}

// Cache recent results
pub struct PipelineCache {
    cache: Arc<Mutex<LruCache<String, PipelineExecutionResult>>>,
}

impl PipelineCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    pub async fn get_or_execute(
        &self,
        key: &str,
        pipeline: &ClassifierPipeline,
        text: &str,
    ) -> Result<PipelineExecutionResult> {
        // Check cache
        if let Some(cached) = self.cache.lock().await.get(key) {
            return Ok(cached.clone());
        }

        // Execute and cache
        let result = pipeline.execute(text).await?;
        self.cache.lock().await.put(key.to_string(), result.clone());

        Ok(result)
    }
}
```

## Complete Integration Checklist

- [ ] Add pipeline fields to `AppState`
- [ ] Load classifiers at startup
- [ ] Pre-build all pipelines from config
- [ ] Integrate pipeline execution in request handlers
- [ ] Add per-chunk checks for streaming
- [ ] Configure thresholds and timeouts
- [ ] Add Prometheus metrics
- [ ] Implement error handling and fallbacks
- [ ] Add integration tests
- [ ] Pre-warm classifiers at startup
- [ ] Add result caching if needed
- [ ] Document API changes
- [ ] Update deployment guide

## Next Steps

1. **Implement in Proxy**: Follow this guide to integrate into `checkstream-proxy`
2. **Load Testing**: Verify latency targets under load
3. **Monitoring**: Set up dashboards for pipeline metrics
4. **Tuning**: Adjust thresholds and pipeline configurations based on production data

## See Also

- [Pipeline Configuration Guide](pipeline-configuration.md)
- [Quick Start](QUICKSTART_PIPELINES.md)
- [Example Code](../examples/pipeline_usage.rs)
