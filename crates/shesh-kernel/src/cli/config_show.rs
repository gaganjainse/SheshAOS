//! `shesh config` — Show resolved configuration.

use tracing::info;

use crate::config::{AppConfig, ModelProviderConfig, ToolsConfig};
use crate::error::KernelError;

/// Display the resolved configuration.
pub fn run(config_path: &str) -> Result<(), KernelError> {
    info!("Showing resolved configuration");

    let config = AppConfig::load(config_path)?;

    println!("SheshAOS Configuration");
    println!("----------------------");

    match toml::to_string_pretty(&redact_config(&config)) {
        Ok(toml_str) => println!("{}", toml_str),
        Err(e) => {
            println!("Error formatting config: {}", e);
        }
    }

    Ok(())
}

/// Redact sensitive values from configuration before display.
fn redact_config(config: &AppConfig) -> RedactedConfig {
    use crate::config::{ModelProviderConfig, ToolsConfig};

    RedactedConfig {
        general: config.general.clone(),
        resource_limits: config.resource_limits.clone(),
        policy: config.policy.clone(),
        context: config.context.clone(),
        model_providers: config
            .model_providers
            .iter()
            .map(|p| ModelProviderConfig {
                name: p.name.clone(),
                role: p.role.clone(),
                base_url: p.base_url.clone(),
                model_id: p.model_id.clone(),
                max_context: p.max_context,
                supports_vision: p.supports_vision,
                api_key: "[REDACTED]".to_string(),
                provider_kind: p.provider_kind.clone(),
            })
            .collect(),
        tools: ToolsConfig {
            filesystem: config.tools.filesystem.clone(),
            git: config.tools.git.clone(),
            terminal: config.tools.terminal.clone(),
        },
        shutdown: config.shutdown.clone(),
    }
}

/// A copy of AppConfig with sensitive fields redacted.
#[derive(serde::Serialize)]
struct RedactedConfig {
    general: crate::config::GeneralConfig,
    resource_limits: crate::config::ResourceLimitsConfig,
    policy: crate::config::PolicyConfig,
    context: crate::config::ContextConfig,
    model_providers: Vec<ModelProviderConfig>,
    tools: ToolsConfig,
    shutdown: crate::config::ShutdownConfig,
}
