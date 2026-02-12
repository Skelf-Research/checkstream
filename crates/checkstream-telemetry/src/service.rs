//! Audit service for async audit event processing
//!
//! Provides:
//! - Async interface for recording audit events
//! - Bridging from policy executor AuditRecord to telemetry events
//! - Background persistence with buffering
//! - Query interface for compliance reporting

use crate::audit::{AuditEvent, AuditSeverity as TelemetrySeverity};
use crate::persistence::{
    AuditQuery, AuditReader, AuditWriter, ExportFormat, PersistedAuditEvent, PersistenceConfig,
};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Audit service for managing audit events
pub struct AuditService {
    /// Channel sender for async event recording
    sender: mpsc::UnboundedSender<AuditCommand>,

    /// Reader for queries (can be used directly)
    reader: Arc<AuditReader>,
}

/// Commands sent to the background writer
enum AuditCommand {
    /// Record an event
    Record(Box<PersistedAuditEvent>),

    /// Flush to disk
    Flush,

    /// Shutdown the service
    Shutdown,
}

impl AuditService {
    /// Create a new audit service
    pub fn new(config: PersistenceConfig) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let reader = Arc::new(AuditReader::new(config.clone()));

        // Start background writer task
        let writer_config = config.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_writer(writer_config, receiver) {
                error!("Audit writer thread failed: {}", e);
            }
        });

        info!("Audit service started with dir: {:?}", config.audit_dir);

        Ok(Self { sender, reader })
    }

    /// Record an audit event asynchronously
    pub fn record(&self, event: PersistedAuditEvent) {
        if let Err(e) = self.sender.send(AuditCommand::Record(Box::new(event))) {
            warn!("Failed to send audit event: {}", e);
        }
    }

    /// Record an audit event from policy executor
    pub fn record_from_policy(&self, record: &PolicyAuditRecord, request_context: &RequestContext) {
        let severity = match record.severity {
            PolicySeverity::Low => TelemetrySeverity::Info,
            PolicySeverity::Medium => TelemetrySeverity::Warning,
            PolicySeverity::High => TelemetrySeverity::High,
            PolicySeverity::Critical => TelemetrySeverity::Critical,
        };

        let mut event = AuditEvent::new(&record.category).with_severity(severity);

        if let Some(ref context) = record.context {
            event = event.with_data(serde_json::json!({
                "rule_name": record.rule_name,
                "policy_name": record.policy_name,
                "matched_content": context,
            }));
        } else {
            event = event.with_data(serde_json::json!({
                "rule_name": record.rule_name,
                "policy_name": record.policy_name,
            }));
        }

        // Add regulation if present
        // We'd need to get this from the policy/rule - for now just use category
        let regulation = format!("policy:{}", record.policy_name);
        event = event.with_regulation(regulation);

        let persisted = PersistedAuditEvent::new(event)
            .with_request_id(&request_context.request_id)
            .with_phase(&request_context.phase);

        if let Some(ref session_id) = request_context.session_id {
            self.record(persisted.with_session_id(session_id));
        } else {
            self.record(persisted);
        }
    }

    /// Record a simple event
    pub fn record_event(
        &self,
        event_type: &str,
        severity: TelemetrySeverity,
        request_context: &RequestContext,
        data: Option<serde_json::Value>,
    ) {
        let mut event = AuditEvent::new(event_type).with_severity(severity);

        if let Some(data) = data {
            event = event.with_data(data);
        }

        let mut persisted = PersistedAuditEvent::new(event)
            .with_request_id(&request_context.request_id)
            .with_phase(&request_context.phase);

        if let Some(ref session_id) = request_context.session_id {
            persisted = persisted.with_session_id(session_id);
        }

        if let Some(ref model) = request_context.model {
            persisted = persisted.with_model(model);
        }

        self.record(persisted);
    }

    /// Flush pending events to disk
    pub fn flush(&self) {
        if let Err(e) = self.sender.send(AuditCommand::Flush) {
            warn!("Failed to send flush command: {}", e);
        }
    }

    /// Query audit events
    pub fn query(&self, query: &AuditQuery) -> std::io::Result<Vec<PersistedAuditEvent>> {
        self.reader.query(query)
    }

    /// Count events matching query
    pub fn count(&self, query: &AuditQuery) -> std::io::Result<usize> {
        self.reader.count(query)
    }

    /// Export events to file
    pub fn export(
        &self,
        query: &AuditQuery,
        output_path: &std::path::Path,
        format: ExportFormat,
    ) -> std::io::Result<usize> {
        self.reader.export_to_file(query, output_path, format)
    }

    /// Get audit statistics
    pub fn stats(&self) -> std::io::Result<AuditStats> {
        let total = self.count(&AuditQuery::new())?;
        let critical = self.count(&AuditQuery::new().min_severity(TelemetrySeverity::Critical))?;
        let high = self.count(&AuditQuery::new().min_severity(TelemetrySeverity::High))?;

        // Get recent events (last 24 hours)
        let yesterday = SystemTime::now() - std::time::Duration::from_secs(86400);
        let recent = self.count(&AuditQuery::new().time_range(yesterday, SystemTime::now()))?;

        Ok(AuditStats {
            total_events: total,
            critical_events: critical,
            high_severity_events: high,
            events_last_24h: recent,
        })
    }
}

