//! Application configuration for NexusAOS.
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
    /// Directory where NexusAOS stores events, snapshots, and artifacts.
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
    pub fn into_provider(&self) -> Result<Box<dyn crate::model::provider::ModelProvider>, crate::error::ProviderError> {
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
                )))
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
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_toml() -> &'static str {
        r#"
[general]
data_dir = "/tmp/nexusaos-test"
log_level = "debug"

[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5

[policy]
confirm_destructive = true
confirm_writes = true
confirm_git_commits = true
confirm_terminal = true
dedup_window_secs = 5

[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test-planner"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test-model"
max_context = 32768
"#
    }

    #[test]
    fn test_load_from_string() {
        let config = AppConfig::parse_toml(sample_toml()).expect("should parse");
        assert_eq!(config.general.data_dir, "/tmp/nexusaos-test");
        assert_eq!(config.general.log_level, "debug");
        assert_eq!(config.resource_limits.max_ram_mb, 12288);
        assert_eq!(config.model_providers.len(), 1);
        assert_eq!(config.model_providers[0].name, "test-planner");
    }

    #[test]
    fn test_validation_no_providers() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024
"#;
        let result = AppConfig::parse_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolved_data_dir_absolute() {
        let config = AppConfig::parse_toml(sample_toml()).expect("should parse");
        assert_eq!(config.resolved_data_dir(), PathBuf::from("/tmp/nexusaos-test"));
    }

    #[test]
    fn test_resolved_data_dir_tilde() {
        let toml = sample_toml().replace("/tmp/nexusaos-test", "~/.nexusaos");
        let config = AppConfig::parse_toml(&toml).expect("should parse");
        let resolved = config.resolved_data_dir();
        // Should expand ~ to home directory
        assert!(!resolved.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn test_default_tool_config() {
        let config = AppConfig::parse_toml(sample_toml()).expect("should parse");
        assert!(config.tools.git.enabled);
        assert_eq!(config.tools.terminal.timeout_secs, 30);
        assert!(!config.tools.terminal.denied_prefixes.is_empty());
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = AppConfig::parse_toml(sample_toml()).expect("should parse");
        let json = serde_json::to_string(&config).expect("should serialize");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.general.data_dir, config.general.data_dir);
    }

    #[test]
    fn test_validation_zero_max_queue_depth() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 0
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test-model"
max_context = 32768
"#;
        let result = AppConfig::parse_toml(toml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("max_queue_depth"));
    }

    #[test]
    fn test_validation_zero_max_context_tokens() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 0
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test-model"
max_context = 32768
"#;
        let result = AppConfig::parse_toml(toml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("max_context_tokens"));
    }

    #[test]
    fn test_from_str() {
        let config: AppConfig = sample_toml().parse().expect("should parse");
        assert_eq!(config.general.data_dir, "/tmp/nexusaos-test");
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        std::fs::write(&config_path, sample_toml()).unwrap();

        let config = AppConfig::load(config_path.to_str().unwrap()).expect("should load");
        assert_eq!(config.general.data_dir, "/tmp/nexusaos-test");
        assert_eq!(config.model_providers.len(), 1);
    }

    #[test]
    fn test_load_missing_file() {
        let result = AppConfig::load("/nonexistent/path/config.toml");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::NotFound { .. } => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad.toml");
        std::fs::write(&config_path, "this is not valid toml {{{").unwrap();

        let result = AppConfig::load(config_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_model_providers() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "planner"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "model-a"
max_context = 32768

[[model_providers]]
name = "coder"
role = "coder"
base_url = "http://127.0.0.1:1235"
model_id = "model-b"
max_context = 16384
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.model_providers.len(), 2);
        assert_eq!(config.model_providers[0].name, "planner");
        assert_eq!(config.model_providers[1].name, "coder");
    }

    #[test]
    fn test_default_log_level() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.general.log_level, "info"); // default
    }

    #[test]
    fn test_default_policy_values() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
confirm_destructive = true
confirm_writes = true
confirm_git_commits = true
confirm_terminal = true
dedup_window_secs = 5
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert!(config.policy.confirm_destructive);
        assert!(config.policy.confirm_writes);
        assert!(config.policy.confirm_git_commits);
        assert!(config.policy.confirm_terminal);
        assert_eq!(config.policy.dedup_window_secs, 5);
    }

    #[test]
    fn test_default_shutdown_config() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.shutdown.drain_timeout_secs, 10);
    }

    #[test]
    fn test_default_tools_config() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.tools.filesystem.allowed_paths, vec!["."]);
        assert_eq!(config.tools.filesystem.denied_patterns, vec!["**/.git/objects/**", "**/node_modules/**", "**/target/**"]);
        assert!(config.tools.git.enabled);
        assert_eq!(config.tools.terminal.timeout_secs, 30);
        assert_eq!(config.tools.terminal.denied_prefixes, vec!["rm -rf /", "sudo rm", "mkfs", "dd if="]);
    }

    #[test]
    fn test_resolved_data_dir_nonexistent_home() {
        // Even if HOME is unset (which it shouldn't be in practice), the function
        // should handle it gracefully by returning the path as-is when it doesn't start with ~
        let toml = r#"
[general]
data_dir = "/absolute/path"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.resolved_data_dir(), PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_model_provider_config_serde() {
        let toml = r#"
name = "test"
role = "vision"
base_url = "http://localhost:11434"
model_id = "llava"
max_context = 4096
supports_vision = true
"#;
        let config: ModelProviderConfig = toml::from_str(toml).expect("should parse");
        assert_eq!(config.name, "test");
        assert_eq!(config.role, "vision");
        assert!(config.supports_vision);
    }

    #[test]
    fn test_config_parse_io_error() {
        // Passing invalid TOML should give a parse error
        let result = AppConfig::parse_toml("[[not valid");
        assert!(result.is_err());
    }

    #[test]
    fn test_context_config_fields() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5
[policy]
[context]
simple_question = 4096
code_edit = 8192
feature_work = 16384
architecture = 32768
ram_headroom_mb = 1024
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.context.simple_question, 4096);
        assert_eq!(config.context.code_edit, 8192);
        assert_eq!(config.context.feature_work, 16384);
        assert_eq!(config.context.architecture, 32768);
        assert_eq!(config.context.ram_headroom_mb, 1024);
    }

    #[test]
    fn test_resource_limits_fields() {
        let toml = r#"
[general]
data_dir = "/tmp/test"
[resource_limits]
max_ram_mb = 8192
max_vram_mb = 4096
max_context_tokens = 32768
max_queue_depth = 16
min_disk_free_gb = 10
[policy]
[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048
vram_headroom_mb = 1024

[[model_providers]]
name = "test"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "test"
max_context = 32768
"#;
        let config = AppConfig::parse_toml(toml).expect("should parse");
        assert_eq!(config.resource_limits.max_ram_mb, 8192);
        assert_eq!(config.resource_limits.max_vram_mb, 4096);
        assert_eq!(config.resource_limits.max_context_tokens, 32768);
        assert_eq!(config.resource_limits.max_queue_depth, 16);
        assert_eq!(config.resource_limits.min_disk_free_gb, 10);
    }
}
