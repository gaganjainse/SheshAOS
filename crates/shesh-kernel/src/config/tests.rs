use super::*;
use tempfile::TempDir;

fn sample_toml() -> &'static str {
    r#"
[general]
data_dir = "/tmp/shesh-test"
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
    assert_eq!(config.general.data_dir, "/tmp/shesh-test");
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
    assert_eq!(config.resolved_data_dir(), PathBuf::from("/tmp/shesh-test"));
}

#[test]
fn test_resolved_data_dir_tilde() {
    let toml = sample_toml().replace("/tmp/shesh-test", "~/.shesh");
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
    assert_eq!(config.general.data_dir, "/tmp/shesh-test");
}

#[test]
fn test_load_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, sample_toml()).unwrap();

    let config = AppConfig::load(config_path.to_str().unwrap()).expect("should load");
    assert_eq!(config.general.data_dir, "/tmp/shesh-test");
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
    assert_eq!(
        config.tools.filesystem.denied_patterns,
        vec!["**/.git/objects/**", "**/node_modules/**", "**/target/**"]
    );
    assert!(config.tools.git.enabled);
    assert_eq!(config.tools.terminal.timeout_secs, 30);
    assert_eq!(
        config.tools.terminal.denied_prefixes,
        vec!["rm -rf /", "sudo rm", "mkfs", "dd if="]
    );
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
