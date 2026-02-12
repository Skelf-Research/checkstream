//! CheckStream Telemetry
//!
//! Telemetry, metrics, and audit trail functionality for CheckStream.
//!
//! Provides:
//! - Cryptographic audit trails for regulatory compliance
//! - Persistent audit storage with query and export
//! - Performance metrics and monitoring
//! - Event logging and aggregation

pub mod audit;
pub mod metrics;
pub mod persistence;
pub mod service;

pub use audit::{AuditEvent, AuditSeverity, AuditTrail};
pub use metrics::MetricsCollector;
pub use persistence::{
    AuditQuery, AuditReader, AuditWriter, ExportFormat, PersistedAuditEvent, PersistenceConfig,
};
pub use service::{AuditService, AuditStats, PolicyAuditRecord, PolicySeverity, RequestContext};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::audit::{AuditEvent, AuditSeverity, AuditTrail};
    pub use crate::metrics::MetricsCollector;
    pub use crate::persistence::{AuditQuery, ExportFormat, PersistenceConfig};
    pub use crate::service::{AuditService, RequestContext};
}
