//! Task router for SheshAOS.
//!
//! Classifies task intent and routes to the appropriate specialist model role.
//! Uses keyword-based heuristics — no ML required for the router itself.

use serde::{Deserialize, Serialize};

use crate::state::ModelRole;

/// The result of task classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// The primary role to handle this task.
    pub primary_role: ModelRole,

    /// Optional secondary role for review or follow-up.
    pub review_role: Option<ModelRole>,

    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,

    /// Reason for the routing decision.
    pub reason: String,
}

/// Keyword sets for classification.
struct ClassificationKeywords;

impl ClassificationKeywords {
    /// Keywords that indicate planning/architecture tasks.
    const PLANNER: &'static [&'static str] = &[
        "plan",
        "design",
        "architect",
        "architecture",
        "trade-off",
        "tradeoff",
        "decompose",
        "breakdown",
        "strategy",
        "approach",
        "evaluate",
        "compare",
        "review",
        "assess",
        "analyze",
        "scope",
        "roadmap",
        "requirements",
        "specification",
        "rfc",
        "proposal",
        "decision",
    ];

    /// Keywords that indicate coding tasks.
    const CODER: &'static [&'static str] = &[
        "implement",
        "code",
        "write",
        "create",
        "build",
        "fix",
        "bug",
        "debug",
        "refactor",
        "test",
        "function",
        "class",
        "struct",
        "module",
        "api",
        "endpoint",
        "database",
        "query",
        "migration",
        "compile",
        "syntax",
        "error",
        "lint",
        "format",
        "optimize",
    ];

    /// Keywords that indicate vision tasks.
    const VISION: &'static [&'static str] = &[
        "screenshot",
        "image",
        "picture",
        "photo",
        "diagram",
        "pdf",
        "document",
        "ui",
        "interface",
        "layout",
        "visual",
        "ocr",
        "read",
        "display",
        "screen",
        "mockup",
        "wireframe",
    ];
}

/// The task router classifies intent and selects specialist roles.
pub struct TaskRouter;

impl TaskRouter {
    /// Classify a task input and return a routing decision.
    pub fn route(input_text: &str, has_images: bool) -> RouteDecision {
        // Vision takes priority if images are present
        if has_images {
            return RouteDecision {
                primary_role: ModelRole::Vision,
                review_role: Some(ModelRole::Planner),
                confidence: 0.9,
                reason: "Task includes image attachments".to_string(),
            };
        }

        let lower = input_text.to_lowercase();

        // Score each category
        let planner_score = Self::keyword_score(&lower, ClassificationKeywords::PLANNER);
        let coder_score = Self::keyword_score(&lower, ClassificationKeywords::CODER);
        let vision_score = Self::keyword_score(&lower, ClassificationKeywords::VISION);

        let max_score = planner_score.max(coder_score).max(vision_score);

        // If no keywords match, default to planner (ambiguous → planner first)
        if max_score == 0 {
            return RouteDecision {
                primary_role: ModelRole::Planner,
                review_role: None,
                confidence: 0.3,
                reason: "No strong keyword match — routing to planner for clarification"
                    .to_string(),
            };
        }

        // Determine winning role
        let (primary_role, confidence, reason) =
            if planner_score >= coder_score && planner_score >= vision_score {
                (
                    ModelRole::Planner,
                    Self::normalize_confidence(planner_score, max_score),
                    format!(
                        "Planning keywords matched ({} hits vs coder:{}, vision:{})",
                        planner_score, coder_score, vision_score
                    ),
                )
            } else if coder_score >= vision_score {
                (
                    ModelRole::Coder,
                    Self::normalize_confidence(coder_score, max_score),
                    format!(
                        "Coding keywords matched ({} hits vs planner:{}, vision:{})",
                        coder_score, planner_score, vision_score
                    ),
                )
            } else {
                (
                    ModelRole::Vision,
                    Self::normalize_confidence(vision_score, max_score),
                    format!(
                        "Vision keywords matched ({} hits vs planner:{}, coder:{})",
                        vision_score, planner_score, coder_score
                    ),
                )
            };

        // Add reviewer for coding tasks
        let review_role =
            if primary_role == ModelRole::Coder { Some(ModelRole::Reviewer) } else { None };

        RouteDecision { primary_role, review_role, confidence, reason }
    }

    /// Count keyword matches in the input.
    fn keyword_score(input: &str, keywords: &[&str]) -> usize {
        keywords.iter().filter(|kw| input.contains(**kw)).count()
    }

