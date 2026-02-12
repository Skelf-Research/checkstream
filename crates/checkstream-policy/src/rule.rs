//! Policy and rule definitions

use serde::{Deserialize, Serialize};

use crate::{Action, Trigger};

/// A complete policy containing multiple rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy name
    pub name: String,

    /// Description of what this policy enforces
    pub description: String,

    /// Version of the policy
    #[serde(default)]
    pub version: String,

    /// Regulatory framework this policy supports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regulation: Option<String>,

    /// Rules in this policy
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Load a policy from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load a policy from a file
    pub fn from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&content)?)
    }
}

/// A single rule within a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule identifier
    pub name: String,

    /// Description of what this rule does
    pub description: String,

    /// Trigger conditions
    pub trigger: Trigger,

    /// Actions to take when triggered
    pub actions: Vec<Action>,

    /// Specific regulation this rule maps to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regulation: Option<String>,

    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_deserialization() {
        let yaml = r#"
name: test-policy
description: Test policy
version: "1.0"
rules:
  - name: test-rule
    description: Test rule
    trigger:
      type: pattern
      pattern: "test"
    actions:
      - type: log
        message: "Pattern matched"
    enabled: true
"#;

        let policy = Policy::from_yaml(yaml).unwrap();
        assert_eq!(policy.name, "test-policy");
        assert_eq!(policy.rules.len(), 1);
    }
}
