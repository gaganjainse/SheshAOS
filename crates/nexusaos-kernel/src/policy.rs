//! Policy engine types and enforcement logic.
//!
//! The policy engine implements a deny-by-default model. Every action must be
//! explicitly allowed by a policy rule or it is denied. Rules are evaluated
//! in order; first match wins.

use serde::{Deserialize, Serialize};

/// The result of a policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// Action is allowed to proceed.
    Allow,

    /// Action is denied with a reason.
    Deny(String),

    /// Action requires explicit user confirmation before proceeding.
    RequireConfirmation(String),
}

impl PolicyDecision {
    /// Returns true if the action is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }

    /// Returns true if the action is denied.
    pub fn is_denied(&self) -> bool {
        matches!(self, PolicyDecision::Deny(_))
    }

    /// Returns true if the action requires confirmation.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, PolicyDecision::RequireConfirmation(_))
    }
}

/// A single policy rule that matches actions and produces decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Human-readable name for this rule.
    pub name: String,

    /// Pattern to match against action names (e.g., "filesystem.read_*").
    pub action_pattern: String,

    /// Decision to apply when this rule matches.
    pub decision: String,

    /// Minimum trust tier required for this rule to apply.
    #[serde(default)]
    pub trust_tier: u8,

    /// Optional description of why this rule exists.
    #[serde(default)]
    pub description: Option<String>,
}

/// Trust tiers define levels of autonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum TrustTier {
    /// All confirmations required.
    Untrusted = 0,
    /// Read operations allowed, writes confirmed.
    #[default]
    Basic = 1,
    /// Most operations allowed, destructive confirmed.
    Trusted = 2,
    /// All operations allowed — use with extreme caution.
    Autonomous = 3,
}

impl TrustTier {
    /// Create from a numeric tier value.
    ///
    /// Returns `Err(ConfigError)` for unknown levels instead of silently defaulting.
    pub fn from_level(level: u8) -> Result<Self, crate::error::ConfigError> {
        match level {
            0 => Ok(TrustTier::Untrusted),
            1 => Ok(TrustTier::Basic),
            2 => Ok(TrustTier::Trusted),
            3 => Ok(TrustTier::Autonomous),
            _ => Err(crate::error::ConfigError::Invalid {
                message: format!("Unknown trust tier level: {}", level),
            }),
        }
    }
}

/// Well-known action patterns used for policy evaluation.
///
/// Action patterns follow the format `<domain>.<action>`, e.g. `task.create`,
/// `filesystem.read_file`. The policy engine matches these against rule
/// `action_pattern` fields using exact match or glob suffix matching (`*`).
pub mod actions {
    /// Policy check for creating a new task.
    pub const TASK_CREATE: &str = "task.create";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// The action that was attempted.
    pub action: String,

    /// The rule that caused the denial.
    pub rule_name: String,

    /// Why it was denied.
    pub reason: String,

    /// When the violation occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// The policy engine evaluates actions against a set of rules.
#[derive(Debug, Clone)]
pub struct PolicyEngine {
    /// Ordered list of policy rules (first match wins).
    rules: Vec<PolicyRule>,

    /// Current trust tier.
    trust_tier: TrustTier,
}

impl PolicyEngine {
    /// Create a new policy engine with the given rules and trust tier.
    pub fn new(rules: Vec<PolicyRule>, trust_tier: TrustTier) -> Self {
        Self { rules, trust_tier }
    }

    /// Create a policy engine with deny-all defaults.
    pub fn deny_all() -> Self {
        Self { rules: Vec::new(), trust_tier: TrustTier::Untrusted }
    }

    /// Evaluate an action against the policy rules.
    ///
    /// Returns the decision from the first matching rule, or `Deny` if no rule matches
    /// (deny-by-default).
    pub fn evaluate(&self, action: &str) -> PolicyDecision {
        for rule in &self.rules {
            if self.matches_pattern(&rule.action_pattern, action) {
                let rule_tier = match TrustTier::from_level(rule.trust_tier) {
                    Ok(tier) => tier,
                    Err(_) => continue,
                };
                if self.trust_tier >= rule_tier {
                    return self.parse_decision(&rule.decision, &rule.name);
                }
            }
        }

        // Deny by default
        PolicyDecision::Deny(format!(
            "No matching policy rule for action '{}' at trust tier {:?}",
            action, self.trust_tier
        ))
    }

