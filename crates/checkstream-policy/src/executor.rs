//! Action executor for policy enforcement
//!
//! Executes actions returned from policy evaluation, handling:
//! - Logging with appropriate levels
//! - Stopping streams with status codes
//! - Redacting sensitive content
//! - Injecting warnings or disclaimers
//! - Adapting generation parameters
//! - Audit trail recording

use crate::action::{Action, AuditSeverity, InjectPosition, LogLevel};
use crate::engine::EvaluationResult;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

/// Outcome of executing actions
#[derive(Debug, Clone, Default)]
pub struct ActionOutcome {
    /// Whether to stop the stream
    pub should_stop: bool,

    /// Message to return if stopping
    pub stop_message: Option<String>,

    /// HTTP status code if stopping
    pub stop_status: Option<u16>,

    /// Text modifications to apply
    pub modifications: Vec<TextModification>,

    /// Audit records to persist
    pub audit_records: Vec<AuditRecord>,

    /// Parameter adaptations for generation
    pub adaptations: Vec<ParameterAdaptation>,
}

impl ActionOutcome {
    /// Create a new empty outcome
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any action was taken
    pub fn has_actions(&self) -> bool {
        self.should_stop
            || !self.modifications.is_empty()
            || !self.audit_records.is_empty()
            || !self.adaptations.is_empty()
    }

    /// Merge another outcome into this one
    pub fn merge(&mut self, other: ActionOutcome) {
        // Stop takes precedence
        if other.should_stop {
            self.should_stop = true;
            self.stop_message = other.stop_message.or(self.stop_message.take());
            self.stop_status = other.stop_status.or(self.stop_status.take());
        }

        self.modifications.extend(other.modifications);
        self.audit_records.extend(other.audit_records);
        self.adaptations.extend(other.adaptations);
    }
}

/// A text modification to apply
#[derive(Debug, Clone)]
pub struct TextModification {
    /// Type of modification
    pub kind: ModificationKind,

    /// Content for the modification
    pub content: String,

    /// Position for injection (if applicable)
    pub position: Option<InjectPosition>,

    /// Span to modify (start, end) - if applicable
    pub span: Option<(usize, usize)>,
}

/// Kind of text modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModificationKind {
    /// Redact matched content
    Redact,
    /// Inject additional content
    Inject,
}

/// An audit record to persist
#[derive(Debug, Clone)]
pub struct AuditRecord {
    /// Rule that triggered this
    pub rule_name: String,

    /// Policy the rule belongs to
    pub policy_name: String,

    /// Category for auditing
    pub category: String,

    /// Severity level
    pub severity: AuditSeverity,

    /// Additional context
    pub context: Option<String>,

    /// Timestamp (will be set by persistence layer)
    pub timestamp: Option<SystemTime>,
}

/// Parameter adaptation for generation
#[derive(Debug, Clone)]
pub struct ParameterAdaptation {
    /// Parameter name
    pub parameter: String,

    /// New value
    pub value: f32,

    /// Reason for adaptation
    pub reason: String,
}

/// Action executor
pub struct ActionExecutor {
    /// Whether to record audit for all evaluations
    pub audit_all: bool,
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new() -> Self {
        Self { audit_all: false }
    }

    /// Enable auditing for all evaluations
    pub fn with_audit_all(mut self) -> Self {
        self.audit_all = true;
        self
    }

    /// Execute actions from evaluation results
    pub fn execute(&self, results: &[EvaluationResult]) -> ActionOutcome {
        let mut outcome = ActionOutcome::new();

        for result in results {
            let result_outcome = self.execute_result(result);
            outcome.merge(result_outcome);
        }

        outcome
    }

