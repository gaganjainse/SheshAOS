//! `nexusaos-tui` — Interactive TUI, SSE Token Streamer, and Code Diff Visualizer.

pub mod app;
pub mod block;
pub mod diff;
pub mod modal;
pub mod patch;
pub mod stream;
pub mod ui;

pub use app::App;
pub use diff::DiffViewer;
pub use modal::ApprovalModal;
pub use patch::PatchEngine;
pub use stream::TokenStreamer;
pub use ui::render_ui;
pub mod layout;
pub mod input;
