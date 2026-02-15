//! Audit trail persistence layer
//!
//! Provides file-based persistence for audit events with:
//! - JSON-lines format for append-only writes
//! - Automatic rotation based on size/time
//! - Query and filter capabilities
//! - Export functionality for compliance reports

use crate::audit::{AuditEvent, AuditSeverity, AuditTrail};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Configuration for audit persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Directory to store audit files
    pub audit_dir: PathBuf,

    /// Maximum file size before rotation (bytes)
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Maximum age before rotation (seconds)
    #[serde(default = "default_max_file_age")]
    pub max_file_age_secs: u64,

    /// Retain files for this many days
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Flush to disk after this many events
    #[serde(default = "default_flush_interval")]
    pub flush_interval: usize,

    /// Enable compression for rotated files
    #[serde(default)]
    pub compress_rotated: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            audit_dir: PathBuf::from("./audit"),
            max_file_size: default_max_file_size(),
            max_file_age_secs: default_max_file_age(),
            retention_days: default_retention_days(),
            flush_interval: default_flush_interval(),
            compress_rotated: false,
        }
    }
}

fn default_max_file_size() -> u64 {
    100 * 1024 * 1024 // 100MB
}

fn default_max_file_age() -> u64 {
    86400 // 24 hours
}

fn default_retention_days() -> u32 {
    90 // 90 days for regulatory compliance
}

fn default_flush_interval() -> usize {
    10 // Flush every 10 events
}

/// Persisted audit event with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuditEvent {
    /// Unique event ID
    pub id: String,

    /// Request ID this event belongs to
    pub request_id: Option<String>,

    /// Session ID if applicable
    pub session_id: Option<String>,

    /// The core audit event
    #[serde(flatten)]
    pub event: AuditEvent,

    /// Source IP (hashed for privacy)
    pub source_ip_hash: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Model being used
    pub model: Option<String>,

    /// Phase where event occurred (ingress/midstream/egress)
    pub phase: Option<String>,
}

impl PersistedAuditEvent {
    /// Create a new persisted event from an audit event
    pub fn new(event: AuditEvent) -> Self {
        Self {
            id: generate_event_id(),
            request_id: None,
            session_id: None,
            event,
            source_ip_hash: None,
            user_agent: None,
            model: None,
            phase: None,
        }
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set phase
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Generate a cryptographically secure unique event ID using UUID v4
fn generate_event_id() -> String {
    format!("evt_{}", uuid::Uuid::new_v4())
}

/// Audit file writer with rotation support
pub struct AuditWriter {
    config: PersistenceConfig,
    current_file: Option<BufWriter<File>>,
    current_path: Option<PathBuf>,
    current_size: u64,
    current_start: SystemTime,
    events_since_flush: usize,
    trail: AuditTrail,
}

impl AuditWriter {
    /// Create a new audit writer
    pub fn new(config: PersistenceConfig) -> std::io::Result<Self> {
        // Ensure audit directory exists
        std::fs::create_dir_all(&config.audit_dir)?;

        let mut writer = Self {
            config,
            current_file: None,
            current_path: None,
            current_size: 0,
            current_start: SystemTime::now(),
            events_since_flush: 0,
            trail: AuditTrail::new(),
        };

        writer.open_new_file()?;
        Ok(writer)
    }

    /// Write an event to the audit log
    pub fn write_event(&mut self, event: &PersistedAuditEvent) -> std::io::Result<()> {
        // Check if rotation is needed
        if self.should_rotate() {
            self.rotate()?;
        }

        // Hash-chain each event before writing it to disk.
        let mut event = event.clone();
        event.event = self.trail.chain_event(event.event);

        // Serialize event to JSON line
        let json = serde_json::to_string(&event)?;
        let line = format!("{}\n", json);
        let bytes = line.as_bytes();

        // Write to file
        if let Some(ref mut writer) = self.current_file {
            writer.write_all(bytes)?;
            self.current_size += bytes.len() as u64;
            self.events_since_flush += 1;

            // Flush if needed
            if self.events_since_flush >= self.config.flush_interval {
                writer.flush()?;
                self.events_since_flush = 0;
            }
        }

        Ok(())
    }

    /// Force flush to disk
    pub fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut writer) = self.current_file {
            writer.flush()?;
            self.events_since_flush = 0;
        }
        Ok(())
    }

