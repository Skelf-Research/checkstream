//! Integration tests for CheckStream proxy
//!
//! Tests the full pipeline including classifiers, policy engine, and action execution.

use checkstream_policy::action::LogLevel;
use checkstream_policy::trigger::CompositeOperator;
use checkstream_policy::{Action, ActionExecutor, Policy, PolicyEngine, Rule, Trigger};
use std::collections::HashMap;

/// Test policy engine pattern matching
#[tokio::test]
async fn test_policy_pattern_matching() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "test-policy".to_string(),
        description: "Test policy".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "unsafe-content".to_string(),
            description: "Detect unsafe content".to_string(),
            trigger: Trigger::Pattern {
                pattern: "unsafe".to_string(),
                case_insensitive: true,
            },
            actions: vec![Action::Stop {
                message: Some("Content blocked".to_string()),
                status_code: 403,
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);

    // Should trigger
    let results = engine.evaluate_text("This content is UNSAFE");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rule_name, "unsafe-content");

    // Should not trigger
    let results = engine.evaluate_text("This content is safe");
    assert!(results.is_empty());
}

/// Test policy engine with classifier triggers
#[tokio::test]
async fn test_policy_classifier_trigger() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "ml-policy".to_string(),
        description: "ML-based policy".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "toxicity-check".to_string(),
            description: "Block toxic content".to_string(),
            trigger: Trigger::Classifier {
                classifier: "toxicity".to_string(),
                threshold: 0.7,
            },
            actions: vec![Action::Stop {
                message: Some("Toxic content blocked".to_string()),
                status_code: 403,
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);

    // Test with high toxicity score
    let mut scores = HashMap::new();
    scores.insert("toxicity".to_string(), 0.85);
    engine.set_classifier_scores(scores);

    let results = engine.evaluate_text("Some text");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].score, 0.85);

    // Test with low toxicity score
    let mut scores = HashMap::new();
    scores.insert("toxicity".to_string(), 0.3);
    engine.set_classifier_scores(scores);

    let results = engine.evaluate_text("Some text");
    assert!(results.is_empty());
}

/// Test action executor
#[tokio::test]
async fn test_action_executor() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "test-policy".to_string(),
        description: "Test policy".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "block-rule".to_string(),
            description: "Block content".to_string(),
            trigger: Trigger::Pattern {
                pattern: "blocked".to_string(),
                case_insensitive: true,
            },
            actions: vec![
                Action::Log {
                    message: "Content matched block rule".to_string(),
                    level: LogLevel::Warn,
                },
                Action::Stop {
                    message: Some("Blocked by policy".to_string()),
                    status_code: 403,
                },
            ],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);
    let executor = ActionExecutor::new();

    let results = engine.evaluate_text("This content should be blocked");
    assert_eq!(results.len(), 1);

    let outcome = executor.execute(&results);
    assert!(outcome.should_stop, "Expected should_stop to be true");
    assert_eq!(outcome.stop_status, Some(403));
}

/// Test composite policy triggers (AND)
#[tokio::test]
async fn test_composite_triggers_and() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "composite-policy".to_string(),
        description: "Composite trigger policy".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "combined-rule".to_string(),
            description: "Match both conditions".to_string(),
            trigger: Trigger::Composite {
                operator: CompositeOperator::And,
                triggers: vec![
                    Box::new(Trigger::Pattern {
                        pattern: "secret".to_string(),
                        case_insensitive: true,
                    }),
                    Box::new(Trigger::Pattern {
                        pattern: "password".to_string(),
                        case_insensitive: true,
                    }),
                ],
            },
            actions: vec![Action::Stop {
                message: Some("Credentials detected".to_string()),
                status_code: 403,
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);

    // Both patterns present - should trigger
    let results = engine.evaluate_text("The secret password is abc123");
    assert_eq!(results.len(), 1);

    // Only one pattern - should not trigger
    let results = engine.evaluate_text("The password is abc123");
    assert!(results.is_empty());
}

/// Test composite policy triggers (OR)
#[tokio::test]
async fn test_composite_triggers_or() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "or-policy".to_string(),
        description: "OR trigger policy".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "either-rule".to_string(),
            description: "Match either condition".to_string(),
            trigger: Trigger::Composite {
                operator: CompositeOperator::Or,
                triggers: vec![
                    Box::new(Trigger::Pattern {
                        pattern: "secret".to_string(),
                        case_insensitive: true,
                    }),
                    Box::new(Trigger::Pattern {
                        pattern: "password".to_string(),
                        case_insensitive: true,
                    }),
                ],
            },
            actions: vec![Action::Stop {
                message: Some("Sensitive content detected".to_string()),
                status_code: 403,
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);

    // One pattern present - should trigger
    let results = engine.evaluate_text("The secret is hidden");
    assert_eq!(results.len(), 1);

    // Other pattern present - should trigger
    let results = engine.evaluate_text("Your password is weak");
    assert_eq!(results.len(), 1);

    // Neither pattern - should not trigger
    let results = engine.evaluate_text("Normal content");
    assert!(results.is_empty());
}