    /// Check if an action pattern matches a given action.
    ///
    /// Supports simple glob patterns:
    /// - `*` matches any suffix
    /// - exact match otherwise
    fn matches_pattern(&self, pattern: &str, action: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            action.starts_with(prefix)
        } else {
            pattern == action
        }
    }

    /// Parse a decision string into a PolicyDecision.
    fn parse_decision(&self, decision: &str, rule_name: &str) -> PolicyDecision {
        match decision {
            "allow" => PolicyDecision::Allow,
            "deny" => PolicyDecision::Deny(format!("Denied by rule: {}", rule_name)),
            "require_confirmation" => PolicyDecision::RequireConfirmation(format!(
                "Confirmation required by: {}",
                rule_name
            )),
            other => PolicyDecision::Deny(format!(
                "Unknown decision '{}' in rule '{}', denying",
                other, rule_name
            )),
        }
    }

    /// Get the current trust tier.
    pub fn trust_tier(&self) -> TrustTier {
        self.trust_tier
    }

    /// Add a new rule to the policy engine.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Set the trust tier.
    pub fn set_trust_tier(&mut self, tier: TrustTier) {
        self.trust_tier = tier;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_rules() -> Vec<PolicyRule> {
        vec![
            PolicyRule {
                name: "allow-read".to_string(),
                action_pattern: "filesystem.read_*".to_string(),
                decision: "allow".to_string(),
                trust_tier: 0,
                description: None,
            },
            PolicyRule {
                name: "confirm-write".to_string(),
                action_pattern: "filesystem.write_*".to_string(),
                decision: "require_confirmation".to_string(),
                trust_tier: 1,
                description: None,
            },
            PolicyRule {
                name: "deny-delete-untrusted".to_string(),
                action_pattern: "filesystem.delete_*".to_string(),
                decision: "deny".to_string(),
                trust_tier: 0,
                description: Some("Deny deletes at tier 0".to_string()),
            },
            PolicyRule {
                name: "confirm-delete-trusted".to_string(),
                action_pattern: "filesystem.delete_*".to_string(),
                decision: "require_confirmation".to_string(),
                trust_tier: 1,
                description: None,
            },
        ]
    }

    #[test]
    fn test_allow_read() {
        let engine = PolicyEngine::new(test_rules(), TrustTier::Basic);
        let decision = engine.evaluate("filesystem.read_file");
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_confirm_write() {
        let engine = PolicyEngine::new(test_rules(), TrustTier::Basic);
        let decision = engine.evaluate("filesystem.write_file");
        assert!(decision.requires_confirmation());
    }

    #[test]
    fn test_deny_by_default() {
        let engine = PolicyEngine::new(test_rules(), TrustTier::Basic);
        let decision = engine.evaluate("unknown.action");
        assert!(decision.is_denied());
    }

    #[test]
    fn test_deny_all_engine() {
        let engine = PolicyEngine::deny_all();
        let decision = engine.evaluate("filesystem.read_file");
        assert!(decision.is_denied());
    }

    #[test]
    fn test_trust_tier_gating() {
        // At tier 0, write rules (tier 1) should not match
        let engine = PolicyEngine::new(test_rules(), TrustTier::Untrusted);
        let decision = engine.evaluate("filesystem.write_file");
        assert!(decision.is_denied());
    }

    #[test]
    fn test_delete_at_untrusted() {
        let engine = PolicyEngine::new(test_rules(), TrustTier::Untrusted);
        let decision = engine.evaluate("filesystem.delete_file");
        assert!(decision.is_denied());
    }

    #[test]
    fn test_delete_at_basic() {
        let engine = PolicyEngine::new(test_rules(), TrustTier::Basic);
        let decision = engine.evaluate("filesystem.delete_file");
        // First matching rule at tier <= Basic is the deny at tier 0
        assert!(decision.is_denied());
    }

    #[test]
    fn test_policy_decision_helpers() {
        assert!(PolicyDecision::Allow.is_allowed());
        assert!(!PolicyDecision::Allow.is_denied());
        assert!(!PolicyDecision::Allow.requires_confirmation());

        assert!(!PolicyDecision::Deny("test".into()).is_allowed());
        assert!(PolicyDecision::Deny("test".into()).is_denied());

        assert!(PolicyDecision::RequireConfirmation("test".into()).requires_confirmation());
    }

    #[test]
    fn test_trust_tier_ordering() {
        assert!(TrustTier::Autonomous > TrustTier::Trusted);
        assert!(TrustTier::Trusted > TrustTier::Basic);
        assert!(TrustTier::Basic > TrustTier::Untrusted);
    }

    #[test]
    fn test_serde_roundtrip() {
        let decision = PolicyDecision::RequireConfirmation("test".to_string());
        let json = serde_json::to_string(&decision).expect("serialize");
        let back: PolicyDecision = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decision, back);
    }

    #[test]
    fn test_trust_tier_getter() {
        let engine = PolicyEngine::new(vec![], TrustTier::Trusted);
        assert_eq!(engine.trust_tier(), TrustTier::Trusted);
    }

    #[test]
    fn test_set_trust_tier() {
        let mut engine = PolicyEngine::new(vec![], TrustTier::Untrusted);
        engine.set_trust_tier(TrustTier::Autonomous);
        assert_eq!(engine.trust_tier(), TrustTier::Autonomous);
    }

    #[test]
    fn test_add_rule_then_evaluate() {
        let mut engine = PolicyEngine::deny_all();
        engine.add_rule(PolicyRule {
            name: "allow-read".into(),
            action_pattern: "fs.read".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: None,
        });
        let decision = engine.evaluate("fs.read");
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_matches_pattern_exact() {
        let engine = PolicyEngine::new(vec![], TrustTier::Autonomous);
        assert!(engine.matches_pattern("fs.read", "fs.read"));
        assert!(!engine.matches_pattern("fs.read", "fs.write"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        let engine = PolicyEngine::new(vec![], TrustTier::Autonomous);
        assert!(engine.matches_pattern("*", "anything"));
        assert!(engine.matches_pattern("fs.*", "fs.read"));
        assert!(engine.matches_pattern("fs.*", "fs.write"));
        assert!(!engine.matches_pattern("fs.*", "db.query"));
    }

    #[test]
    fn test_matches_pattern_prefix_only() {
        let engine = PolicyEngine::new(vec![], TrustTier::Autonomous);
        assert!(engine.matches_pattern("fs.read_*", "fs.read_file"));
        assert!(engine.matches_pattern("fs.read_*", "fs.read_dir"));
        assert!(!engine.matches_pattern("fs.read_*", "fs.write_file"));
    }

    #[test]
    fn test_parse_decision_unknown() {
        let engine = PolicyEngine::new(vec![], TrustTier::Autonomous);
        let decision = engine.parse_decision("maybe", "bad-rule");
        assert!(decision.is_denied());
        let msg = match decision {
            PolicyDecision::Deny(msg) => msg,
            _ => panic!("expected deny"),
        };
        assert!(msg.contains("maybe"));
        assert!(msg.contains("bad-rule"));
    }

    #[test]
    fn test_evaluate_first_match_wins() {
        let rules = vec![
            PolicyRule {
                name: "deny-all".into(),
                action_pattern: "*".into(),
                decision: "deny".into(),
                trust_tier: 0,
                description: None,
            },
            PolicyRule {
                name: "allow-read".into(),
                action_pattern: "fs.read".into(),
                decision: "allow".into(),
                trust_tier: 0,
                description: None,
            },
        ];
        let engine = PolicyEngine::new(rules, TrustTier::Autonomous);
        // First match wins: deny-all matches everything first
        let decision = engine.evaluate("fs.read");
        assert!(decision.is_denied());
    }

    #[test]
    fn test_evaluate_allow_with_trust_tier_gating() {
        let rules = vec![
            PolicyRule {
                name: "allow-admin".into(),
                action_pattern: "admin.*".into(),
                decision: "allow".into(),
                trust_tier: 3, // requires Autonomous
                description: None,
            },
        ];
        let engine = PolicyEngine::new(rules, TrustTier::Basic);
        let decision = engine.evaluate("admin.create");
        assert!(decision.is_denied()); // trust tier too low
    }

    #[test]
    fn test_policy_rule_with_description() {
        let rule = PolicyRule {
            name: "test".into(),
            action_pattern: "test.*".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: Some("A test rule".to_string()),
        };
        assert_eq!(rule.description, Some("A test rule".to_string()));
    }

    #[test]
    fn test_policy_rule_serde_roundtrip() {
        let rule = PolicyRule {
            name: "test".into(),
            action_pattern: "test.*".into(),
            decision: "allow".into(),
            trust_tier: 2,
            description: Some("desc".to_string()),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: PolicyRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule.name, back.name);
        assert_eq!(rule.trust_tier, back.trust_tier);
    }

    #[test]
    fn test_policy_violation_construction() {
        let violation = PolicyViolation {
            action: "fs.write".to_string(),
            rule_name: "deny-writes".to_string(),
            reason: "no write access".to_string(),
            timestamp: Utc::now(),
        };
        assert_eq!(violation.action, "fs.write");
        assert_eq!(violation.rule_name, "deny-writes");
        assert!(violation.reason.contains("no write access"));
    }

    #[test]
    fn test_policy_decision_serde_all_variants() {
        let decisions = vec![
            PolicyDecision::Allow,
            PolicyDecision::Deny("reason".into()),
            PolicyDecision::RequireConfirmation("confirm".into()),
        ];
        for d in decisions {
            let json = serde_json::to_string(&d).unwrap();
            let back: PolicyDecision = serde_json::from_str(&json).unwrap();
            assert_eq!(d, back);
        }
    }

    #[test]
    fn test_trust_tier_from_level_all_values() {
        assert!(matches!(TrustTier::from_level(0), Ok(TrustTier::Untrusted)));
        assert!(matches!(TrustTier::from_level(1), Ok(TrustTier::Basic)));
        assert!(matches!(TrustTier::from_level(2), Ok(TrustTier::Trusted)));
        assert!(matches!(TrustTier::from_level(3), Ok(TrustTier::Autonomous)));
        assert!(TrustTier::from_level(99).is_err());
        assert!(TrustTier::from_level(255).is_err());
    }

    #[test]
    fn test_trust_tier_default() {
        assert_eq!(TrustTier::default(), TrustTier::Basic);
    }

    #[test]
    fn test_require_confirmation_decision_message() {
        let decision = PolicyDecision::RequireConfirmation("need-approval".into());
        let msg = match decision {
            PolicyDecision::RequireConfirmation(msg) => msg,
            _ => panic!("expected require confirmation"),
        };
        assert!(msg.contains("need-approval"));
    }

    #[test]
    fn test_evaluate_at_different_trust_tiers() {
        let rules = vec![
            PolicyRule {
                name: "read".into(),
                action_pattern: "fs.read".into(),
                decision: "allow".into(),
                trust_tier: 1,
                description: None,
            },
            PolicyRule {
                name: "write".into(),
                action_pattern: "fs.write".into(),
                decision: "allow".into(),
                trust_tier: 2,
                description: None,
            },
        ];
        // At Basic: read allowed, write denied (tier 2 > Basic)
        let engine_basic = PolicyEngine::new(rules.clone(), TrustTier::Basic);
        assert!(engine_basic.evaluate("fs.read").is_allowed());
        assert!(engine_basic.evaluate("fs.write").is_denied());

        // At Trusted: both allowed
        let engine_trusted = PolicyEngine::new(rules, TrustTier::Trusted);
        assert!(engine_trusted.evaluate("fs.read").is_allowed());
        assert!(engine_trusted.evaluate("fs.write").is_allowed());
    }
}