    /// Check if rotation is needed
    fn should_rotate(&self) -> bool {
        // Check size
        if self.current_size >= self.config.max_file_size {
            return true;
        }

        // Check age
        let age = SystemTime::now()
            .duration_since(self.current_start)
            .unwrap_or_default();
        if age.as_secs() >= self.config.max_file_age_secs {
            return true;
        }

        false
    }

    /// Rotate to a new file
    fn rotate(&mut self) -> std::io::Result<()> {
        // Flush and close current file
        if let Some(ref mut writer) = self.current_file {
            writer.flush()?;
        }
        self.current_file = None;

        // Rename current file with timestamp
        if let Some(ref current_path) = self.current_path {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let rotated_name = format!("audit_{}.jsonl", timestamp);
            let rotated_path = self.config.audit_dir.join(&rotated_name);

            if let Err(e) = std::fs::rename(current_path, &rotated_path) {
                warn!("Failed to rotate audit file: {}", e);
            } else {
                info!("Rotated audit file to: {:?}", rotated_path);
            }
        }

        // Open new file
        self.open_new_file()?;

        // Cleanup old files
        if let Err(e) = self.cleanup_old_files() {
            warn!("Failed to cleanup old audit files: {}", e);
        }

        Ok(())
    }

    /// Open a new audit file
    fn open_new_file(&mut self) -> std::io::Result<()> {
        let path = self.config.audit_dir.join("audit_current.jsonl");

        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        let metadata = file.metadata()?;
        self.current_size = metadata.len();
        self.current_start = SystemTime::now();
        self.current_file = Some(BufWriter::new(file));
        self.current_path = Some(path);
        self.events_since_flush = 0;

        Ok(())
    }