    /// Execute actions from a single evaluation result
    fn execute_result(&self, result: &EvaluationResult) -> ActionOutcome {
        let mut outcome = ActionOutcome::new();

        for action in &result.actions {
            match action {
                Action::Log { message, level } => {
                    self.execute_log(message, *level, &result.rule_name);
                }

                Action::Stop {
                    message,
                    status_code,
                } => {
                    outcome.should_stop = true;
                    outcome.stop_message = message.clone();
                    outcome.stop_status = Some(*status_code);

                    warn!(
                        rule = %result.rule_name,
                        policy = %result.policy_name,
                        status = %status_code,
                        "Stopping stream due to policy violation"
                    );
                }

                Action::Redact { replacement } => {
                    // If we have span info from the evaluation, use it
                    let span = result.metadata.matched_content.as_ref().map(|_| {
                        // Use token indices if available
                        if !result.metadata.token_indices.is_empty() {
                            let start = *result.metadata.token_indices.first().unwrap_or(&0);
                            let end = *result.metadata.token_indices.last().unwrap_or(&0) + 1;
                            (start, end)
                        } else {
                            (0, 0) // Will need to search for the content
                        }
                    });

                    outcome.modifications.push(TextModification {
                        kind: ModificationKind::Redact,
                        content: replacement.clone(),
                        position: None,
                        span,
                    });

                    debug!(
                        rule = %result.rule_name,
                        replacement = %replacement,
                        "Redacting content"
                    );
                }

                Action::Inject { content, position } => {
                    outcome.modifications.push(TextModification {
                        kind: ModificationKind::Inject,
                        content: content.clone(),
                        position: Some(*position),
                        span: None,
                    });

                    debug!(
                        rule = %result.rule_name,
                        position = ?position,
                        "Injecting content"
                    );
                }

                Action::Adapt { parameter, value } => {
                    let param_name = format!("{:?}", parameter).to_lowercase();
                    outcome.adaptations.push(ParameterAdaptation {
                        parameter: param_name.clone(),
                        value: *value,
                        reason: format!("Rule '{}' adaptation", result.rule_name),
                    });

                    info!(
                        rule = %result.rule_name,
                        parameter = %param_name,
                        value = %value,
                        "Adapting generation parameter"
                    );
                }

                Action::Audit { category, severity } => {
                    outcome.audit_records.push(AuditRecord {
                        rule_name: result.rule_name.clone(),
                        policy_name: result.policy_name.clone(),
                        category: category.clone(),
                        severity: *severity,
                        context: result.metadata.matched_content.clone(),
                        timestamp: Some(SystemTime::now()),
                    });

                    match severity {
                        AuditSeverity::Critical => error!(
                            rule = %result.rule_name,
                            category = %category,
                            "CRITICAL audit event"
                        ),
                        AuditSeverity::High => warn!(
                            rule = %result.rule_name,
                            category = %category,
                            "High severity audit event"
                        ),
                        AuditSeverity::Medium => info!(
                            rule = %result.rule_name,
                            category = %category,
                            "Medium severity audit event"
                        ),
                        AuditSeverity::Low => debug!(
                            rule = %result.rule_name,
                            category = %category,
                            "Low severity audit event"
                        ),
                    }
                }
            }
        }

        outcome
    }

    /// Execute a log action
    fn execute_log(&self, message: &str, level: LogLevel, rule_name: &str) {
        match level {
            LogLevel::Debug => debug!(rule = %rule_name, "{}", message),
            LogLevel::Info => info!(rule = %rule_name, "{}", message),
            LogLevel::Warn => warn!(rule = %rule_name, "{}", message),
            LogLevel::Error => error!(rule = %rule_name, "{}", message),
        }
    }
}

