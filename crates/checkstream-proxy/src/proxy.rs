//! Core proxy logic

use anyhow::Result;
use checkstream_classifiers::{
    ClassifierRegistry, ClassifierPipeline, StreamingPipeline,
};
use checkstream_policy::{
    PolicyEngine, ActionExecutor, ActionOutcome, EvaluationResult,
};
use checkstream_telemetry::{
    AuditService, PersistenceConfig, RequestContext, PolicyAuditRecord, PolicySeverity,
};
use metrics_exporter_prometheus::PrometheusHandle;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

use crate::config::ProxyConfig;

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    /// Loaded configuration
    pub config: Arc<ProxyConfig>,

    /// Classifier registry with all loaded classifiers
    pub registry: Arc<ClassifierRegistry>,

    /// Pre-built pipelines for each phase
    pub pipelines: Arc<Pipelines>,

    /// HTTP client for backend requests
    pub http_client: reqwest::Client,

    /// Prometheus metrics handle for rendering
    pub metrics_handle: PrometheusHandle,

    /// Policy engine for evaluating rules
    pub policy_engine: Arc<RwLock<PolicyEngine>>,

    /// Action executor for enforcing policies
    pub action_executor: Arc<ActionExecutor>,

    /// Audit service for compliance logging
    pub audit_service: Arc<AuditService>,
}

/// Pre-built pipelines for the three phases
pub struct Pipelines {
    /// Phase 1: Ingress (pre-generation validation)
    pub ingress: ClassifierPipeline,

    /// Phase 2: Midstream (streaming checks)
    pub midstream: ClassifierPipeline,

    /// Phase 3: Egress (post-generation compliance)
    pub egress: ClassifierPipeline,
}

impl AppState {
    /// Initialize application state from configuration
    pub async fn new(config: ProxyConfig, metrics_handle: PrometheusHandle) -> Result<Self> {
        info!("Initializing application state");

        // Load classifier registry
        info!("Loading classifiers from: {}", config.classifiers_config);
        let registry = ClassifierRegistry::from_file(&config.classifiers_config).await?;
        info!("Loaded {} classifiers", registry.count());

        // Build pipelines for each phase
        let pipelines = Self::build_pipelines(&config, &registry)?;

        // Load policy engine
        let policy_engine = Self::load_policy_engine(&config)?;
        let policy_count = policy_engine.policies().len();
        info!("Loaded {} policies from: {}", policy_count, config.policy_path);

        // Create action executor
        let action_executor = ActionExecutor::new();

        // Initialize audit service
        let audit_config = PersistenceConfig {
            audit_dir: std::path::PathBuf::from("./audit"),
            ..Default::default()
        };
        let audit_service = AuditService::new(audit_config)
            .map_err(|e| anyhow::anyhow!("Failed to initialize audit service: {}", e))?;

        // Create HTTP client for backend requests
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()?;

        Ok(Self {
            config: Arc::new(config),
            registry: Arc::new(registry),
            pipelines: Arc::new(pipelines),
            http_client,
            metrics_handle,
            policy_engine: Arc::new(RwLock::new(policy_engine)),
            action_executor: Arc::new(action_executor),
            audit_service: Arc::new(audit_service),
        })
    }

    /// Load policy engine from configuration
    fn load_policy_engine(config: &ProxyConfig) -> Result<PolicyEngine> {
        let mut engine = PolicyEngine::new();

        let policy_path = std::path::Path::new(&config.policy_path);
        if policy_path.exists() {
            if policy_path.is_file() {
                // Load single policy file
                engine.load_policy(&config.policy_path)?;
            } else if policy_path.is_dir() {
                // Load all YAML files in the directory
                for entry in std::fs::read_dir(policy_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                        if let Err(e) = engine.load_policy(&path) {
                            warn!("Failed to load policy {:?}: {}", path, e);
                        }
                    }
                }
            }
        } else {
            info!("Policy path does not exist, using empty policy engine: {}", config.policy_path);
        }

        Ok(engine)
    }

    /// Build pipelines from configuration
    fn build_pipelines(config: &ProxyConfig, registry: &ClassifierRegistry) -> Result<Pipelines> {
        info!("Building pipelines");

        // Build Phase 1: Ingress pipeline
        info!("Building ingress pipeline: {}", config.pipelines.ingress_pipeline);
        let ingress = registry.build_pipeline(&config.pipelines.ingress_pipeline)?;

        // Build Phase 2: Midstream pipeline
        info!("Building midstream pipeline: {}", config.pipelines.midstream_pipeline);
        let midstream = registry.build_pipeline(&config.pipelines.midstream_pipeline)?;

        // Build Phase 3: Egress pipeline
        info!("Building egress pipeline: {}", config.pipelines.egress_pipeline);
        let egress = registry.build_pipeline(&config.pipelines.egress_pipeline)?;

        Ok(Pipelines {
            ingress,
            midstream,
            egress,
        })
    }
}

