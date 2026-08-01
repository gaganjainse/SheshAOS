//! Context manager for NexusAOS.
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
    /// Estimate complexity from task input text length (heuristic).
    pub fn estimate_from_input(input_len: usize, has_attachments: bool) -> Self {
        if has_attachments {
            return TaskComplexity::Feature;
        }
        match input_len {
            0..=200 => TaskComplexity::Simple,
            201..=1000 => TaskComplexity::CodeEdit,
            1001..=5000 => TaskComplexity::Feature,
            _ => TaskComplexity::Architecture,
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
        if pressure.ram_available_mb < self.config.ram_headroom_mb {
            // Under memory pressure: halve the budget
            let reduced = max_tokens / 2;
            if reduced < self.config.simple_question {
                return Err(ResourceError::InsufficientRam {
                    needed_mb: self.config.ram_headroom_mb,
                    available_mb: pressure.ram_available_mb,
                });
            }
            max_tokens = reduced;
            was_clamped = true;
            clamp_reason = Some(format!(
                "RAM pressure: {} MB available, {} MB headroom required. Budget halved.",
                pressure.ram_available_mb, self.config.ram_headroom_mb
            ));
        }

        Ok(ContextBudget { max_tokens, complexity, was_clamped, clamp_reason })
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
            vram_available_mb: 1000,
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
        assert_eq!(TaskComplexity::estimate_from_input(50, false), TaskComplexity::Simple);
        assert_eq!(TaskComplexity::estimate_from_input(500, false), TaskComplexity::CodeEdit);
        assert_eq!(TaskComplexity::estimate_from_input(3000, false), TaskComplexity::Feature);
        assert_eq!(TaskComplexity::estimate_from_input(10000, false), TaskComplexity::Architecture);
        // Attachments bump to Feature minimum
        assert_eq!(TaskComplexity::estimate_from_input(50, true), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_boundary_exact_200() {
        assert_eq!(TaskComplexity::estimate_from_input(200, false), TaskComplexity::Simple);
        assert_eq!(TaskComplexity::estimate_from_input(201, false), TaskComplexity::CodeEdit);
    }

    #[test]
    fn test_complexity_boundary_exact_1000() {
        assert_eq!(TaskComplexity::estimate_from_input(1000, false), TaskComplexity::CodeEdit);
        assert_eq!(TaskComplexity::estimate_from_input(1001, false), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_boundary_exact_5000() {
        assert_eq!(TaskComplexity::estimate_from_input(5000, false), TaskComplexity::Feature);
        assert_eq!(TaskComplexity::estimate_from_input(5001, false), TaskComplexity::Architecture);
    }

    #[test]
    fn test_complexity_zero_input_with_attachments() {
        assert_eq!(TaskComplexity::estimate_from_input(0, true), TaskComplexity::Feature);
    }

    #[test]
    fn test_complexity_zero_input_no_attachments() {
        assert_eq!(TaskComplexity::estimate_from_input(0, false), TaskComplexity::Simple);
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
        // Very large input should be Architecture
        assert_eq!(TaskComplexity::estimate_from_input(usize::MAX, false), TaskComplexity::Architecture);
    }

    #[test]
    fn test_context_budget_complexity_field() {
        let mgr = ContextManager::new(test_config());
        let budget = mgr.estimate_budget(TaskComplexity::CodeEdit, &normal_pressure(), 65536).unwrap();
        assert_eq!(budget.complexity, TaskComplexity::CodeEdit);
    }
}
