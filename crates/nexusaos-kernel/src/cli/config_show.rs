//! `nexusaos config` — Show resolved configuration.

use tracing::info;

use crate::{config::AppConfig, error::NexusError};

/// Display the resolved configuration.
pub fn run(config_path: &str) -> Result<(), NexusError> {
    info!("Showing resolved configuration");

    let config = AppConfig::load(config_path)?;

    println!("NexusAOS Configuration");
    println!("----------------------");

    match toml::to_string_pretty(&config) {
        Ok(toml_str) => println!("{}", toml_str),
        Err(e) => {
            println!("Error formatting config: {}", e);
        }
    }

    Ok(())
}
