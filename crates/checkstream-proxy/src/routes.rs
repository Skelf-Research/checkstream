//! HTTP routes and handlers

use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use checkstream_telemetry::{AuditQuery as TelemetryAuditQuery, AuditSeverity};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use axum::extract::Path;
use checkstream_classifiers::{StreamingPipeline, StreamingConfig};
use checkstream_core::{StreamAdapter, ParsedChunk};
use crate::proxy::{self, AppState, generate_request_id};
use crate::tenant::TenantRuntime;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and metrics (tenant-agnostic)
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))
        .route("/metrics", get(metrics))
        // Chat completions - default tenant
        .route("/v1/chat/completions", post(chat_completions))
        // Chat completions - tenant-prefixed route
        .route("/:tenant_id/v1/chat/completions", post(chat_completions_with_tenant))
        // Audit endpoints
        .route("/audit", get(audit_query))
        .route("/audit/stats", get(audit_stats))
        // Tenant info endpoint
        .route("/tenants", get(list_tenants))
        .fallback(fallback)
        .with_state(state)
}

/// Basic health check - always returns OK if server is running
async fn health_check() -> &'static str {
    "OK"
}

/// Liveness probe - indicates if the service is alive
/// Returns 200 if the service is running, even if not ready to serve traffic
async fn liveness_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "alive",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

/// Readiness probe - indicates if the service is ready to serve traffic
/// Checks that all components are initialized
async fn readiness_check(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let classifier_count = state.registry.count();

    // Check if we have classifiers loaded
    if classifier_count == 0 {
        return Err(AppError::InternalError("No classifiers loaded".to_string()));
    }

    // Check if policy engine has policies (optional - may be valid with no policies)
    let policy_count = state.policy_engine.read().unwrap().policies().len();

    Ok(Json(json!({
        "status": "ready",
        "components": {
            "classifiers": classifier_count,
            "policies": policy_count,
            "audit_service": "ok"
        },
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })))
}

async fn metrics(State(state): State<AppState>) -> String {
    // Render actual Prometheus metrics from the handle
    state.metrics_handle.render()
}

/// Audit query request parameters
#[derive(Debug, Deserialize)]
struct AuditQueryParams {
    /// Filter by event type
    event_type: Option<String>,
    /// Filter by request ID
    request_id: Option<String>,
    /// Filter by phase (ingress/midstream/egress)
    phase: Option<String>,
    /// Minimum severity (info/warning/high/critical)
    min_severity: Option<String>,
    /// Maximum number of results
    limit: Option<usize>,
}

/// Audit query handler
async fn audit_query(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut query = TelemetryAuditQuery::new();

    if let Some(event_type) = params.event_type {
        query = query.event_type(&event_type);
    }

    if let Some(request_id) = params.request_id {
        query = query.request_id(&request_id);
    }

    if let Some(phase) = params.phase {
        query = query.phase(&phase);
    }

    if let Some(severity) = params.min_severity {
        let min_sev = match severity.to_lowercase().as_str() {
            "info" => AuditSeverity::Info,
            "warning" => AuditSeverity::Warning,
            "high" => AuditSeverity::High,
            "critical" => AuditSeverity::Critical,
            _ => AuditSeverity::Info,
        };
        query = query.min_severity(min_sev);
    }

    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let events = state.audit_service.query(&query)
        .map_err(|e| AppError::InternalError(format!("Audit query failed: {}", e)))?;

    let events_json: Vec<serde_json::Value> = events.iter().map(|e| {
        json!({
            "timestamp": e.event.timestamp.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            "event_type": e.event.event_type,
            "severity": format!("{:?}", e.event.severity),
            "request_id": e.request_id,
            "session_id": e.session_id,
            "phase": e.phase,
            "model": e.model,
            "data": e.event.data,
        })
    }).collect();

    Ok(Json(json!({
        "count": events_json.len(),
        "events": events_json
    })))
}

/// Audit stats handler
async fn audit_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = state.audit_service.stats()
        .map_err(|e| AppError::InternalError(format!("Audit stats failed: {}", e)))?;

    Ok(Json(json!({
        "total_events": stats.total_events,
        "critical_events": stats.critical_events,
        "high_severity_events": stats.high_severity_events,
        "events_last_24h": stats.events_last_24h
    })))
}