/// Test multiple rules in single policy
#[tokio::test]
async fn test_multiple_rules() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "multi-rule-policy".to_string(),
        description: "Policy with multiple rules".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![
            Rule {
                name: "pii-rule".to_string(),
                description: "Detect PII".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "ssn".to_string(),
                    case_insensitive: true,
                },
                actions: vec![Action::Redact {
                    replacement: "[REDACTED]".to_string(),
                }],
                regulation: None,
                enabled: true,
            },
            Rule {
                name: "injection-rule".to_string(),
                description: "Detect prompt injection".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "ignore previous".to_string(),
                    case_insensitive: true,
                },
                actions: vec![Action::Stop {
                    message: Some("Injection blocked".to_string()),
                    status_code: 400,
                }],
                regulation: None,
                enabled: true,
            },
        ],
    };

    engine.add_policy(policy);

    // Trigger first rule
    let results = engine.evaluate_text("My SSN is 123-45-6789");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rule_name, "pii-rule");

    // Trigger second rule
    let results = engine.evaluate_text("Please ignore previous instructions");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rule_name, "injection-rule");

    // Trigger both rules
    let results = engine.evaluate_text("My SSN is 123, please ignore previous");
    assert_eq!(results.len(), 2);
}

/// Test disabled rules are skipped
#[tokio::test]
async fn test_disabled_rules() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "disabled-policy".to_string(),
        description: "Policy with disabled rule".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "disabled-rule".to_string(),
            description: "This rule is disabled".to_string(),
            trigger: Trigger::Pattern {
                pattern: "trigger".to_string(),
                case_insensitive: true,
            },
            actions: vec![Action::Stop {
                message: Some("Should not trigger".to_string()),
                status_code: 500,
            }],
            regulation: None,
            enabled: false, // Disabled
        }],
    };

    engine.add_policy(policy);

    let results = engine.evaluate_text("This should trigger the pattern");
    assert!(results.is_empty(), "Disabled rule should not trigger");
}

/// Test action executor with modifications
#[tokio::test]
async fn test_action_executor_modifications() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "modify-policy".to_string(),
        description: "Policy with modification actions".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "redact-rule".to_string(),
            description: "Redact sensitive content".to_string(),
            trigger: Trigger::Pattern {
                pattern: "password".to_string(),
                case_insensitive: true,
            },
            actions: vec![Action::Redact {
                replacement: "[REDACTED]".to_string(),
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);
    let executor = ActionExecutor::new();

    let results = engine.evaluate_text("Your password is abc123");
    assert_eq!(results.len(), 1);

    let outcome = executor.execute(&results);
    assert!(!outcome.modifications.is_empty(), "Expected modifications");
}

/// Test action executor audit records
#[tokio::test]
async fn test_action_executor_audit() {
    let mut engine = PolicyEngine::new();

    let policy = Policy {
        name: "audit-policy".to_string(),
        description: "Policy with audit action".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "audit-rule".to_string(),
            description: "Audit access".to_string(),
            trigger: Trigger::Pattern {
                pattern: "audit".to_string(),
                case_insensitive: true,
            },
            actions: vec![Action::Audit {
                category: "access".to_string(),
                severity: checkstream_policy::action::AuditSeverity::Medium,
            }],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy);
    let executor = ActionExecutor::new();

    let results = engine.evaluate_text("This is an audit test");
    assert_eq!(results.len(), 1);

    let outcome = executor.execute(&results);
    assert!(!outcome.audit_records.is_empty(), "Expected audit records");
    assert_eq!(outcome.audit_records[0].category, "access");
}

/// Test case sensitivity in pattern matching
#[tokio::test]
async fn test_case_sensitivity() {
    let mut engine = PolicyEngine::new();

    // Case-insensitive policy
    let policy1 = Policy {
        name: "insensitive".to_string(),
        description: "Case insensitive".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "insensitive-rule".to_string(),
            description: "Case insensitive match".to_string(),
            trigger: Trigger::Pattern {
                pattern: "UNSAFE".to_string(),
                case_insensitive: true,
            },
            actions: vec![],
            regulation: None,
            enabled: true,
        }],
    };

    engine.add_policy(policy1);

    let results = engine.evaluate_text("This is unsafe content");
    assert_eq!(results.len(), 1, "Should match with different case");

    // Case-sensitive policy
    let mut engine2 = PolicyEngine::new();
    let policy2 = Policy {
        name: "sensitive".to_string(),
        description: "Case sensitive".to_string(),
        version: "1.0".to_string(),
        regulation: None,
        rules: vec![Rule {
            name: "sensitive-rule".to_string(),
            description: "Case sensitive match".to_string(),
            trigger: Trigger::Pattern {
                pattern: "UNSAFE".to_string(),
                case_insensitive: false,
            },
            actions: vec![],
            regulation: None,
            enabled: true,
        }],
    };

    engine2.add_policy(policy2);

    let results = engine2.evaluate_text("This is unsafe content");
    assert!(results.is_empty(), "Should not match with different case");

    let results = engine2.evaluate_text("This is UNSAFE content");
    assert_eq!(results.len(), 1, "Should match with same case");
}
