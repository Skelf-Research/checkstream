//! Policy evaluation engine

use checkstream_core::{Result, Token};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::{Action, Policy, Trigger};
use crate::trigger::CompositeOperator;

/// Policy evaluation engine
pub struct PolicyEngine {
    policies: Vec<Policy>,
    /// Cached regex patterns for performance
    regex_cache: HashMap<String, Regex>,
    /// Classifier scores from external evaluation (injected before evaluate)
    classifier_scores: HashMap<String, f32>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            regex_cache: HashMap::new(),
            classifier_scores: HashMap::new(),
        }
    }

    /// Load a policy from file
    pub fn load_policy(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let policy = Policy::from_file(path)
            .map_err(|e| checkstream_core::Error::policy(format!("Failed to load policy: {}", e)))?;

        // Pre-compile regex patterns for this policy
        self.compile_patterns(&policy);
        self.policies.push(policy);
        Ok(())
    }

    /// Add a policy directly
    pub fn add_policy(&mut self, policy: Policy) {
        self.compile_patterns(&policy);
        self.policies.push(policy);
    }

    /// Pre-compile regex patterns for a policy
    fn compile_patterns(&mut self, policy: &Policy) {
        for rule in &policy.rules {
            self.compile_trigger_patterns(&rule.trigger);
        }
    }

    /// Recursively compile patterns in triggers
    fn compile_trigger_patterns(&mut self, trigger: &Trigger) {
        match trigger {
            Trigger::Pattern { pattern, case_insensitive } => {
                let cache_key = format!("{}:{}", pattern, case_insensitive);
                if !self.regex_cache.contains_key(&cache_key) {
                    let regex = if *case_insensitive {
                        Regex::new(&format!("(?i){}", regex::escape(pattern)))
                    } else {
                        Regex::new(&regex::escape(pattern))
                    };
                    if let Ok(re) = regex {
                        self.regex_cache.insert(cache_key, re);
                    }
                }
            }
            Trigger::Composite { triggers, .. } => {
                for t in triggers {
                    self.compile_trigger_patterns(t);
                }
            }
            _ => {}
        }
    }

    /// Set classifier scores before evaluation
    /// Call this with results from running classifiers on the text
    pub fn set_classifier_scores(&mut self, scores: HashMap<String, f32>) {
        self.classifier_scores = scores;
    }

    /// Add a single classifier score
    pub fn add_classifier_score(&mut self, classifier: &str, score: f32) {
        self.classifier_scores.insert(classifier.to_string(), score);
    }

    /// Clear all classifier scores
    pub fn clear_classifier_scores(&mut self) {
        self.classifier_scores.clear();
    }

    /// Evaluate text against all policies
    pub fn evaluate_text(&self, text: &str) -> Vec<EvaluationResult> {
        let mut results = Vec::new();

        for policy in &self.policies {
            for rule in &policy.rules {
                if !rule.enabled {
                    continue;
                }

                if let Some((triggered, score, metadata)) = self.evaluate_trigger(&rule.trigger, text) {
                    if triggered {
                        results.push(EvaluationResult {
                            rule_name: rule.name.clone(),
                            policy_name: policy.name.clone(),
                            actions: rule.actions.clone(),
                            score,
                            metadata,
                        });
                    }
                }
            }
        }

        results
    }

    /// Evaluate tokens against all policies
    pub fn evaluate(&self, tokens: &[Token]) -> Vec<EvaluationResult> {
        // Concatenate token texts for pattern matching
        let text: String = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join("");
        self.evaluate_text(&text)
    }

    /// Evaluate a single trigger against text
    /// Returns (triggered, score, metadata)
    fn evaluate_trigger(&self, trigger: &Trigger, text: &str) -> Option<(bool, f32, EvaluationMetadata)> {
        match trigger {
            Trigger::Pattern { pattern, case_insensitive } => {
                let cache_key = format!("{}:{}", pattern, case_insensitive);
                if let Some(regex) = self.regex_cache.get(&cache_key) {
                    if let Some(m) = regex.find(text) {
                        return Some((
                            true,
                            1.0, // Pattern matches are binary
                            EvaluationMetadata {
                                token_indices: vec![],
                                matched_content: Some(m.as_str().to_string()),
                                classifier_scores: vec![],
                            },
                        ));
                    }
                } else {
                    // Fallback to non-cached pattern matching
                    let found = if *case_insensitive {
                        text.to_lowercase().contains(&pattern.to_lowercase())
                    } else {
                        text.contains(pattern)
                    };
                    if found {
                        return Some((
                            true,
                            1.0,
                            EvaluationMetadata {
                                token_indices: vec![],
                                matched_content: Some(pattern.clone()),
                                classifier_scores: vec![],
                            },
                        ));
                    }
                }
                Some((false, 0.0, EvaluationMetadata::default()))
            }

            Trigger::Classifier { classifier, threshold } => {
                if let Some(&score) = self.classifier_scores.get(classifier) {
                    let triggered = score >= *threshold;
                    Some((
                        triggered,
                        score,
                        EvaluationMetadata {
                            token_indices: vec![],
                            matched_content: None,
                            classifier_scores: vec![(classifier.clone(), score)],
                        },
                    ))
                } else {
                    // Classifier not evaluated - return None to indicate unknown
                    Some((false, 0.0, EvaluationMetadata::default()))
                }
            }

            Trigger::Context { field, value } => {
                // Context triggers check for field=value in the text
                // This is a simple implementation - in practice would check structured context
                let pattern = format!("{}:{}", field, value);
                let triggered = text.to_lowercase().contains(&pattern.to_lowercase());
                Some((
                    triggered,
                    if triggered { 1.0 } else { 0.0 },
                    EvaluationMetadata::default(),
                ))
            }

            Trigger::Composite { operator, triggers } => {
                let mut sub_results: Vec<(bool, f32, EvaluationMetadata)> = Vec::new();

                for t in triggers {
                    if let Some(result) = self.evaluate_trigger(t, text) {
                        sub_results.push(result);
                    }
                }

                if sub_results.is_empty() {
                    return Some((false, 0.0, EvaluationMetadata::default()));
                }

                let triggered = match operator {
                    CompositeOperator::And => sub_results.iter().all(|(t, _, _)| *t),
                    CompositeOperator::Or => sub_results.iter().any(|(t, _, _)| *t),
                };

                // Aggregate scores and metadata
                let avg_score = sub_results.iter().map(|(_, s, _)| s).sum::<f32>()
                    / sub_results.len() as f32;

                let mut combined_metadata = EvaluationMetadata::default();
                for (_, _, meta) in sub_results {
                    if let Some(content) = meta.matched_content {
                        combined_metadata.matched_content = Some(content);
                    }
                    combined_metadata.classifier_scores.extend(meta.classifier_scores);
                }

                Some((triggered, avg_score, combined_metadata))
            }
        }
    }

    /// Get loaded policies
    pub fn policies(&self) -> &[Policy] {
        &self.policies
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of policy evaluation
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// The rule that was triggered
    pub rule_name: String,

    /// The policy this rule belongs to
    pub policy_name: String,

    /// Actions to take
    pub actions: Vec<Action>,

    /// Score or confidence (0.0-1.0)
    pub score: f32,

    /// Additional metadata
    pub metadata: EvaluationMetadata,
}