/// Phase 1: Ingress - Validate request before sending to LLM
pub async fn execute_ingress(
    state: &AppState,
    prompt: &str,
    request_id: &str,
) -> Result<IngressResult> {
    debug!("Phase 1: Executing ingress checks on prompt");

    let start = std::time::Instant::now();
    let result = state.pipelines.ingress.execute(prompt).await?;
    let classifier_latency = start.elapsed();

    // Record classifier metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "ingress")
        .record(classifier_latency.as_micros() as f64);

    // Extract classifier scores and inject into policy engine
    let classifier_scores = extract_classifier_scores(&result);

    // Evaluate policies
    let policy_results = {
        let mut engine = state.policy_engine.write().unwrap();
        engine.set_classifier_scores(classifier_scores);
        engine.evaluate_text(prompt)
    };

    // Execute actions from triggered policies
    let action_outcome = state.action_executor.execute(&policy_results);

    let latency = start.elapsed();

    // Check for blocking - either from action outcome or threshold
    let should_block = action_outcome.should_stop || result.final_decision
        .as_ref()
        .map_or(false, |d| d.score > state.config.pipelines.safety_threshold);

    // Record audit events for triggered policies
    let request_ctx = RequestContext::new(request_id, "ingress");
    for audit_record in &action_outcome.audit_records {
        let policy_record = PolicyAuditRecord {
            rule_name: audit_record.rule_name.clone(),
            policy_name: audit_record.policy_name.clone(),
            category: audit_record.category.clone(),
            severity: convert_severity(&audit_record.severity),
            context: audit_record.context.clone(),
        };
        state.audit_service.record_from_policy(&policy_record, &request_ctx);
    }

    if should_block {
        metrics::counter!("checkstream_decisions_total", "phase" => "ingress", "action" => "block")
            .increment(1);

        if action_outcome.should_stop {
            info!(
                "Phase 1: BLOCKED by policy - Rules: {:?}, Latency: {:?}",
                policy_results.iter().map(|r| &r.rule_name).collect::<Vec<_>>(),
                latency
            );
        } else {
            info!(
                "Phase 1: BLOCKED - Score: {:.3}, Latency: {:?}",
                result.final_decision.as_ref().unwrap().score,
                latency
            );
        }
    } else {
        metrics::counter!("checkstream_decisions_total", "phase" => "ingress", "action" => "pass")
            .increment(1);

        debug!("Phase 1: PASSED - Latency: {:?}", latency);
    }

    // Record policy evaluation metrics
    if !policy_results.is_empty() {
        metrics::counter!("checkstream_policies_triggered_total", "phase" => "ingress")
            .increment(policy_results.len() as u64);
    }

    Ok(IngressResult {
        blocked: should_block,
        result,
        latency,
        policy_results,
        action_outcome,
    })
}

/// Phase 2: Midstream - Check streaming chunks as they arrive
pub async fn execute_midstream_chunk(
    state: &AppState,
    streaming: &mut StreamingPipeline,
    chunk: String,
    threshold: f32,
    request_id: &str,
) -> Result<MidstreamResult> {
    debug!("Phase 2: Checking chunk: {:?}", chunk);

    let start = std::time::Instant::now();
    let result = streaming.execute_chunk(chunk.clone()).await?;
    let classifier_latency = start.elapsed();

    // Record classifier metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "midstream")
        .record(classifier_latency.as_micros() as f64);

    // Extract classifier scores and inject into policy engine
    let classifier_scores = extract_classifier_scores(&result);

    // Evaluate policies on the chunk
    let policy_results = {
        let mut engine = state.policy_engine.write().unwrap();
        engine.set_classifier_scores(classifier_scores);
        engine.evaluate_text(&chunk)
    };

    // Execute actions from triggered policies
    let action_outcome = state.action_executor.execute(&policy_results);

    let latency = start.elapsed();

    // Record audit events for triggered policies
    let request_ctx = RequestContext::new(request_id, "midstream");
    for audit_record in &action_outcome.audit_records {
        let policy_record = PolicyAuditRecord {
            rule_name: audit_record.rule_name.clone(),
            policy_name: audit_record.policy_name.clone(),
            category: audit_record.category.clone(),
            severity: convert_severity(&audit_record.severity),
            context: audit_record.context.clone(),
        };
        state.audit_service.record_from_policy(&policy_record, &request_ctx);
    }

    // Check if this chunk should be redacted (from policy or threshold)
    let should_redact = action_outcome.should_stop
        || !action_outcome.modifications.is_empty()
        || result.final_decision.as_ref().map_or(false, |d| d.score > threshold);

    if should_redact {
        metrics::counter!("checkstream_decisions_total", "phase" => "midstream", "action" => "redact")
            .increment(1);

        debug!(
            "Phase 2: REDACTING chunk - Score: {:.3}",
            result.final_decision.as_ref().map_or(0.0, |d| d.score)
        );
    }

    // Record policy evaluation metrics
    if !policy_results.is_empty() {
        metrics::counter!("checkstream_policies_triggered_total", "phase" => "midstream")
            .increment(policy_results.len() as u64);
    }

    Ok(MidstreamResult {
        redacted: should_redact,
        result,
        latency,
        policy_results,
        action_outcome,
    })
}