/// Apply text modifications to content
pub fn apply_modifications(text: &str, modifications: &[TextModification]) -> String {
    let mut result = text.to_string();

    // Sort modifications by position (reverse order for replacements to work correctly)
    let mut sorted_mods: Vec<&TextModification> = modifications.iter().collect();
    sorted_mods.sort_by(|a, b| match (&a.span, &b.span) {
        (Some((a_start, _)), Some((b_start, _))) => b_start.cmp(a_start),
        _ => std::cmp::Ordering::Equal,
    });

    for modification in sorted_mods {
        match modification.kind {
            ModificationKind::Redact => {
                if let Some((start, end)) = modification.span {
                    if start < result.len() && end <= result.len() && start < end {
                        result = format!(
                            "{}{}{}",
                            &result[..start],
                            &modification.content,
                            &result[end..]
                        );
                    }
                }
            }
            ModificationKind::Inject => match modification.position {
                Some(InjectPosition::Before) => {
                    result = format!("{}{}", modification.content, result);
                }
                Some(InjectPosition::After) | None => {
                    result = format!("{}{}", result, modification.content);
                }
                Some(InjectPosition::Replace) => {
                    result = modification.content.clone();
                }
            },
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AdaptParameter;

    fn create_test_result(actions: Vec<Action>) -> EvaluationResult {
        EvaluationResult {
            rule_name: "test-rule".to_string(),
            policy_name: "test-policy".to_string(),
            actions,
            score: 0.9,
            metadata: crate::engine::EvaluationMetadata::default(),
        }
    }

    #[test]
    fn test_log_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Log {
            message: "Test message".to_string(),
            level: LogLevel::Info,
        }]);

        let outcome = executor.execute(&[result]);
        // Log actions don't produce visible outcome
        assert!(!outcome.should_stop);
        assert!(outcome.modifications.is_empty());
    }

    #[test]
    fn test_stop_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Stop {
            message: Some("Blocked".to_string()),
            status_code: 403,
        }]);

        let outcome = executor.execute(&[result]);
        assert!(outcome.should_stop);
        assert_eq!(outcome.stop_message, Some("Blocked".to_string()));
        assert_eq!(outcome.stop_status, Some(403));
    }

    #[test]
    fn test_redact_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Redact {
            replacement: "[REDACTED]".to_string(),
        }]);

        let outcome = executor.execute(&[result]);
        assert_eq!(outcome.modifications.len(), 1);
        assert_eq!(outcome.modifications[0].kind, ModificationKind::Redact);
    }

    #[test]
    fn test_inject_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Inject {
            content: "WARNING: ".to_string(),
            position: InjectPosition::Before,
        }]);

        let outcome = executor.execute(&[result]);
        assert_eq!(outcome.modifications.len(), 1);
        assert_eq!(outcome.modifications[0].kind, ModificationKind::Inject);
        assert_eq!(
            outcome.modifications[0].position,
            Some(InjectPosition::Before)
        );
    }

    #[test]
    fn test_adapt_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Adapt {
            parameter: AdaptParameter::Temperature,
            value: 0.5,
        }]);

        let outcome = executor.execute(&[result]);
        assert_eq!(outcome.adaptations.len(), 1);
        assert_eq!(outcome.adaptations[0].value, 0.5);
    }

    #[test]
    fn test_audit_action() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![Action::Audit {
            category: "financial_advice".to_string(),
            severity: AuditSeverity::High,
        }]);

        let outcome = executor.execute(&[result]);
        assert_eq!(outcome.audit_records.len(), 1);
        assert_eq!(outcome.audit_records[0].category, "financial_advice");
    }

    #[test]
    fn test_multiple_actions() {
        let executor = ActionExecutor::new();
        let result = create_test_result(vec![
            Action::Log {
                message: "Issue detected".to_string(),
                level: LogLevel::Warn,
            },
            Action::Audit {
                category: "safety".to_string(),
                severity: AuditSeverity::Medium,
            },
            Action::Stop {
                message: Some("Blocked for safety".to_string()),
                status_code: 451,
            },
        ]);

        let outcome = executor.execute(&[result]);
        assert!(outcome.should_stop);
        assert_eq!(outcome.stop_status, Some(451));
        assert_eq!(outcome.audit_records.len(), 1);
    }

    #[test]
    fn test_apply_inject_before() {
        let mods = vec![TextModification {
            kind: ModificationKind::Inject,
            content: "WARNING: ".to_string(),
            position: Some(InjectPosition::Before),
            span: None,
        }];

        let result = apply_modifications("Hello", &mods);
        assert_eq!(result, "WARNING: Hello");
    }

    #[test]
    fn test_apply_inject_after() {
        let mods = vec![TextModification {
            kind: ModificationKind::Inject,
            content: " [END]".to_string(),
            position: Some(InjectPosition::After),
            span: None,
        }];

        let result = apply_modifications("Hello", &mods);
        assert_eq!(result, "Hello [END]");
    }

    #[test]
    fn test_apply_redact_with_span() {
        let mods = vec![TextModification {
            kind: ModificationKind::Redact,
            content: "[REDACTED]".to_string(),
            position: None,
            span: Some((6, 11)), // "World"
        }];

        let result = apply_modifications("Hello World!", &mods);
        assert_eq!(result, "Hello [REDACTED]!");
    }

    #[test]
    fn test_outcome_merge() {
        let mut outcome1 = ActionOutcome::new();
        outcome1.audit_records.push(AuditRecord {
            rule_name: "rule1".to_string(),
            policy_name: "policy1".to_string(),
            category: "cat1".to_string(),
            severity: AuditSeverity::Low,
            context: None,
            timestamp: None,
        });

        let mut outcome2 = ActionOutcome::new();
        outcome2.should_stop = true;
        outcome2.stop_message = Some("Stopped".to_string());
        outcome2.stop_status = Some(403);

        outcome1.merge(outcome2);

        assert!(outcome1.should_stop);
        assert_eq!(outcome1.stop_message, Some("Stopped".to_string()));
        assert_eq!(outcome1.audit_records.len(), 1);
    }
}
