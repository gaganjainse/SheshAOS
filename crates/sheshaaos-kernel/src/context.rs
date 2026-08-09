//! Context manager for SheshaAOS.
//!
//! Estimates token budgets for inference requests based on task complexity,
//! system pressure, and model capabilities. Ensures the system never exceeds
//! safe resource limits.

use serde::{Deserialize, Serialize};

use crate::{config::ContextConfig, error::ResourceError, resource::SystemPressure};

/// Task complexity categories for context budget estimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskComplexity {
    /// Simple question or lookup — minimal context needed.
    Simple,
    /// Small code edit or single-file change.
    CodeEdit,
    /// Multi-file feature work.
    Feature,
    /// Architecture reasoning, large repo analysis.
    Architecture,
}

impl TaskComplexity {
    /// Estimate complexity from task input text and content heuristics.
    pub fn estimate_from_input(input: &str, has_attachments: bool) -> Self {
        if has_attachments {
            return TaskComplexity::Feature;
        }

        let code_keywords = ["fn ", "struct ", "impl ", "class ", "def ", "function ", "async ", "pub ", "mod "];
        let file_path_indicators = ["src/", "tests/", "lib/", "Cargo.toml", "package.json", ".rs", ".py", ".ts", ".js"];
        let architecture_patterns = ["refactor", "redesign", "architecture", "migrate", "rewrite", "system design"];
        let requirement_keywords = ["require", "must ", "should ", "implement", "feature", "bug", "fix ", "issue ", "ticket"];
        let error_patterns = ["error:", "panic:", "traceback", "exception:", "failed to", "fatal:"];

        let input_lower = input.to_lowercase();

        let has_code = code_keywords.iter().any(|kw| input_lower.contains(kw));
        let has_file_paths = file_path_indicators.iter().any(|ind| input.contains(ind));
        let has_architecture = architecture_patterns.iter().any(|pat| input_lower.contains(pat));
        let has_requirements = requirement_keywords.iter().any(|kw| input_lower.contains(kw));
        let has_errors = error_patterns.iter().any(|pat| input_lower.contains(pat));

        // Count distinct file references
        let file_count = file_path_indicators.iter().filter(|ind| input.contains(**ind)).count();
        let multiple_files = file_count >= 2;

        // Count lines — more lines suggests more content to process
        let line_count = input.lines().count();

        let input_len = input.len();

        if has_architecture || input_len > 5000 || (has_code && multiple_files && line_count > 50) {
            TaskComplexity::Architecture
        } else if (has_code && has_file_paths && has_requirements && input_len > 200)
            || (has_code && multiple_files)
            || (has_code && has_errors && input_len > 300)
        {
            TaskComplexity::Feature
        } else {
            match input_len {
                0..=200 => TaskComplexity::Simple,
                201..=1000 => TaskComplexity::CodeEdit,
                1001..=5000 => TaskComplexity::Feature,
                _ => if has_code || has_file_paths {
                    TaskComplexity::Feature
                } else {
                    TaskComplexity::Architecture
                },
            }
        }
    }
}

/// A validated context budget for an inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Maximum tokens allowed for this request.
    pub max_tokens: usize,

    /// The complexity category that was estimated.
    pub complexity: TaskComplexity,

    /// Whether the budget was clamped due to resource pressure.
    pub was_clamped: bool,

    /// Reason for clamping, if applicable.
    pub clamp_reason: Option<String>,
}

/// The context manager estimates and validates context budgets.
pub struct ContextManager {
    config: ContextConfig,
}

