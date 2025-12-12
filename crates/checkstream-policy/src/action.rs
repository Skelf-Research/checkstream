//! Policy action definitions

use serde::{Deserialize, Serialize};

/// Action to take when a policy rule is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Log the event
    Log {
        /// Message to log
        message: String,

        /// Log level
        #[serde(default = "default_level")]
        level: LogLevel,
    },

    /// Stop the stream immediately
    Stop {
        /// Message to return to client
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,

        /// HTTP status code
        #[serde(default = "default_status")]
        status_code: u16,
    },

    /// Redact matched content
    Redact {
        /// Replacement text
        #[serde(default = "default_redaction")]
        replacement: String,
    },

    /// Inject additional content
    Inject {
        /// Content to inject
        content: String,

        /// Where to inject (before/after/replace)
        #[serde(default)]
        position: InjectPosition,
    },

    /// Adapt generation parameters
    Adapt {
        /// Parameter to adapt
        parameter: AdaptParameter,

        /// New value
        value: f32,
    },

    /// Mark for audit
    Audit {
        /// Audit category
        category: String,

        /// Severity level
        severity: AuditSeverity,
    },
}

/// Type alias for backward compatibility
pub type ActionType = Action;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InjectPosition {
    Before,
    #[default]
    After,
    Replace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptParameter {
    Temperature,
    TopP,
    TopK,
    RepetitionPenalty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

fn default_level() -> LogLevel {
    LogLevel::Info
}

fn default_status() -> u16 {
    403
}

fn default_redaction() -> String {
    "[REDACTED]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_action() {
        let json = r#"{"type": "log", "message": "Test"}"#;
        let action: Action = serde_json::from_str(json).unwrap();

        match action {
            Action::Log { message, level } => {
                assert_eq!(message, "Test");
                assert!(matches!(level, LogLevel::Info));
            },
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_stop_action() {
        let json = r#"{"type": "stop", "message": "Blocked", "status_code": 451}"#;
        let action: Action = serde_json::from_str(json).unwrap();

        match action {
            Action::Stop { message, status_code } => {
                assert_eq!(message, Some("Blocked".to_string()));
                assert_eq!(status_code, 451);
            },
            _ => panic!("Wrong action type"),
        }
    }
}
