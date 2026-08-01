//! NexusAOS CLI entrypoint.

use clap::{Parser, Subcommand};

/// NexusAOS — Governance-first AI operating environment
#[derive(Parser, Debug)]
#[command(name = "nexusaos", version, about, long_about = None)]
struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to configuration file
    #[arg(short, long, default_value = "configs/default.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new NexusAOS data directory
    Init,

    /// Check system health and prerequisites
    Doctor,

    /// Show current kernel state, active tasks, and resource pressure
    Status,

    /// Submit a task for execution
    Run {
        /// The task description
        task: String,

        /// Run in background without waiting for completion
        #[arg(long)]
        background: bool,

        /// Skip confirmation prompts (trust mode)
        #[arg(long)]
        yes: bool,
    },

    /// Replay event history for a task
    Replay {
        /// The task ID to replay
        task_id: String,
    },

    /// Show resolved configuration
    Config,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(false)
        .init();

    tracing::info!("NexusAOS v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Init => nexusaos::cli::init::run(&cli.config)?,
        Commands::Doctor => nexusaos::cli::doctor::run(&cli.config)?,
        Commands::Status => nexusaos::cli::status::run(&cli.config)?,
        Commands::Run { task, background, yes } => {
            nexusaos::cli::run::execute(&cli.config, &task, background, yes)?
        }
        Commands::Replay { task_id } => nexusaos::cli::replay::run(&cli.config, &task_id)?,
        Commands::Config => nexusaos::cli::config_show::run(&cli.config)?,
    }

    Ok(())
}
