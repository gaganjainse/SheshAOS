use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    Confirm,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Violation {
    pub violation_type: String,
    pub details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyEngine {
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub enabled: bool,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self { rules: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_decision_variants() {
        let decision = PolicyDecision::Allow;
        assert_eq!(decision, PolicyDecision::Allow);
    }
}
