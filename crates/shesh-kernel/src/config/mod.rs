//! Application configuration for SheshAOS.
//!
//! Configuration is loaded from TOML files and provides all tunable parameters
//! for the kernel, model providers, tools, resource limits, and policies.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// General settings.
    pub general: GeneralConfig,

    /// Resource limits and budgets.
    pub resource_limits: ResourceLimitsConfig,

    /// Policy settings.
    pub policy: PolicyConfig,

    /// Context budget settings.
    pub context: ContextConfig,

    /// Model provider configurations.
    pub model_providers: Vec<ModelProviderConfig>,

    /// Tool configurations.
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Shutdown settings.
    #[serde(default)]
    pub shutdown: ShutdownConfig,
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Directory where SheshAOS stores events, snapshots, and artifacts.
    pub data_dir: String,

    /// Log level: trace, debug, info, warn, error.
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Resource limits to prevent system instability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum RAM usage in MB before refusing new work.
    pub max_ram_mb: u64,

    /// Maximum VRAM usage in MB before refusing model loads.
    pub max_vram_mb: u64,

    /// Maximum context tokens for any single inference request.
    pub max_context_tokens: usize,

    /// Maximum number of tasks in the scheduler queue.
    pub max_queue_depth: usize,

    /// Minimum free disk space in GB before refusing writes.
    pub min_disk_free_gb: u64,

    /// Maximum tool output size in bytes before truncation.
    #[serde(default = "default_max_tool_output_size")]
    pub max_tool_output_size: usize,
}

/// Policy enforcement settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Require confirmation for destructive actions.
    #[serde(default = "default_true")]
    pub confirm_destructive: bool,

    /// Require confirmation for file writes.
    #[serde(default = "default_true")]
    pub confirm_writes: bool,

    /// Require confirmation for git commits.
    #[serde(default = "default_true")]
    pub confirm_git_commits: bool,

    /// Require confirmation for terminal commands.
    #[serde(default = "default_true")]
    pub confirm_terminal: bool,

    /// Task deduplication window in seconds.
    #[serde(default = "default_dedup_window")]
    pub dedup_window_secs: u64,
}

/// Context budget configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Default context budget for simple questions (tokens).
    pub simple_question: usize,

    /// Default context budget for code edits (tokens).
    pub code_edit: usize,

    /// Default context budget for feature work (tokens).
    pub feature_work: usize,

    /// Default context budget for architecture reasoning (tokens).
    pub architecture: usize,

    /// RAM headroom required before allowing inference (MB).
    pub ram_headroom_mb: u64,

    /// VRAM headroom required before allowing inference (MB).
    pub vram_headroom_mb: u64,
}

/// Configuration for a single model provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderConfig {
    /// Human-readable name for this provider.
    pub name: String,

    /// Role this provider fills: planner, coder, vision, reviewer.
    pub role: String,

    /// Base URL of the OpenAI-compatible API.
    pub base_url: String,

    /// Model ID to request from the provider.
    pub model_id: String,

    /// Maximum context length in tokens.
    pub max_context: usize,

    /// Whether this provider supports vision/image input.
    #[serde(default)]
    pub supports_vision: bool,

    /// API key for providers that require authentication (e.g., Anthropic).
    #[serde(default)]
    pub api_key: String,

    /// Provider backend kind: openai, anthropic, etc.
    #[serde(default = "default_provider_kind")]
    pub provider_kind: String,
}

impl ModelProviderConfig {
    /// Create a model provider from the configuration.
    ///
    /// Returns `Ok(OpenAiCompatProvider)` for OpenAI-compatible providers
    /// or appropriate provider implementations.
    pub fn into_provider(
        &self,
    ) -> Result<Box<dyn crate::model::provider::ModelProvider>, crate::error::ProviderError> {
        match self.provider_kind.as_str() {
            "anthropic" | "claude" => {
                let role = match self.role.to_lowercase().as_str() {
                    "planner" => crate::state::ModelRole::Planner,
                    "coder" => crate::state::ModelRole::Coder,
                    "vision" => crate::state::ModelRole::Vision,
                    "reviewer" => crate::state::ModelRole::Reviewer,
                    _ => {
                        // This should be unreachable after config validation
                        return Err(crate::error::ProviderError::NoProviderForRole {
                            role: self.role.clone(),
                        });
                    }
                };
                Ok(Box::new(crate::model::claude::ClaudeProvider::new(
                    self.api_key.clone(),
                    self.model_id.clone(),
                    role,
                )?))
            }
            _ => {
                let provider = crate::model::openai_compat::OpenAiCompatProvider::new(self)?;
                Ok(Box::new(provider))
            }
        }
    }
}

/// Tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Filesystem tool configuration.
    #[serde(default)]
    pub filesystem: FilesystemToolConfig,

    /// Git tool configuration.
    #[serde(default)]
    pub git: GitToolConfig,

    /// Terminal tool configuration.
    #[serde(default)]
    pub terminal: TerminalToolConfig,
}

/// Filesystem tool settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemToolConfig {
    /// Allowed root paths for filesystem operations.
    #[serde(default)]
    pub allowed_paths: Vec<String>,

    /// Denied path patterns (glob).
    #[serde(default)]
    pub denied_patterns: Vec<String>,
}

impl Default for FilesystemToolConfig {
    fn default() -> Self {
        Self {
            allowed_paths: vec![".".to_string()],
            denied_patterns: vec![
                "**/.git/objects/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
            ],
        }
    }
}

/// Git tool settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitToolConfig {
    /// Whether git operations are enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for GitToolConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Terminal tool settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalToolConfig {
    /// Maximum execution time for terminal commands in seconds.
    #[serde(default = "default_terminal_timeout")]
    pub timeout_secs: u64,

    /// Allowed command prefixes (empty = all allowed but gated by confirmation).
    #[serde(default)]
    pub allowed_prefixes: Vec<String>,

    /// Denied command prefixes.
    #[serde(default)]
    pub denied_prefixes: Vec<String>,
}

impl Default for TerminalToolConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            allowed_prefixes: vec![],
            denied_prefixes: vec![
                "rm -rf /".to_string(),
                "sudo rm".to_string(),
                "mkfs".to_string(),
                "dd if=".to_string(),
            ],
        }
    }
}

/// Shutdown configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownConfig {
    /// Maximum time to wait for active tasks during shutdown (seconds).
    #[serde(default = "default_drain_timeout")]
    pub drain_timeout_secs: u64,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self { drain_timeout_secs: 10 }
    }
}

// Default value helpers for serde

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_dedup_window() -> u64 {
    5
}

fn default_terminal_timeout() -> u64 {
    30
}

fn default_drain_timeout() -> u64 {
    10
}

fn default_max_tool_output_size() -> usize {
    1_048_576
}

fn default_provider_kind() -> String {
    "openai".to_string()
}

impl AppConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(ConfigError::NotFound { path: path.display().to_string() });
        }
        let contents = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a TOML string (useful for testing).
    pub fn parse_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let config: AppConfig = toml::from_str(toml_str).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values after loading.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.model_providers.is_empty() {
            return Err(ConfigError::Invalid {
                message: "At least one model provider must be configured".to_string(),
            });
        }

        if self.resource_limits.max_queue_depth == 0 {
            return Err(ConfigError::Invalid {
                message: "max_queue_depth must be > 0".to_string(),
            });
        }

        if self.resource_limits.max_context_tokens == 0 {
            return Err(ConfigError::Invalid {
                message: "max_context_tokens must be > 0".to_string(),
            });
        }

        // Validate provider roles
        let valid_roles = ["planner", "coder", "vision", "reviewer"];
        let mut seen_names = HashSet::new();
        let mut has_planner = false;

        for provider in &self.model_providers {
            // Check for duplicate provider names
            if !seen_names.insert(&provider.name) {
                return Err(ConfigError::Invalid {
                    message: format!("Duplicate provider name: {}", provider.name),
                });
            }

            // Validate role
            let role_lower = provider.role.to_lowercase();
            if !valid_roles.contains(&role_lower.as_str()) {
                return Err(ConfigError::Invalid {
                    message: format!(
                        "Invalid role '{}' for provider '{}'. Valid roles: planner, coder, vision, reviewer",
                        provider.role, provider.name
                    ),
                });
            }

            if role_lower == "planner" {
                has_planner = true;
            }
        }

        if !has_planner {
            return Err(ConfigError::Invalid {
                message: "At least one provider with role 'planner' must be configured".to_string(),
            });
        }

        Ok(())
    }

    /// Resolve the data directory path, expanding `~` to the home directory.
    pub fn resolved_data_dir(&self) -> PathBuf {
        let path = &self.general.data_dir;
        if let (Some(rest), Some(home)) = (path.strip_prefix('~'), dirs_home()) {
            return PathBuf::from(format!("{}{}", home, rest));
        }
        PathBuf::from(path)
    }
}

impl std::str::FromStr for AppConfig {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_toml(s)
    }
}

/// Get the home directory path.
fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

#[cfg(test)]
mod tests;
