use crate::models::{DemoEvent, IssueConfig, TrafficConfig, TrafficState};
use crate::state::DemoAppState;
use crate::traffic::TrafficGenerator;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

// ============================================================================
// Health endpoints
// ============================================================================

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// ============================================================================
// Configuration endpoints
// ============================================================================

pub async fn get_config(State(state): State<DemoAppState>) -> impl IntoResponse {
    let config = state.config.read().clone();
    Json(config)
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub policy_path: Option<String>,
    pub classifiers_path: Option<String>,
}

pub async fn update_config(
    State(state): State<DemoAppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let mut config = state.config.write();
    if let Some(policy_path) = req.policy_path {
        config.policy_path = policy_path;
    }
    if let Some(classifiers_path) = req.classifiers_path {
        config.classifiers_path = classifiers_path;
    }
    Json(serde_json::json!({ "status": "updated" }))
}

pub async fn get_issue_config(State(state): State<DemoAppState>) -> impl IntoResponse {
    let config = state.mock_backend.get_issue_config();
    Json(config)
}

pub async fn update_issue_config(
    State(state): State<DemoAppState>,
    Json(config): Json<IssueConfig>,
) -> impl IntoResponse {
    state.mock_backend.set_issue_config(config.clone());

    // Notify clients
    state
        .event_bus
        .publish(DemoEvent::ConfigChanged(crate::models::ConfigChangeEvent {
            field: "issue_config".to_string(),
            old_value: serde_json::json!({}),
            new_value: serde_json::to_value(&config).unwrap_or_default(),
        }));

    Json(serde_json::json!({ "status": "updated" }))
}

// ============================================================================
// Traffic generation endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartTrafficRequest {
    pub rate: Option<u32>,
    pub duration_secs: Option<u64>,
    pub issue_config: Option<IssueConfig>,
}

pub async fn start_traffic(
    State(state): State<DemoAppState>,
    Json(req): Json<StartTrafficRequest>,
) -> impl IntoResponse {
    // Update issue config if provided
    if let Some(issue_config) = req.issue_config {
        state.mock_backend.set_issue_config(issue_config);
    }

    // Try to start traffic
    let stop_signal = match state.traffic_controller.start() {
        Some(signal) => signal,
        None => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "Traffic generation already running" })),
            )
        }
    };

    // Notify clients
    state
        .event_bus
        .publish(DemoEvent::TrafficStateChanged(TrafficState::Running));

    // Spawn traffic generator
    let generator = TrafficGenerator::new(Arc::new(state.clone()));
    let config = TrafficConfig {
        rate: req.rate.unwrap_or(10),
        duration_secs: req.duration_secs,
        templates: vec!["general".to_string(), "coding".to_string()],
    };

    tokio::spawn(async move {
        generator.run(config, stop_signal).await;
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "started" })),
    )
}

pub async fn stop_traffic(State(state): State<DemoAppState>) -> impl IntoResponse {
    state.traffic_controller.stop();
    state
        .event_bus
        .publish(DemoEvent::TrafficStateChanged(TrafficState::Stopped));
    Json(serde_json::json!({ "status": "stopped" }))
}

pub async fn traffic_status(State(state): State<DemoAppState>) -> impl IntoResponse {
    let status = state.traffic_controller.state();
    Json(serde_json::json!({ "state": status }))
}

// ============================================================================
// Statistics endpoints
// ============================================================================

pub async fn get_stats(State(state): State<DemoAppState>) -> impl IntoResponse {
    let stats = state.metrics.stats();
    Json(stats)
}

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub window_minutes: Option<i64>,
}

pub async fn get_heatmap(
    State(state): State<DemoAppState>,
    Query(query): Query<HeatmapQuery>,
) -> impl IntoResponse {
    let window = query.window_minutes.unwrap_or(60);
    let heatmap = state.metrics.heatmap(window);
    Json(heatmap)
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub window_minutes: Option<u32>,
}

pub async fn get_timeline(
    State(state): State<DemoAppState>,
    Query(query): Query<TimelineQuery>,
) -> impl IntoResponse {
    let window = query.window_minutes.unwrap_or(60);
    let timeline = state.metrics.timeline(window);
    Json(timeline)
}

// ============================================================================
// Events/History endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub action: Option<String>,
    pub issue_type: Option<String>,
    pub limit: Option<usize>,
}

pub async fn list_events(
    State(state): State<DemoAppState>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);
    let records = state.get_recent_requests(limit);

    // Filter if needed
    let filtered: Vec<_> = records
        .into_iter()
        .filter(|r| {
            if let Some(ref action) = query.action {
                let action_str = format!("{:?}", r.result.action).to_lowercase();
                if !action_str.contains(&action.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref issue_type) = query.issue_type {
                if !r
                    .result
                    .issues_detected
                    .iter()
                    .any(|i| i.issue_type.contains(issue_type))
                {
                    return false;
                }
            }
            true
        })
        .collect();

    Json(filtered)
}

pub async fn get_event(
    State(state): State<DemoAppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_request_record(&id) {
        Some(record) => (StatusCode::OK, Json(serde_json::to_value(record).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Event not found" })),
        ),
    }
}

// ============================================================================
// Policy endpoints (placeholder for future integration)
// ============================================================================

pub async fn list_policies() -> impl IntoResponse {
    Json(serde_json::json!({
        "policies": [
            {
                "name": "default",
                "description": "Default safety policy",
                "rules_count": 5,
                "enabled": true
            }
        ]
    }))
}

pub async fn get_policy(Path(name): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "name": name,
        "description": "Policy description",
        "rules": [
            { "name": "pii-detection", "enabled": true },
            { "name": "toxicity-filter", "enabled": true },
            { "name": "prompt-injection-defense", "enabled": true }
        ]
    }))
}

// ============================================================================
// Metrics reset endpoint
// ============================================================================

pub async fn reset_metrics(State(state): State<DemoAppState>) -> impl IntoResponse {
    state.metrics.reset();
    Json(serde_json::json!({ "status": "reset" }))
}