/// Metadata about an evaluation
#[derive(Debug, Clone, Default)]
pub struct EvaluationMetadata {
    /// Token indices that triggered the rule
    pub token_indices: Vec<usize>,

    /// Matched content (if applicable)
    pub matched_content: Option<String>,

    /// Classifier outputs (if applicable)
    pub classifier_scores: Vec<(String, f32)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rule;
    use crate::action::LogLevel;

    fn create_test_policy(rules: Vec<Rule>) -> Policy {
        Policy {
            name: "test-policy".to_string(),
            description: "Test policy".to_string(),
            version: "1.0".to_string(),
            regulation: None,
            rules,
        }
    }

    #[test]
    fn test_pattern_trigger_matches() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "unsafe-content".to_string(),
                description: "Detect unsafe content".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![Action::Log {
                    message: "Unsafe content detected".to_string(),
                    level: LogLevel::Warn,
                }],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This content is unsafe");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rule_name, "unsafe-content");
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn test_pattern_trigger_no_match() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "unsafe-content".to_string(),
                description: "Detect unsafe content".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This content is safe");
        assert!(results.is_empty());
    }

    #[test]
    fn test_pattern_trigger_case_insensitive() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "test".to_string(),
                description: "Test rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "UNSAFE".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe content");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_pattern_trigger_case_sensitive() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "test".to_string(),
                description: "Test rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "UNSAFE".to_string(),
                    case_insensitive: false,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe content");
        assert!(results.is_empty()); // lowercase "unsafe" won't match "UNSAFE"
    }

    #[test]
    fn test_classifier_trigger_above_threshold() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "toxic-content".to_string(),
                description: "Detect toxic content".to_string(),
                trigger: Trigger::Classifier {
                    classifier: "toxicity".to_string(),
                    threshold: 0.7,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        engine.add_classifier_score("toxicity", 0.85);
        let results = engine.evaluate_text("Some text");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, 0.85);
    }

    #[test]
    fn test_classifier_trigger_below_threshold() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "toxic-content".to_string(),
                description: "Detect toxic content".to_string(),
                trigger: Trigger::Classifier {
                    classifier: "toxicity".to_string(),
                    threshold: 0.7,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        engine.add_classifier_score("toxicity", 0.5);
        let results = engine.evaluate_text("Some text");
        assert!(results.is_empty());
    }

    #[test]
    fn test_composite_and_both_true() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "both-patterns".to_string(),
                description: "Match both patterns".to_string(),
                trigger: Trigger::Composite {
                    operator: CompositeOperator::And,
                    triggers: vec![
                        Box::new(Trigger::Pattern {
                            pattern: "unsafe".to_string(),
                            case_insensitive: true,
                        }),
                        Box::new(Trigger::Pattern {
                            pattern: "dangerous".to_string(),
                            case_insensitive: true,
                        }),
                    ],
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe and dangerous content");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_composite_and_one_false() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "both-patterns".to_string(),
                description: "Match both patterns".to_string(),
                trigger: Trigger::Composite {
                    operator: CompositeOperator::And,
                    triggers: vec![
                        Box::new(Trigger::Pattern {
                            pattern: "unsafe".to_string(),
                            case_insensitive: true,
                        }),
                        Box::new(Trigger::Pattern {
                            pattern: "dangerous".to_string(),
                            case_insensitive: true,
                        }),
                    ],
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe but safe content");
        assert!(results.is_empty());
    }

    #[test]
    fn test_composite_or_one_true() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "either-pattern".to_string(),
                description: "Match either pattern".to_string(),
                trigger: Trigger::Composite {
                    operator: CompositeOperator::Or,
                    triggers: vec![
                        Box::new(Trigger::Pattern {
                            pattern: "unsafe".to_string(),
                            case_insensitive: true,
                        }),
                        Box::new(Trigger::Pattern {
                            pattern: "dangerous".to_string(),
                            case_insensitive: true,
                        }),
                    ],
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe content");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_disabled_rule_skipped() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "disabled".to_string(),
                description: "Disabled rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: false,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe content");
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_rules_trigger() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "rule1".to_string(),
                description: "First rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            },
            Rule {
                name: "rule2".to_string(),
                description: "Second rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "dangerous".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe and dangerous content");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_actions_returned_in_result() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "test".to_string(),
                description: "Test rule with actions".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![
                    Action::Log {
                        message: "Unsafe detected".to_string(),
                        level: LogLevel::Warn,
                    },
                    Action::Stop {
                        message: Some("Blocked".to_string()),
                        status_code: 403,
                    },
                ],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("unsafe content");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actions.len(), 2);
    }

    #[test]
    fn test_matched_content_in_metadata() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "test".to_string(),
                description: "Test rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: false,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let results = engine.evaluate_text("This is unsafe content");
        assert_eq!(results.len(), 1);
        assert!(results[0].metadata.matched_content.is_some());
        assert_eq!(results[0].metadata.matched_content.as_ref().unwrap(), "unsafe");
    }

    #[test]
    fn test_evaluate_with_tokens() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(create_test_policy(vec![
            Rule {
                name: "test".to_string(),
                description: "Test rule".to_string(),
                trigger: Trigger::Pattern {
                    pattern: "unsafe".to_string(),
                    case_insensitive: true,
                },
                actions: vec![],
                regulation: None,
                enabled: true,
            }
        ]));

        let tokens = vec![
            Token::new("This "),
            Token::new("is "),
            Token::new("unsafe "),
            Token::new("content"),
        ];
        let results = engine.evaluate(&tokens);
        assert_eq!(results.len(), 1);
    }
}
