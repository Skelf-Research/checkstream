//! CheckStream Telemetry
//!
//! Telemetry, metrics, and audit trail functionality for CheckStream.
//!
//! Provides:
//! - Cryptographic audit trails for regulatory compliance
//! - Performance metrics and monitoring
//! - Event logging and aggregation

pub mod audit;
pub mod metrics;

pub use audit::{AuditTrail, AuditEvent};
pub use metrics::MetricsCollector;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::audit::{AuditTrail, AuditEvent};
    pub use crate::metrics::MetricsCollector;
}
