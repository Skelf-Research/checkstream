//! Cryptographic audit trail

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::time::SystemTime;

/// Audit trail with hash-chained events for tamper detection
pub struct AuditTrail {
    events: Vec<AuditEvent>,
    chain_hash: Option<String>,
}

impl AuditTrail {
    /// Create a new audit trail
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            chain_hash: None,
        }
    }

    /// Add an event to the audit trail
    pub fn add_event(&mut self, event: AuditEvent) {
        let mut event = event;
        event.previous_hash = self.chain_hash.clone();

        let hash = self.compute_hash(&event);
        event.hash = Some(hash.clone());

        self.chain_hash = Some(hash);
        self.events.push(event);
    }

    /// Verify the integrity of the audit trail
    pub fn verify(&self) -> bool {
        let mut prev_hash: Option<String> = None;

        for event in &self.events {
            // Check if previous hash matches
            if event.previous_hash != prev_hash {
                return false;
            }

            // Recompute hash and verify
            let computed_hash = self.compute_hash(event);
            if event.hash.as_ref() != Some(&computed_hash) {
                return false;
            }

            prev_hash = event.hash.clone();
        }

        true
    }

    /// Get all events
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Compute hash for an event
    fn compute_hash(&self, event: &AuditEvent) -> String {
        let mut hasher = Sha256::new();

        // Hash the event data (excluding the hash field itself)
        hasher.update(event.event_type.as_bytes());
        if let Some(ref data) = event.data {
            hasher.update(data.as_bytes());
        }
        hasher.update(format!("{:?}", event.timestamp).as_bytes());
        if let Some(ref prev) = event.previous_hash {
            hasher.update(prev.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

/// A single audit event in the trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event type/category
    pub event_type: String,

    /// Event data (JSON serialized)
    pub data: Option<String>,

    /// Timestamp
    pub timestamp: SystemTime,

    /// Hash of this event
    pub hash: Option<String>,

    /// Hash of previous event (for chaining)
    pub previous_hash: Option<String>,

    /// Regulation or policy this event relates to
    pub regulation: Option<String>,

    /// Severity level
    pub severity: AuditSeverity,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            data: None,
            timestamp: SystemTime::now(),
            hash: None,
            previous_hash: None,
            regulation: None,
            severity: AuditSeverity::Info,
        }
    }

    /// Set event data
    pub fn with_data(mut self, data: impl Serialize) -> Self {
        self.data = serde_json::to_string(&data).ok();
        self
    }

    /// Set regulation
    pub fn with_regulation(mut self, regulation: impl Into<String>) -> Self {
        self.regulation = Some(regulation.into());
        self
    }

    /// Set severity
    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_trail() {
        let mut trail = AuditTrail::new();

        trail.add_event(AuditEvent::new("policy_triggered"));
        trail.add_event(AuditEvent::new("action_taken"));

        assert!(trail.verify());
        assert_eq!(trail.events().len(), 2);
    }

    #[test]
    fn test_tamper_detection() {
        let mut trail = AuditTrail::new();

        trail.add_event(AuditEvent::new("event1"));
        trail.add_event(AuditEvent::new("event2"));

        // Tamper with an event
        trail.events[0].event_type = "tampered".to_string();

        // Verification should fail
        assert!(!trail.verify());
    }
}
