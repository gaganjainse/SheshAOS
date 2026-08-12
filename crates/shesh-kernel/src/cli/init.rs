//! `shesh init` — Initialize a new SheshAOS data directory.

use std::path::Path;

use tracing::info;

use crate::error::KernelError;

/// Run the init command: create data directory and write default config.
pub fn run(config_path: &str) -> Result<(), KernelError> {
    info!("Initializing SheshAOS with config: {}", config_path);

    let config_path_obj = Path::new(config_path);
    let config = if config_path_obj.exists() {
        crate::config::AppConfig::load(config_path)?
    } else {
        let toml_str = r#"
[general]
data_dir = "~/.shesh"
log_level = "info"

[resource_limits]
max_ram_mb = 16384
max_vram_mb = 8192
max_context_tokens = 128000
max_queue_depth = 100
min_disk_free_gb = 10

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
name = "default-planner"
role = "planner"
base_url = "http://127.0.0.1:11434"
model_id = "llama3"
max_context = 128000
supports_vision = false
"#;
        if let Some(parent) = config_path_obj.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(config_path_obj, toml_str.trim())?;
        println!("✓ Created default config file at {}", config_path);
        crate::config::AppConfig::parse_toml(toml_str.trim())?
    };

    let data_dir = config.resolved_data_dir();

    if data_dir.exists() {
        info!("Data directory already exists: {}", data_dir.display());
    } else {
        std::fs::create_dir_all(&data_dir)?;
        info!("Created data directory: {}", data_dir.display());
    }

    // Create subdirectories
    let subdirs = ["events", "snapshots", "artifacts"];
    for sub in &subdirs {
        let path = data_dir.join(sub);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            info!("Created: {}", path.display());
        }
    }

    println!("✓ SheshAOS initialized at {}", data_dir.display());
    Ok(())
}
