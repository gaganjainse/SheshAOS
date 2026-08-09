//! TUI Application state machine for the JetBrains IDE layout.

use crate::block::TileGrid;
use sheshaaos_wps::broker::Broker;
use sheshaaos_wps::events::SubscriptionRequest;
use sheshaaos_wps::events::{EVENT_CONFIG, EVENT_BLOCK_UPDATE};
use sheshaaos_waveobj::store::WaveStore;
use sheshaaos_blockctl::controller::ControllerRegistry;
use std::sync::Arc;
use futures::StreamExt;
use crossterm::event::{Event, EventStream, KeyCode};
use ratatui::{backend::Backend, Terminal};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveToolWindow {
    ProjectTree,
    Editor,
    Terminal,
    AiAssistant,
    CommandVault,
    GitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    NormalPrompt,
    StreamingResponse,
    ApprovalModal { action: String, details: String },
}

pub struct App {
    pub active_tool_window: ActiveToolWindow,
    pub mode: AppMode,
    pub tile_grid: TileGrid,
    pub input_buffer: String,
    pub history: Vec<String>,
    pub pty_output: Vec<String>,
    pub project_tree: Vec<String>,
    pub git_branch: String,
    pub run_config: String,
    pub current_file: String,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub cpu_usage_pct: f32,
    pub task_count: usize,
    pub active_model: String,
    pub status_message: String,
    pub running: bool,
    
    pub broker: Arc<Broker>,
    pub store: Arc<WaveStore>,
    pub registry: Arc<ControllerRegistry>,
}

impl App {
    pub fn new(broker: Arc<Broker>, store: Arc<WaveStore>, registry: Arc<ControllerRegistry>) -> Self {
        Self {
            active_tool_window: ActiveToolWindow::AiAssistant,
            mode: AppMode::NormalPrompt,
            tile_grid: TileGrid::new(),
            input_buffer: String::new(),
            history: vec![
                "=== JETBRAINS NEXUSAOS AI IDE ===".to_string(),
                "Loaded Project: /home/gagan/Workspace/SheshaAOS".to_string(),
                "Active Models: Gemma-12B (Planner) | Qwen-30B (Coder)".to_string(),
                "Press Alt+1: Project Tree | Alt+F12: Terminal | Ctrl+R: Run | F10: Exit"
                    .to_string(),
            ],
            pty_output: vec![
                "bash - 80x24 (master)".to_string(),
                "$ cargo +nightly test --workspace".to_string(),
                "  89 passed; 0 failed".to_string(),
            ],
            project_tree: vec![
                "▼ SheshaAOS [Workspace]".to_string(),
                "  ▶ bin/sheshaaos-cli/".to_string(),
                "  ▼ crates/".to_string(),
                "    ▶ sheshaaos-kernel/".to_string(),
                "    ▶ sheshaaos-vault/".to_string(),
                "    ▶ sheshaaos-tui/".to_string(),
                "    ▶ sheshaaos-terminal/".to_string(),
                "  ▶ configs/default.toml".to_string(),
                "  📄 Cargo.toml".to_string(),
                "  📄 README.md".to_string(),
            ],
            git_branch: "main*".to_string(),
            run_config: "sheshaaos [Debug]".to_string(),
            current_file: "crates/sheshaaos-kernel/src/lib.rs".to_string(),
            ram_used_mb: 7420,
            ram_total_mb: 16000,
            vram_used_mb: 1536,
            vram_total_mb: 6144,
            cpu_usage_pct: 12.4,
            task_count: 4,
            active_model: "gemma-4-12b / qwen3-30b".to_string(),
            status_message: "Ready".to_string(),
            running: true,
            broker,
            store,
            registry,
        }
    }

    /// Create a minimal App for CLI interactive mode (without full broker/store/registry).
    pub fn new_cli() -> Self {
        use sheshaaos_wps::broker::Broker;
        use sheshaaos_waveobj::store::WaveStore;
        use sheshaaos_blockctl::controller::ControllerRegistry;
        use std::sync::Arc;
        
        let broker = Broker::new(100);
        let store = Arc::new(WaveStore::open_in_memory().expect("Failed to create in-memory store"));
        let registry = Arc::new(ControllerRegistry::new());
        
        Self::new(broker, store, registry)
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
        let route_id = Uuid::now_v7().to_string();
        
        self.broker.subscribe(&route_id, SubscriptionRequest {
            topic: EVENT_CONFIG.to_string(),
            scopes: vec![],
        });
        self.broker.subscribe(&route_id, SubscriptionRequest {
            topic: EVENT_BLOCK_UPDATE.to_string(),
            scopes: vec![],
        });
        
        let mut broker_rx = self.broker.receiver();
        let mut events = EventStream::new();
        
        terminal.draw(|f| crate::ui::render_ui(f, self))?;
        
        while self.running {
            tokio::select! {
                maybe_event = events.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_event {
                            // Basic exit for demo
                            if key.code == KeyCode::F(10) {
                                self.running = false;
                                break;
                            }
                            
                            // Route input
                            let active_block_id = match self.active_tool_window {
                                ActiveToolWindow::Terminal => "terminal_block",
                                _ => "default_block",
                            };
                            
                            let _ = crate::input::handle_key_event(key, active_block_id, self.registry.clone()).await;
                            
                            // In real app we might only draw if handle_key_event resulted in local state change,
                            // but for simplicity redraw
                            terminal.draw(|f| crate::ui::render_ui(f, self))?;
                        }
                }
                
                Ok((_route, wave_event)) = broker_rx.recv() => {
                    match wave_event.topic.as_str() {
                        EVENT_CONFIG => {
                            terminal.draw(|f| crate::ui::render_ui(f, self))?;
                        }
                        EVENT_BLOCK_UPDATE => {
                            // Update specific block's buffer if needed
                            // Then redraw
                            terminal.draw(|f| crate::ui::render_ui(f, self))?;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        self.broker.unsubscribe_all(&route_id);
        Ok(())
    }

    pub fn set_tool_window(&mut self, window: ActiveToolWindow) {
        self.active_tool_window = window;
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input_buffer.trim().is_empty() {
            return None;
        }

        let input = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        Some(input)
    }

    pub fn push_log(&mut self, log: &str) {
        self.history.push(log.to_string());
    }

    // --- Methods needed by CLI ---
    
    pub fn handle_click(&mut self, col: u16, row: u16) {
        // Simple click handling - could be expanded to focus blocks
        // For now, just log it
        self.push_log(&format!("[CLICK] col={}, row={}", col, row));
    }

    pub fn push_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn pop_input_char(&mut self) {
        self.input_buffer.pop();
    }
}
