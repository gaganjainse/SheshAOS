//! SheshAOS CLI entrypoint.

use clap::{Parser, Subcommand};

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
    /// Launch interactive terminal TUI session (Claude Code / Antigravity style)
    Tui,

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

    /// Test native Zig VT100 parser & PTY shell bridge
    Pty,
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
        None | Some(Commands::Tui) => run_interactive_tui()?,
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
            let vault_path = std::path::PathBuf::from("~/.shesh/data/commands.jsonl");
            let store = shesh_vault::snippet::VaultStore::new(vault_path);
            let loaded = store.load_all().unwrap_or_default();
            println!("Loaded {} saved snippets from vault.", loaded.len());
        }
        Some(Commands::Explain { command }) => {
            println!("SheshAOS Flag Inspector for command: {}", command);
            let flags = shesh_vault::inspector::FlagInspector::explain_flags(&command);
            for (flag, exp) in flags {
                println!("  {:12} -> {}", flag, exp);
            }
        }
        Some(Commands::Pty) => {
            println!("Testing Native Zig VT100 Parser & PTY Integration...");
            if let Some(parser) = shesh_terminal::ZigVt100Parser::new(80, 24) {
                parser.feed(b"Echo from PTY\nLine 2\n");
                println!(
                    "Zig VT100 Parser processed {} lines successfully.",
                    parser.lines_processed()
                );
            }
        }
    }

    Ok(())
}

fn run_interactive_tui() -> anyhow::Result<()> {
    use std::io;

    use crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
            MouseEventKind,
        },
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = shesh_tui::App::new_cli();

    while app.running {
        terminal.draw(|f| shesh_tui::render_ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Mouse(mouse_event) => {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                        app.handle_click(mouse_event.column, mouse_event.row);
                    }
                }
                Event::Key(key) => match key.code {
                    KeyCode::F(10) | KeyCode::Char('q')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.running = false;
                    }
                    KeyCode::Char('k')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.tile_grid.launcher_open = !app.tile_grid.launcher_open;
                        app.push_log("[LAUNCHER] Toggled Quick Launcher Overlay (Ctrl+K)");
                    }
                    KeyCode::Char('d')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::CodeEditor);
                        app.push_log("[TILE] Split tile horizontally -> Added Code Editor Block");
                    }
                    KeyCode::Char('e')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::MarkdownReader);
                        app.push_log("[TILE] Split tile vertically -> Added Markdown Reader Block");
                    }
                    KeyCode::Char('w')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.tile_grid.close_active();
                        app.push_log("[TILE] Closed active tile block");
                    }
                    KeyCode::Char('1') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::PtyTerminal);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('2') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::WaveAi);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('3') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::CodeEditor);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('4') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::MarkdownReader);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('5') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::AiFileDiff);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('6') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::ProcessViewer);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('7') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::SysInfoGauges);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('8') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::CsvViewer);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::Char('9') if app.tile_grid.launcher_open => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::WaveConfig);
                        app.tile_grid.launcher_open = false;
                    }
                    KeyCode::F(1) => app.tile_grid.split_tile(shesh_tui::block::BlockKind::WaveAi),
                    KeyCode::F(2) => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::CodeEditor)
                    }
                    KeyCode::F(3) => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::MarkdownReader)
                    }
                    KeyCode::F(4) => {
                        app.tile_grid.split_tile(shesh_tui::block::BlockKind::AiFileDiff)
                    }
                    KeyCode::Tab => {
                        app.tile_grid.cycle_focus();
                    }
                    KeyCode::Char('c')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.running = false;
                    }
                    KeyCode::Char(c) => {
                        app.push_input_char(c);
                    }
                    KeyCode::Backspace => {
                        app.pop_input_char();
                    }
                    KeyCode::Enter => {
                        if let Some(submitted) = app.submit_input() {
                            if submitted == "/exit" || submitted == "/quit" {
                                app.running = false;
                            } else if submitted == "/clear" {
                                app.history.clear();
                            } else if submitted == "/vault" {
                                app.set_tool_window(shesh_tui::app::ActiveToolWindow::CommandVault);
                            } else if let Some(cmd) = submitted.strip_prefix("/explain ") {
                                let flags =
                                    shesh_vault::inspector::FlagInspector::explain_flags(cmd);
                                app.push_log(&format!("[EXPLAIN] Flags for: {}", cmd));
                                for (f, desc) in flags {
                                    app.push_log(&format!("  {:12} -> {}", f, desc));
                                }
                            } else {
                                app.push_log(&format!(
                                    "[PROCESSED] Input submitted: {}",
                                    submitted
                                ));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.mode = shesh_tui::app::AppMode::NormalPrompt;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    println!("SheshAOS interactive session closed cleanly.");
    Ok(())
}