/// Phase 3: Egress - Final compliance check on complete response
pub async fn execute_egress(
    state: &AppState,
    full_text: &str,
    request_id: &str,
) -> Result<EgressResult> {
    info!("Phase 3: Executing egress compliance check");

    let start = std::time::Instant::now();
    let result = state.pipelines.egress.execute(full_text).await?;
    let classifier_latency = start.elapsed();

    // Record classifier metrics
    metrics::histogram!("checkstream_pipeline_latency_us", "phase" => "egress")
        .record(classifier_latency.as_micros() as f64);

    // Extract classifier scores and inject into policy engine
    let classifier_scores = extract_classifier_scores(&result);

    // Evaluate policies on complete response
    let policy_results = {
        let mut engine = state.policy_engine.write().unwrap();
        engine.set_classifier_scores(classifier_scores);
        engine.evaluate_text(full_text)
    };

    // Execute actions from triggered policies
    let action_outcome = state.action_executor.execute(&policy_results);

    let latency = start.elapsed();

    // Record audit events for triggered policies
    let request_ctx = RequestContext::new(request_id, "egress");
    for audit_record in &action_outcome.audit_records {
        let policy_record = PolicyAuditRecord {
            rule_name: audit_record.rule_name.clone(),
            policy_name: audit_record.policy_name.clone(),
            category: audit_record.category.clone(),
            severity: convert_severity(&audit_record.severity),
            context: audit_record.context.clone(),
        };
        state.audit_service.record_from_policy(&policy_record, &request_ctx);
    }

    // Record policy evaluation metrics
    if !policy_results.is_empty() {
        metrics::counter!("checkstream_policies_triggered_total", "phase" => "egress")
            .increment(policy_results.len() as u64);

        // Log compliance issues
        for result in &policy_results {
            if result.score > 0.7 {
                warn!(
                    "Phase 3: Compliance issue detected - Rule: {}, Score: {:.3}",
                    result.rule_name, result.score
                );
            }
        }
    }

    metrics::counter!("checkstream_decisions_total", "phase" => "egress", "action" => "complete")
        .increment(1);

    info!("Phase 3: COMPLETE - Latency: {:?}", latency);

    Ok(EgressResult {
        result,
        latency,
        policy_results,
        action_outcome,
    })
}

/// Result from Phase 1: Ingress
pub struct IngressResult {
    pub blocked: bool,
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
    /// Policy evaluation results
    pub policy_results: Vec<EvaluationResult>,
    /// Action outcomes from policy execution
    pub action_outcome: ActionOutcome,
}

/// Result from Phase 2: Midstream chunk check
pub struct MidstreamResult {
    pub redacted: bool,
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
    /// Policy evaluation results
    pub policy_results: Vec<EvaluationResult>,
    /// Action outcomes from policy execution
    pub action_outcome: ActionOutcome,
}

/// Result from Phase 3: Egress
pub struct EgressResult {
    pub result: checkstream_classifiers::PipelineExecutionResult,
    pub latency: std::time::Duration,
    /// Policy evaluation results
    pub policy_results: Vec<EvaluationResult>,
    /// Action outcomes from policy execution
    pub action_outcome: ActionOutcome,
}

/// Extract classifier scores from pipeline execution result
fn extract_classifier_scores(
    result: &checkstream_classifiers::PipelineExecutionResult
) -> HashMap<String, f32> {
    let mut scores = HashMap::new();

    // Extract scores from pipeline results
    for pipeline_result in &result.results {
        scores.insert(
            pipeline_result.classifier_name.clone(),
            pipeline_result.result.score,
        );
    }

    // Also include final decision if available
    if let Some(ref decision) = result.final_decision {
        scores.insert("_final".to_string(), decision.score);
    }

    scores
}

/// Convert policy action AuditSeverity to telemetry PolicySeverity
fn convert_severity(severity: &checkstream_policy::action::AuditSeverity) -> PolicySeverity {
    use checkstream_policy::action::AuditSeverity;
    match severity {
        AuditSeverity::Low => PolicySeverity::Low,
        AuditSeverity::Medium => PolicySeverity::Medium,
        AuditSeverity::High => PolicySeverity::High,
        AuditSeverity::Critical => PolicySeverity::Critical,
    }
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("req_{:x}", timestamp)
}
