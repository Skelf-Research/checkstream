use crate::mock::MockLlmBackend;
use crate::models::{DemoConfig, RequestRecord};
use crate::state::{EventBus, MetricsCollector};
use crate::traffic::TrafficController;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;

const MAX_REQUEST_HISTORY: usize = 1000;

/// Shared application state
#[derive(Clone)]
pub struct DemoAppState {
    /// Demo configuration
    pub config: Arc<RwLock<DemoConfig>>,

    /// Real-time event bus for WebSocket broadcasting
    pub event_bus: Arc<EventBus>,

    /// Metrics collector for dashboard stats
    pub metrics: Arc<MetricsCollector>,

    /// Traffic generator control
    pub traffic_controller: Arc<TrafficController>,

    /// Mock LLM backend (when in mock mode)
    pub mock_backend: Arc<MockLlmBackend>,

    /// Request history for inspector
    pub request_history: Arc<RwLock<VecDeque<RequestRecord>>>,
}

impl DemoAppState {
    pub fn new(config: DemoConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            event_bus: Arc::new(EventBus::default()),
            metrics: Arc::new(MetricsCollector::new()),
            traffic_controller: Arc::new(TrafficController::new()),
            mock_backend: Arc::new(MockLlmBackend::new()),
            request_history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_REQUEST_HISTORY))),
        }
    }

    /// Add a request record to history
    pub fn add_request_record(&self, record: RequestRecord) {
        let mut history = self.request_history.write();
        history.push_front(record);
        if history.len() > MAX_REQUEST_HISTORY {
            history.pop_back();
        }
    }

    /// Get a request record by ID
    pub fn get_request_record(&self, id: &str) -> Option<RequestRecord> {
        let history = self.request_history.read();
        history.iter().find(|r| r.id == id).cloned()
    }

    /// Get recent request records
    pub fn get_recent_requests(&self, limit: usize) -> Vec<RequestRecord> {
        let history = self.request_history.read();
        history.iter().take(limit).cloned().collect()
    }
}
