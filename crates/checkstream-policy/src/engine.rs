//! Policy evaluation engine

use checkstream_core::{Result, Token};
use std::path::Path;

use crate::{Action, Policy};

/// Policy evaluation engine
pub struct PolicyEngine {
    policies: Vec<Policy>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Load a policy from file
    pub fn load_policy(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let policy = Policy::from_file(path)
            .map_err(|e| checkstream_core::Error::policy(format!("Failed to load policy: {}", e)))?;

        self.policies.push(policy);
        Ok(())
    }

    /// Add a policy directly
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Evaluate tokens against all policies
    pub fn evaluate(&self, _tokens: &[Token]) -> Vec<EvaluationResult> {
        let results = Vec::new();

        for policy in &self.policies {
            for rule in &policy.rules {
                if !rule.enabled {
                    continue;
                }

                // TODO: Implement actual trigger evaluation
                // This is a placeholder
            }
        }

        results
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