impl ContextManager {
    /// Create a new context manager with the given configuration.
    pub fn new(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Estimate a context budget for a task.
    ///
    /// Takes into account:
    /// - Task complexity
    /// - System resource pressure
    /// - Model maximum context
    /// - Configuration limits
    pub fn estimate_budget(
        &self,
        complexity: TaskComplexity,
        pressure: &SystemPressure,
        model_max_context: usize,
    ) -> Result<ContextBudget, ResourceError> {
        // Base budget from complexity
        let base_budget = match complexity {
            TaskComplexity::Simple => self.config.simple_question,
            TaskComplexity::CodeEdit => self.config.code_edit,
            TaskComplexity::Feature => self.config.feature_work,
            TaskComplexity::Architecture => self.config.architecture,
        };

        let mut max_tokens = base_budget;
        let mut was_clamped = false;
        let mut clamp_reason = None;

        // Clamp to model maximum
        if max_tokens > model_max_context {
            max_tokens = model_max_context;
            was_clamped = true;
            clamp_reason =
                Some(format!("Clamped to model max context: {} tokens", model_max_context));
        }

        // Check memory pressure — refuse if RAM is too tight
        self.check_pressure(
            pressure.ram_available_mb,
            self.config.ram_headroom_mb,
            &mut max_tokens,
            &mut was_clamped,
            &mut clamp_reason,
            "RAM",
            |needed, available| ResourceError::InsufficientRam {
                needed_mb: needed,
                available_mb: available,
            },
        )?;

        // Check VRAM pressure — halve budget if VRAM is too tight
        self.check_pressure(
            pressure.vram_available_mb,
            self.config.vram_headroom_mb,
            &mut max_tokens,
            &mut was_clamped,
            &mut clamp_reason,
            "VRAM",
            |needed, available| ResourceError::InsufficientVram {
                needed_mb: needed,
                available_mb: available,
            },
        )?;

        Ok(ContextBudget { max_tokens, complexity, was_clamped, clamp_reason })
    }

    #[allow(clippy::too_many_arguments)]
    fn check_pressure(
        &self,
        available_mb: u64,
        headroom_mb: u64,
        max_tokens: &mut usize,
        was_clamped: &mut bool,
        clamp_reason: &mut Option<String>,
        label: &str,
        make_error: impl FnOnce(u64, u64) -> ResourceError,
    ) -> Result<(), ResourceError> {
        if available_mb < headroom_mb {
            let reduced = *max_tokens / 2;
            if reduced < self.config.simple_question {
                return Err(make_error(headroom_mb, available_mb));
            }
            *max_tokens = reduced;
            *was_clamped = true;
            *clamp_reason = Some(format!(
                "{} pressure: {} MB available, {} MB headroom required. Budget halved.",
                label, available_mb, headroom_mb
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ContextConfig {
        ContextConfig {
            simple_question: 8192,
            code_edit: 16384,
            feature_work: 32768,
            architecture: 65536,
            ram_headroom_mb: 2048,
            vram_headroom_mb: 1024,
        }
    }

    fn normal_pressure() -> SystemPressure {
        SystemPressure {
            ram_available_mb: 8000,
            ram_total_mb: 16000,
            vram_available_mb: 4000,
            vram_total_mb: 6000,
            disk_available_gb: 100,
            queue_depth: 0,
        }
    }

    fn high_pressure() -> SystemPressure {
        SystemPressure {
            ram_available_mb: 1000,
            ram_total_mb: 16000,
            vram_available_mb: 2000,
            vram_total_mb: 6000,
            disk_available_gb: 100,
            queue_depth: 20,
        }
    }

    #[test]
    fn test_simple_budget() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr
            .estimate_budget(TaskComplexity::Simple, &normal_pressure(), 65536)
            .expect("should succeed");
        assert_eq!(budget.max_tokens, 8192);
        assert!(!budget.was_clamped);
    }

    #[test]
    fn test_architecture_budget() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr
            .estimate_budget(TaskComplexity::Architecture, &normal_pressure(), 65536)
            .expect("should succeed");
        assert_eq!(budget.max_tokens, 65536);
    }

    #[test]
    fn test_clamp_to_model_max() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr
            .estimate_budget(TaskComplexity::Architecture, &normal_pressure(), 32768)
            .expect("should succeed");
        assert_eq!(budget.max_tokens, 32768);
        assert!(budget.was_clamped);
    }

    #[test]
    fn test_memory_pressure_halves_budget() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr
            .estimate_budget(TaskComplexity::Feature, &high_pressure(), 65536)
            .expect("should succeed");
        assert_eq!(budget.max_tokens, 16384); // 32768 / 2
        assert!(budget.was_clamped);
    }

    #[test]
    fn test_memory_pressure_refuses_tiny_budget() {
        let mgr = ContextManager::new(test_config());
        // Simple question halved = 4096, which is < simple_question (8192)
        let result = mgr.estimate_budget(TaskComplexity::Simple, &high_pressure(), 65536);
        assert!(result.is_err());
    }

    #[test]
    fn test_complexity_estimation() {
        assert_eq!(TaskComplexity::estimate_from_input("hello", false), TaskComplexity::Simple);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(500), false), TaskComplexity::CodeEdit);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(3000), false), TaskComplexity::Feature);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(10000), false), TaskComplexity::Architecture);
        assert_eq!(TaskComplexity::estimate_from_input("short", true), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_boundary_exact_200() {
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(200), false), TaskComplexity::Simple);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(201), false), TaskComplexity::CodeEdit);
    }

    #[test]
    fn test_complexity_boundary_exact_1000() {
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(1000), false), TaskComplexity::CodeEdit);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(1001), false), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_boundary_exact_5000() {
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(5000), false), TaskComplexity::Feature);
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(5001), false), TaskComplexity::Architecture);
    }

    #[test]
    fn test_complexity_zero_input_with_attachments() {
        assert_eq!(TaskComplexity::estimate_from_input("", true), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_zero_input_no_attachments() {
        assert_eq!(TaskComplexity::estimate_from_input("", false), TaskComplexity::Simple);
    }

    #[test]
    fn test_budget_all_complexity_levels() {
        let mgr = ContextManager::new(test_config());
        let pressure = normal_pressure();

        let simple = mgr.estimate_budget(TaskComplexity::Simple, &pressure, 65536).unwrap();
        assert_eq!(simple.max_tokens, 8192);
        assert!(!simple.was_clamped);
        assert!(simple.clamp_reason.is_none());

        let code = mgr.estimate_budget(TaskComplexity::CodeEdit, &pressure, 65536).unwrap();
        assert_eq!(code.max_tokens, 16384);
        assert!(!code.was_clamped);
        assert!(code.clamp_reason.is_none());

        let feature = mgr.estimate_budget(TaskComplexity::Feature, &pressure, 65536).unwrap();
        assert_eq!(feature.max_tokens, 32768);
        assert!(!feature.was_clamped);
        assert!(feature.clamp_reason.is_none());

        let arch = mgr.estimate_budget(TaskComplexity::Architecture, &pressure, 65536).unwrap();
        assert_eq!(arch.max_tokens, 65536);
        assert!(!arch.was_clamped);
        assert!(arch.clamp_reason.is_none());
    }

    #[test]
    fn test_budget_clamp_reason_set() {
        let mgr = ContextManager::new(test_config());
        let pressure = normal_pressure();
        let budget = mgr.estimate_budget(TaskComplexity::Architecture, &pressure, 32768).unwrap();
        assert!(budget.was_clamped);
        assert!(budget.clamp_reason.is_some());
        assert!(budget.clamp_reason.unwrap().contains("32768"));
    }

    #[test]
    fn test_budget_no_clamp_reason_when_not_clamped() {
        let mgr = ContextManager::new(test_config());
        let pressure = normal_pressure();
        let budget = mgr.estimate_budget(TaskComplexity::Simple, &pressure, 65536).unwrap();
        assert!(!budget.was_clamped);
        assert!(budget.clamp_reason.is_none());
    }

    #[test]
    fn test_budget_memory_pressure_clamp_reason() {
        let mgr = ContextManager::new(test_config());
        let pressure = high_pressure();
        let budget = mgr.estimate_budget(TaskComplexity::Feature, &pressure, 65536).unwrap();
        assert!(budget.was_clamped);
        assert!(budget.clamp_reason.is_some());
        let reason = budget.clamp_reason.unwrap();
        assert!(reason.contains("RAM pressure"));
        assert!(reason.contains("1000"));
        assert!(reason.contains("2048"));
    }

    #[test]
    fn test_budget_model_max_zero() {
        let mgr = ContextManager::new(test_config());
        let pressure = normal_pressure();
        // model_max_context = 0 means everything gets clamped to 0
        let budget = mgr.estimate_budget(TaskComplexity::Simple, &pressure, 0).unwrap();
        assert_eq!(budget.max_tokens, 0);
        assert!(budget.was_clamped);
    }

    #[test]
    fn test_budget_model_max_less_than_simple_question() {
        let mgr = ContextManager::new(test_config());
        let pressure = normal_pressure();
        // model max is 4096, simple_question is 8192, so clamped to 4096
        let budget = mgr.estimate_budget(TaskComplexity::Simple, &pressure, 4096).unwrap();
        assert_eq!(budget.max_tokens, 4096);
        assert!(budget.was_clamped);
    }

    #[test]
    fn test_estimate_from_input_max_usize() {
        assert_eq!(TaskComplexity::estimate_from_input(&"x".repeat(10_000_000), false), TaskComplexity::Architecture);
    }

    #[test]
    fn test_context_budget_complexity_field() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr.estimate_budget(TaskComplexity::CodeEdit, &normal_pressure(), 65536).unwrap();
        assert_eq!(budget.complexity, TaskComplexity::CodeEdit);
    }
}
