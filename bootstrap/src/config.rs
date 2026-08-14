use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub backend_url: String,
    pub policy_enforced: bool,
    pub resource_budgets: ResourceBudgets,
    pub work_dir: PathBuf,
    pub log_level: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceBudgets {
    pub max_ram_mb: u64,
    pub max_vram_mb: u64,
    pub max_context_length: usize,
}

impl Default for ResourceBudgets {
    fn default() -> Self {
        Self {
            max_ram_mb: 4096,
            max_vram_mb: 8192,
            max_context_length: 32768,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.resource_budgets.max_ram_mb, 4096);
        assert!(!config.policy_enforced);
    }
}