    /// Normalize a score to a confidence value.
    fn normalize_confidence(score: usize, _max: usize) -> f32 {
        match score {
            0 => 0.3,
            1 => 0.5,
            2 => 0.7,
            3 => 0.8,
            _ => 0.9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_planning_task() {
        let decision = TaskRouter::route("Design the architecture for a new API", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
        assert!(decision.confidence >= 0.5);
    }

    #[test]
    fn test_route_coding_task() {
        let decision =
            TaskRouter::route("Implement a function to parse JSON and write tests", false);
        assert_eq!(decision.primary_role, ModelRole::Coder);
        assert!(decision.review_role.is_some());
    }

    #[test]
    fn test_route_vision_task() {
        let decision = TaskRouter::route("Analyze this screenshot of the UI layout", false);
        assert_eq!(decision.primary_role, ModelRole::Vision);
    }

    #[test]
    fn test_route_with_images() {
        let decision = TaskRouter::route("What does this show?", true);
        assert_eq!(decision.primary_role, ModelRole::Vision);
        assert!(decision.confidence >= 0.8);
    }

    #[test]
    fn test_route_ambiguous() {
        let decision = TaskRouter::route("Hello, how are you?", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
        assert!(decision.confidence <= 0.5);
    }

    #[test]
    fn test_route_mixed_keywords() {
        let decision = TaskRouter::route(
            "Review the architecture and implement the database migration",
            false,
        );
        // Should pick the highest-scoring category
        assert!(
            decision.primary_role == ModelRole::Planner
                || decision.primary_role == ModelRole::Coder
        );
    }

    #[test]
    fn test_coder_gets_reviewer() {
        let decision = TaskRouter::route("Fix the bug in the login function", false);
        assert_eq!(decision.primary_role, ModelRole::Coder);
        assert_eq!(decision.review_role, Some(ModelRole::Reviewer));
    }

    #[test]
    fn test_route_empty_input() {
        let decision = TaskRouter::route("", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
        assert!(decision.confidence <= 0.5);
    }

    #[test]
    fn test_route_single_char() {
        let decision = TaskRouter::route("a", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
    }

    #[test]
    fn test_route_case_insensitive() {
        let decision = TaskRouter::route("IMPLEMENT a feature", false);
        assert_eq!(decision.primary_role, ModelRole::Coder);
    }

    #[test]
    fn test_route_uppercase_keywords() {
        let decision = TaskRouter::route("DESIGN the ARCHITECTURE", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
    }

    #[test]
    fn test_route_vision_keywords_without_images() {
        let decision = TaskRouter::route("Analyze the screenshot of the UI layout", false);
        assert_eq!(decision.primary_role, ModelRole::Vision);
    }

    #[test]
    fn test_route_images_override_keywords() {
        // Even with planning keywords, images should route to vision
        let decision = TaskRouter::route("Design this image", true);
        assert_eq!(decision.primary_role, ModelRole::Vision);
        assert_eq!(decision.review_role, Some(ModelRole::Planner));
        assert!(decision.confidence >= 0.8);
    }

    #[test]
    fn test_route_planner_no_reviewer() {
        let decision = TaskRouter::route("Plan the architecture", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
        assert!(decision.review_role.is_none());
    }

    #[test]
    fn test_route_vision_no_reviewer() {
        let decision = TaskRouter::route("Look at this screenshot", false);
        assert_eq!(decision.primary_role, ModelRole::Vision);
        assert!(decision.review_role.is_none());
    }

    #[test]
    fn test_route_confidence_values() {
        // "implement code" has 2 coder keyword matches → 0.7
        let two_match = TaskRouter::route("implement code", false);
        assert_eq!(two_match.confidence, 0.7);

        // "implement code write tests" has 4 coder matches → 0.9 (>= 3)
        let four_match = TaskRouter::route("implement code write tests", false);
        assert_eq!(four_match.confidence, 0.9);
    }

    #[test]
    fn test_route_tie_planner_wins() {
        // Equal score for planner and coder: planner should win due to >= comparison
        let decision = TaskRouter::route("plan and implement", false);
        // "plan" matches planner (1), "implement" matches coder (1)
        assert_eq!(decision.primary_role, ModelRole::Planner);
    }

    #[test]
    fn test_route_decision_reason_non_empty() {
        let decision = TaskRouter::route("write some code", false);
        assert!(!decision.reason.is_empty());
        assert!(decision.reason.contains("Coding") || decision.reason.contains("coding"));
    }

    #[test]
    fn test_route_decision_confidence_range() {
        for _ in 0..10 {
            let decision = TaskRouter::route("implement a function", false);
            assert!(decision.confidence >= 0.0);
            assert!(decision.confidence <= 1.0);
        }
    }

    #[test]
    fn test_route_special_characters() {
        let decision = TaskRouter::route("fix bug!!! @#$%", false);
        assert_eq!(decision.primary_role, ModelRole::Coder);
    }

    #[test]
    fn test_route_unicode_input() {
        let decision = TaskRouter::route("implementar función en español", false);
        // "implementar" contains "implement" so coder should match
        assert_eq!(decision.primary_role, ModelRole::Coder);
    }

    #[test]
    fn test_route_decision_serde() {
        let decision = RouteDecision {
            primary_role: ModelRole::Coder,
            review_role: Some(ModelRole::Reviewer),
            confidence: 0.8,
            reason: "test".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let back: RouteDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision.primary_role, back.primary_role);
        assert_eq!(decision.confidence, back.confidence);
    }

    #[test]
    fn test_route_whitespace_only() {
        let decision = TaskRouter::route("   \t\n  ", false);
        assert_eq!(decision.primary_role, ModelRole::Planner);
    }

    #[test]
    fn test_route_newlines() {
        let decision = TaskRouter::route("implement\ncode\nand\ntest", false);
        assert_eq!(decision.primary_role, ModelRole::Coder);
    }
}
