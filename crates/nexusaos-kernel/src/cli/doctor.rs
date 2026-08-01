//! `nexusaos doctor` — Check system health and prerequisites.

use tracing::info;

use crate::{
    config::AppConfig,
    error::NexusError,
    model::{openai_compat::OpenAiCompatProvider, provider::ModelProvider},
    resource::ResourceMonitor,
};

/// Run the doctor command: verify system prerequisites.
pub fn run(config_path: &str) -> Result<(), NexusError> {
    info!("Running system health check");
    println!("NexusAOS Doctor\n");

    // Check config
    print!("  Configuration ... ");
    let config = match AppConfig::load(config_path) {
        Ok(c) => {
            println!("✓ valid");
            c
        }
        Err(e) => {
            println!("✗ {}", e);
            return Err(e.into());
        }
    };

    // Check data directory
    print!("  Data directory ... ");
    let data_dir = config.resolved_data_dir();
    if data_dir.exists() {
        let test_file = data_dir.join(".write_test");
        if std::fs::write(&test_file, b"ok").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            println!("✓ exists and writable ({})", data_dir.display());
        } else {
            println!("✗ exists but NOT writable ({})", data_dir.display());
        }
    } else {
        println!("✗ not found (run `nexusaos init`)");
    }

    // Check system resources
    print!("  System resources ... ");
    let pressure = ResourceMonitor::snapshot();
    println!(
        "✓ RAM: {} MB free, Disk: {} GB free",
        pressure.ram_available_mb, pressure.disk_available_gb
    );

    // Check GPU
    print!("  GPU VRAM ... ");
    if pressure.vram_available_mb > 0 {
        println!("✓ {} MB free", pressure.vram_available_mb);
    } else {
        println!("⚠ could not detect (nvidia-smi unavailable?)");
    }

    println!("\n  Model Providers:");
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        NexusError::Config(crate::error::ConfigError::Invalid { message: e.to_string() })
    })?;
    rt.block_on(async {
        for provider_cfg in &config.model_providers {
            print!("    - {} ({}) ... ", provider_cfg.name, provider_cfg.role);
            match OpenAiCompatProvider::new(provider_cfg) {
                Ok(provider) => match provider.health_check().await {
                    Ok(true) => println!("✓ online"),
                    Ok(false) => println!("✗ offline"),
                    Err(e) => println!("✗ error: {}", e),
                },
                Err(e) => println!("✗ config error: {}", e),
            }
        }
    });

    println!("\nHealth check complete.");
    Ok(())
}
