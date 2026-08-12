//! SheshAOS CLI entrypoint.

use clap::{CommandFactory, Parser, Subcommand};

/// SheshAOS — Governance-first AI operating environment
#[derive(Parser, Debug)]
#[command(name = "shesh", version, about, long_about = None)]
struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to configuration file
    #[arg(short, long, default_value = "configs/default.toml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new SheshAOS data directory
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

    /// Manage stored command snippets (Komandi Vault)
    Vault {
        /// Action: list, add
        #[arg(default_value = "list")]
        action: String,
    },

    /// Explain CLI flags for a command string (Dry-Run Inspector)
    Explain {
        /// The command string to analyze
        command: String,
    },
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

    match cli.command {
        // The interactive mission-control surface is stock Wave Terminal
        // (ADR-0016); the bespoke ratatui/iced frontends were removed in the
        // 2026-08-12 excision (ADR-0018), so bare `shesh` prints help.
        None => {
            Cli::command().print_help()?;
            println!();
        }
        Some(Commands::Init) => shesh_kernel::cli::init::run(&cli.config)?,
        Some(Commands::Doctor) => shesh_kernel::cli::doctor::run(&cli.config)?,
        Some(Commands::Status) => shesh_kernel::cli::status::run(&cli.config)?,
        Some(Commands::Run { task, background, yes }) => {
            shesh_kernel::cli::run::execute(&cli.config, &task, background, yes)?
        }
        Some(Commands::Replay { task_id }) => {
            shesh_kernel::cli::replay::run(&cli.config, &task_id)?
        }
        Some(Commands::Config) => shesh_kernel::cli::config_show::run(&cli.config)?,
        Some(Commands::Vault { action }) => {
            println!("SheshAOS Command Vault [{}]", action);
            let vault_path = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("cannot resolve home directory"))?
                .join(".shesh/data/commands.jsonl");
            let store = shesh_vault::snippet::VaultStore::new(vault_path.clone());
            match store.load_all() {
                Ok(loaded) => {
                    println!("Loaded {} saved snippets from vault.", loaded.len())
                }
                Err(e) if vault_path.exists() => {
                    // The file exists but failed to parse — never pretend the
                    // vault is empty; that hides data loss.
                    anyhow::bail!("vault at {} is unreadable: {}", vault_path.display(), e)
                }
                Err(_) => println!("No vault yet ({} not found).", vault_path.display()),
            }
        }
        Some(Commands::Explain { command }) => {
            println!("SheshAOS Flag Inspector for command: {}", command);
            let flags = shesh_vault::inspector::FlagInspector::explain_flags(&command);
            for (flag, exp) in flags {
                println!("  {:12} -> {}", flag, exp);
            }
        }
    }

    Ok(())
}
