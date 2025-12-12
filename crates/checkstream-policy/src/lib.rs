//! CheckStream Policy Engine
//!
//! Declarative policy-as-code engine for defining safety, security,
//! and regulatory compliance rules.
//!
//! Policies are defined in YAML and specify:
//! - Triggers (classifiers, patterns, context conditions)
//! - Actions (redact, inject, stop, adapt, log)
//! - Regulatory mappings (FCA, FINRA, MiFID II, etc.)

pub mod action;
pub mod engine;
pub mod executor;
pub mod rule;
pub mod trigger;

pub use action::{Action, ActionType};
pub use engine::{PolicyEngine, EvaluationResult, EvaluationMetadata};
pub use executor::{ActionExecutor, ActionOutcome, AuditRecord, TextModification, apply_modifications};
pub use rule::{Policy, Rule};
pub use trigger::{Trigger, TriggerType};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::action::{Action, ActionType};
    pub use crate::engine::{PolicyEngine, EvaluationResult, EvaluationMetadata};
    pub use crate::executor::{ActionExecutor, ActionOutcome};
    pub use crate::rule::{Policy, Rule};
    pub use crate::trigger::{Trigger, TriggerType};
}
