//! Policy trigger definitions

use serde::{Deserialize, Serialize};

/// Trigger condition for a policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// Pattern-based trigger (regex or string match)
    Pattern {
        /// The pattern to match
        pattern: String,

        /// Case-insensitive matching
        #[serde(default)]
        case_insensitive: bool,
    },

    /// Classifier-based trigger
    Classifier {
        /// Name of the classifier
        classifier: String,

        /// Threshold for triggering (0.0-1.0)
        threshold: f32,
    },

    /// Context-based trigger
    Context {
        /// Context field to check
        field: String,

        /// Expected value
        value: String,
    },

    /// Composite trigger (AND/OR logic)
    Composite {
        /// Logic operator
        operator: CompositeOperator,

        /// Sub-triggers
        triggers: Vec<Box<Trigger>>,
    },
}

/// Operator for composite triggers
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompositeOperator {
    And,
    Or,
}

/// Type alias for backward compatibility
pub type TriggerType = Trigger;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_trigger() {
        let json = r#"{"type": "pattern", "pattern": "test"}"#;
        let trigger: Trigger = serde_json::from_str(json).unwrap();

        match trigger {
            Trigger::Pattern { pattern, .. } => assert_eq!(pattern, "test"),
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_classifier_trigger() {
        let json = r#"{"type": "classifier", "classifier": "toxicity", "threshold": 0.8}"#;
        let trigger: Trigger = serde_json::from_str(json).unwrap();

        match trigger {
            Trigger::Classifier {
                classifier,
                threshold,
            } => {
                assert_eq!(classifier, "toxicity");
                assert_eq!(threshold, 0.8);
            }
            _ => panic!("Wrong trigger type"),
        }
    }
}