/// OpenAI-compatible chat completions request
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(flatten)]
    other: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Message {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completions response (non-streaming)
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// SSE chunk for streaming responses
#[derive(Debug, Serialize, Deserialize)]
struct StreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StreamChoice {
    index: u32,
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

/// Main chat completions handler (uses tenant from header or API key, falls back to default)
async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, AppError> {
    // Resolve tenant from headers/API key
    let tenant = state.tenant_resolver.resolve(&headers, "/v1/chat/completions");
    chat_completions_internal(state, tenant, headers, req).await
}

/// Chat completions handler with explicit tenant from path
async fn chat_completions_with_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, AppError> {
    // Resolve tenant from path, falling back to header/API key resolution
    let tenant = state.tenant_resolver.get(&tenant_id)
        .unwrap_or_else(|| state.tenant_resolver.resolve(&headers, &format!("/{}/v1/chat/completions", tenant_id)));

    chat_completions_internal(state, tenant, headers, req).await
}

/// List configured tenants
async fn list_tenants(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let tenants: Vec<serde_json::Value> = state.tenant_resolver.list_tenants()
        .iter()
        .map(|id| json!({ "id": id }))
        .collect();

    Json(json!({
        "tenants": tenants,
        "multi_tenant_enabled": state.tenant_resolver.is_multi_tenant()
    }))
}

/// Internal chat completions handler (shared by default and tenant-prefixed routes)
async fn chat_completions_internal(
    state: AppState,
    tenant: Arc<TenantRuntime>,
    headers: HeaderMap,
    req: ChatCompletionRequest,
) -> Result<Response, AppError> {
    // Generate unique request ID for audit trail
    let request_id = generate_request_id();
    info!(
        "Received chat completion request for model: {} tenant: {} (request_id: {})",
        req.model, tenant.id, request_id
    );
    metrics::counter!("checkstream_requests_total", "tenant" => tenant.id.clone()).increment(1);

    // Extract the user prompt (last user message)
    let prompt = extract_prompt(&req.messages)?;
    debug!("Extracted prompt: {}", prompt);

    // **Phase 1: Ingress** - Validate prompt before sending to LLM
    let ingress_result = proxy::execute_ingress(&state, &prompt, &request_id).await?;

    if ingress_result.blocked {
        warn!("Request blocked by ingress pipeline (request_id: {})", request_id);
        return Ok(blocked_response(&req).into_response());
    }

    // Forward request to backend LLM
    if req.stream {
        // Streaming response path with Phase 2: Midstream checks
        handle_streaming_request(state, tenant, req, headers, request_id).await
    } else {
        // Non-streaming response path
        handle_non_streaming_request(state, tenant, req, headers, request_id).await
    }
}

/// Handle non-streaming chat completion (complete response at once)
async fn handle_non_streaming_request(
    state: AppState,
    tenant: Arc<TenantRuntime>,
    req: ChatCompletionRequest,
    headers: HeaderMap,
    request_id: String,
) -> Result<Response, AppError> {
    info!("Handling non-streaming request for tenant: {} (request_id: {})", tenant.id, request_id);

    // Forward to tenant-specific backend
    let backend_url = format!("{}/chat/completions", tenant.backend_url);
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let backend_response = state.http_client
        .post(&backend_url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await?;

    if !backend_response.status().is_success() {
        error!("Backend request failed: {} (request_id: {})", backend_response.status(), request_id);
        return Err(AppError::BackendError(backend_response.status()));
    }

    let response_text = backend_response.text().await?;
    let response: ChatCompletionResponse = serde_json::from_str(&response_text)?;

    // **Phase 3: Egress** - Compliance check on complete response
    let assistant_message = &response.choices[0].message.content;
    let egress_result = proxy::execute_egress(&state, assistant_message, &request_id).await?;

    info!("Non-streaming request complete (request_id: {})", request_id);

    Ok(Json(response).into_response())
}

/// Handle streaming chat completion with Phase 2: Midstream checks
async fn handle_streaming_request(
    state: AppState,
    tenant: Arc<TenantRuntime>,
    mut req: ChatCompletionRequest,
    headers: HeaderMap,
    request_id: String,
) -> Result<Response, AppError> {
    info!(
        "Handling streaming request for tenant: {} with midstream checks (request_id: {})",
        tenant.id, request_id
    );

    // Ensure stream is enabled
    req.stream = true;

    // Forward to tenant-specific backend
    let backend_url = format!("{}/chat/completions", tenant.backend_url);
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let backend_response = state.http_client
        .post(&backend_url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await?;

    if !backend_response.status().is_success() {
        error!("Backend streaming request failed: {} (request_id: {})", backend_response.status(), request_id);
        return Err(AppError::BackendError(backend_response.status()));
    }

    // Create streaming pipeline for Phase 2: Midstream checks using tenant-specific settings
    let streaming_config = StreamingConfig {
        context_chunks: tenant.pipeline_settings.streaming.context_chunks,
        max_buffer_size: tenant.pipeline_settings.streaming.max_buffer_size,
        chunk_delimiter: " ".to_string(),
    };

    let midstream_pipeline = tenant.pipelines.midstream.clone();
    let streaming = Arc::new(Mutex::new(
        StreamingPipeline::new(midstream_pipeline, streaming_config)
    ));

    let chunk_threshold = tenant.pipeline_settings.chunk_threshold;
    let full_text = Arc::new(Mutex::new(String::new()));
    let state_for_egress = state.clone();
    let full_text_for_egress = full_text.clone();
    let request_id_for_egress = request_id.clone();

    // Spawn async task to execute Phase 3 after stream completes
    let _egress_handle = tokio::spawn(async move {
        // Wait a bit to ensure stream has collected text
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get full text
        let text = {
            let full = full_text_for_egress.lock().await;
            full.clone()
        };

        if !text.is_empty() {
            // **Phase 3: Egress** - Final compliance check
            match proxy::execute_egress(&state_for_egress, &text, &request_id_for_egress).await {
                Ok(_result) => {
                    info!("Phase 3 completed successfully (request_id: {})", request_id_for_egress);
                }
                Err(e) => {
                    error!("Phase 3 failed: {} (request_id: {})", e, request_id_for_egress);
                }
            }
        }
    });

    // Clone for midstream processing
    let state_for_midstream = state.clone();
    let request_id_for_midstream = request_id.clone();
    let stream_adapter = tenant.stream_adapter.clone();

    // Convert backend stream to SSE stream with midstream checks
    let stream = backend_response.bytes_stream()
        .filter_map(move |chunk_result| {
            let streaming = streaming.clone();
            let full_text = full_text.clone();
            let state = state_for_midstream.clone();
            let req_id = request_id_for_midstream.clone();
            let adapter = stream_adapter.clone();

            async move {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();

                        // Use tenant-specific stream adapter to parse the chunk
                        let parsed_chunks = adapter.parse(&text);

                        // Process each parsed chunk
                        for parsed in &parsed_chunks {
                            if let ParsedChunk::Content { text: content, .. } = parsed {
                                // Store for Phase 3
                                {
                                    let mut full = full_text.lock().await;
                                    full.push_str(content);
                                }

                                // **Phase 2: Midstream** - Check this chunk
                                let mut streaming = streaming.lock().await;
                                match proxy::execute_midstream_chunk(
                                    &state,
                                    &mut *streaming,
                                    content.clone(),
                                    chunk_threshold,
                                    &req_id,
                                ).await {
                                    Ok(result) => {
                                        if result.redacted {
                                            warn!("Chunk redacted by midstream pipeline (request_id: {})", req_id);
                                            return Some(Ok::<String, std::io::Error>("[REDACTED]".to_string()));
                                        }
                                    }
                                    Err(e) => {
                                        error!("Midstream check failed: {} (request_id: {})", e, req_id);
                                        // Pass through on error
                                    }
                                }
                            }
                        }

                        // Return original text if no redaction needed
                        Some(Ok::<String, std::io::Error>(text))
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        None
                    }
                }
            }
        });

    // Return SSE response with tenant-specific content type
    let mut response = Response::new(axum::body::Body::from_stream(stream));
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_str(tenant.stream_adapter.content_type()).unwrap_or(HeaderValue::from_static("text/event-stream"))
    );
    response.headers_mut().insert(
        "Cache-Control",
        HeaderValue::from_static("no-cache")
    );
    response.headers_mut().insert(
        "X-CheckStream-Tenant",
        HeaderValue::from_str(&tenant.id).unwrap_or(HeaderValue::from_static("default"))
    );

    Ok(response)
}

/// Extract user prompt from messages
fn extract_prompt(messages: &[Message]) -> Result<String, AppError> {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .ok_or_else(|| AppError::InvalidRequest("No user message found".to_string()))
}

/// Create blocked response
fn blocked_response(req: &ChatCompletionRequest) -> Json<ChatCompletionResponse> {
    Json(ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: req.model.clone(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: "I cannot assist with that request due to safety policies.".to_string(),
            },
            finish_reason: "content_filter".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    })
}

async fn fallback() -> &'static str {
    "Not found"
}

/// Error handling
#[derive(Debug)]
enum AppError {
    InvalidRequest(String),
    BackendError(StatusCode),
    InternalError(String),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InvalidRequest(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::BackendError(status) => (status, "Backend error".to_string()),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = json!({
            "error": {
                "message": message,
                "type": "invalid_request_error",
            }
        });

        (status, Json(body)).into_response()
    }
}