    /// Cleanup old audit files beyond retention period
    fn cleanup_old_files(&self) -> std::io::Result<()> {
        let retention_secs = self.config.retention_days as u64 * 86400;
        let cutoff = SystemTime::now() - Duration::from_secs(retention_secs);

        for entry in std::fs::read_dir(&self.config.audit_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip current file
            if path.file_name().is_some_and(|n| n == "audit_current.jsonl") {
                continue;
            }

            // Check modification time
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        info!("Removing old audit file: {:?}", path);
                        std::fs::remove_file(&path)?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Query filter for audit events
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by event type
    pub event_type: Option<String>,

    /// Filter by request ID
    pub request_id: Option<String>,

    /// Filter by phase (ingress/midstream/egress)
    pub phase: Option<String>,

    /// Filter by minimum severity
    pub min_severity: Option<AuditSeverity>,

    /// Filter by regulation
    pub regulation: Option<String>,

    /// Start time filter
    pub start_time: Option<SystemTime>,

    /// End time filter
    pub end_time: Option<SystemTime>,

    /// Maximum results to return
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

impl AuditQuery {
    /// Create a new empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by event type
    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    /// Filter by request ID
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Filter by phase
    pub fn phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    /// Filter by minimum severity
    pub fn min_severity(mut self, severity: AuditSeverity) -> Self {
        self.min_severity = Some(severity);
        self
    }

    /// Filter by regulation
    pub fn regulation(mut self, regulation: impl Into<String>) -> Self {
        self.regulation = Some(regulation.into());
        self
    }

    /// Set time range
    pub fn time_range(mut self, start: SystemTime, end: SystemTime) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Set limit and offset
    pub fn paginate(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// Set just limit (for convenience)
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Audit reader for querying persisted events
pub struct AuditReader {
    config: PersistenceConfig,
}

impl AuditReader {
    /// Create a new audit reader
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// Query audit events
    pub fn query(&self, query: &AuditQuery) -> std::io::Result<Vec<PersistedAuditEvent>> {
        let mut results = Vec::new();
        let mut files_to_read = Vec::new();

        // Collect all audit files
        for entry in std::fs::read_dir(&self.config.audit_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "jsonl") {
                files_to_read.push(path);
            }
        }

        // Sort by modification time (oldest first)
        files_to_read.sort();

        // Read and filter events
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(1000);
        let mut skipped = 0;

        for file_path in files_to_read {
            let file = File::open(&file_path)?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line?;
                if line.is_empty() {
                    continue;
                }

                match serde_json::from_str::<PersistedAuditEvent>(&line) {
                    Ok(event) => {
                        if self.matches_query(&event, query) {
                            if skipped < offset {
                                skipped += 1;
                                continue;
                            }

                            results.push(event);

                            if results.len() >= limit {
                                return Ok(results);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse audit event: {}", e);
                        continue;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check if an event matches the query
    fn matches_query(&self, event: &PersistedAuditEvent, query: &AuditQuery) -> bool {
        // Event type filter
        if let Some(ref event_type) = query.event_type {
            if &event.event.event_type != event_type {
                return false;
            }
        }

        // Request ID filter
        if let Some(ref request_id) = query.request_id {
            if event.request_id.as_ref() != Some(request_id) {
                return false;
            }
        }

        // Phase filter
        if let Some(ref phase) = query.phase {
            if event.phase.as_ref() != Some(phase) {
                return false;
            }
        }

        // Severity filter
        if let Some(ref min_severity) = query.min_severity {
            if !severity_gte(&event.event.severity, min_severity) {
                return false;
            }
        }

        // Regulation filter
        if let Some(ref regulation) = query.regulation {
            if event.event.regulation.as_ref() != Some(regulation) {
                return false;
            }
        }

        // Time range filter
        if let Some(start) = query.start_time {
            if event.event.timestamp < start {
                return false;
            }
        }

        if let Some(end) = query.end_time {
            if event.event.timestamp > end {
                return false;
            }
        }

        true
    }

    /// Count total events matching query
    pub fn count(&self, query: &AuditQuery) -> std::io::Result<usize> {
        let mut count = 0;

        for entry in std::fs::read_dir(&self.config.audit_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "jsonl") {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                for line in reader.lines() {
                    let line = line?;
                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<PersistedAuditEvent>(&line) {
                        if self.matches_query(&event, query) {
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Export events to a file for compliance reporting
    pub fn export_to_file(
        &self,
        query: &AuditQuery,
        output_path: &Path,
        format: ExportFormat,
    ) -> std::io::Result<usize> {
        let events = self.query(query)?;
        let count = events.len();

        let mut file = File::create(output_path)?;

        match format {
            ExportFormat::JsonLines => {
                for event in &events {
                    let json = serde_json::to_string(event)?;
                    writeln!(file, "{}", json)?;
                }
            }
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&events)?;
                write!(file, "{}", json)?;
            }
            ExportFormat::Csv => {
                // Write CSV header
                writeln!(
                    file,
                    "id,request_id,event_type,severity,regulation,timestamp,phase,data"
                )?;

                for event in &events {
                    let timestamp = event
                        .event
                        .timestamp
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    writeln!(
                        file,
                        "{},{},{},{:?},{},{},{},{}",
                        event.id,
                        event.request_id.as_deref().unwrap_or(""),
                        event.event.event_type,
                        event.event.severity,
                        event.event.regulation.as_deref().unwrap_or(""),
                        timestamp,
                        event.phase.as_deref().unwrap_or(""),
                        event.event.data.as_deref().unwrap_or("").replace(',', ";")
                    )?;
                }
            }
        }

        Ok(count)
    }
}

/// Export format options
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON Lines format (one JSON object per line)
    JsonLines,
    /// Pretty-printed JSON array
    Json,
    /// CSV format
    Csv,
}

/// Compare severity levels
fn severity_gte(a: &AuditSeverity, b: &AuditSeverity) -> bool {
    let a_val = match a {
        AuditSeverity::Info => 0,
        AuditSeverity::Warning => 1,
        AuditSeverity::High => 2,
        AuditSeverity::Critical => 3,
    };
    let b_val = match b {
        AuditSeverity::Info => 0,
        AuditSeverity::Warning => 1,
        AuditSeverity::High => 2,
        AuditSeverity::Critical => 3,
    };
    a_val >= b_val
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(dir: &Path) -> PersistenceConfig {
        PersistenceConfig {
            audit_dir: dir.to_path_buf(),
            max_file_size: 1024 * 1024, // 1MB
            max_file_age_secs: 3600,
            retention_days: 7,
            flush_interval: 1,
            compress_rotated: false,
        }
    }

    #[test]
    fn test_write_and_read_events() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(temp_dir.path());

        // Write events
        {
            let mut writer = AuditWriter::new(config.clone()).unwrap();

            let event1 = PersistedAuditEvent::new(
                AuditEvent::new("policy_triggered").with_severity(AuditSeverity::High),
            )
            .with_request_id("req-001");

            let event2 = PersistedAuditEvent::new(
                AuditEvent::new("action_taken").with_severity(AuditSeverity::Info),
            )
            .with_request_id("req-001");

            writer.write_event(&event1).unwrap();
            writer.write_event(&event2).unwrap();
            writer.flush().unwrap();
        }

        // Read events
        let reader = AuditReader::new(config);
        let events = reader.query(&AuditQuery::new()).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.event_type, "policy_triggered");
        assert_eq!(events[1].event.event_type, "action_taken");
    }

    #[test]
    fn test_query_filters() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(temp_dir.path());

        // Write events
        {
            let mut writer = AuditWriter::new(config.clone()).unwrap();

            for i in 0..10 {
                let severity = if i % 2 == 0 {
                    AuditSeverity::High
                } else {
                    AuditSeverity::Info
                };

                let event = PersistedAuditEvent::new(
                    AuditEvent::new(format!("event_{}", i)).with_severity(severity),
                )
                .with_request_id(format!("req-{}", i % 3));

                writer.write_event(&event).unwrap();
            }
            writer.flush().unwrap();
        }

        let reader = AuditReader::new(config);

        // Query high severity only
        let high_events = reader
            .query(&AuditQuery::new().min_severity(AuditSeverity::High))
            .unwrap();
        assert_eq!(high_events.len(), 5);

        // Query by request ID
        let req0_events = reader
            .query(&AuditQuery::new().request_id("req-0"))
            .unwrap();
        assert_eq!(req0_events.len(), 4); // 0, 3, 6, 9

        // Query with pagination
        let page1 = reader.query(&AuditQuery::new().paginate(3, 0)).unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = reader.query(&AuditQuery::new().paginate(3, 3)).unwrap();
        assert_eq!(page2.len(), 3);
    }

    #[test]
    fn test_export_csv() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(temp_dir.path());

        // Write events
        {
            let mut writer = AuditWriter::new(config.clone()).unwrap();

            let event = PersistedAuditEvent::new(
                AuditEvent::new("test_event").with_regulation("FCA COBS 9A"),
            )
            .with_request_id("req-export");

            writer.write_event(&event).unwrap();
            writer.flush().unwrap();
        }

        // Export to CSV
        let reader = AuditReader::new(config);
        let export_path = temp_dir.path().join("export.csv");
        let count = reader
            .export_to_file(&AuditQuery::new(), &export_path, ExportFormat::Csv)
            .unwrap();

        assert_eq!(count, 1);
        assert!(export_path.exists());

        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("test_event"));
        assert!(content.contains("FCA COBS 9A"));
    }
}