impl Drop for AuditService {
    fn drop(&mut self) {
        // Signal shutdown
        let _ = self.sender.send(AuditCommand::Shutdown);
    }
}

/// Background writer task
fn run_writer(
    config: PersistenceConfig,
    mut receiver: mpsc::UnboundedReceiver<AuditCommand>,
) -> std::io::Result<()> {
    let mut writer = AuditWriter::new(config)?;

    // Use blocking recv in a thread
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        while let Some(cmd) = receiver.recv().await {
            match cmd {
                AuditCommand::Record(event) => {
                    if let Err(e) = writer.write_event(event.as_ref()) {
                        error!("Failed to write audit event: {}", e);
                    }
                }
                AuditCommand::Flush => {
                    if let Err(e) = writer.flush() {
                        error!("Failed to flush audit writer: {}", e);
                    }
                }
                AuditCommand::Shutdown => {
                    debug!("Audit writer shutting down");
                    let _ = writer.flush();
                    break;
                }
            }
        }
    });

    Ok(())
}

/// Request context for audit events
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Unique request ID
    pub request_id: String,

    /// Session ID if applicable
    pub session_id: Option<String>,

    /// Processing phase (ingress/midstream/egress)
    pub phase: String,

    /// Model being used
    pub model: Option<String>,

    /// Source IP hash (for privacy)
    pub source_ip_hash: Option<String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(request_id: impl Into<String>, phase: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            phase: phase.into(),
            ..Default::default()
        }
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Policy audit record (matches checkstream-policy::executor::AuditRecord)
#[derive(Debug, Clone)]
pub struct PolicyAuditRecord {
    pub rule_name: String,
    pub policy_name: String,
    pub category: String,
    pub severity: PolicySeverity,
    pub context: Option<String>,
}

/// Policy severity levels (matches checkstream-policy::action::AuditSeverity)
#[derive(Debug, Clone, Copy)]
pub enum PolicySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Audit statistics
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub critical_events: usize,
    pub high_severity_events: usize,
    pub events_last_24h: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(dir: &std::path::Path) -> PersistenceConfig {
        PersistenceConfig {
            audit_dir: dir.to_path_buf(),
            max_file_size: 1024 * 1024,
            max_file_age_secs: 3600,
            retention_days: 7,
            flush_interval: 1,
            compress_rotated: false,
        }
    }

    #[tokio::test]
    async fn test_audit_service_record_and_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(temp_dir.path());

        let service = AuditService::new(config).unwrap();

        // Record events
        let ctx = RequestContext::new("req-001", "ingress");

        service.record_event(
            "test_event",
            TelemetrySeverity::High,
            &ctx,
            Some(serde_json::json!({"test": "data"})),
        );

        // Give writer time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        service.flush();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Query events
        let events = service.query(&AuditQuery::new()).unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].event.event_type, "test_event");
    }

    #[tokio::test]
    async fn test_record_from_policy() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(temp_dir.path());

        let service = AuditService::new(config).unwrap();

        let policy_record = PolicyAuditRecord {
            rule_name: "test-rule".to_string(),
            policy_name: "test-policy".to_string(),
            category: "financial_advice".to_string(),
            severity: PolicySeverity::High,
            context: Some("matched content".to_string()),
        };

        let ctx = RequestContext::new("req-002", "egress").with_model("gpt-4");

        service.record_from_policy(&policy_record, &ctx);

        // Give writer time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        service.flush();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Query events
        let events = service.query(&AuditQuery::new()).unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].event.event_type, "financial_advice");
    }
}
