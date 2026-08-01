# NexusAOS Terminal — Developer Handover Document

**Date**: 2026-08-01
**Context**: Handover for transition to VS Code.

## 1. Project Overview
NexusAOS is an ambitious, native-Rust terminal emulator and multiplexer. It uniquely combines GPU-accelerated rendering (`iced` / `wgpu`), a built-in AI engine (`nexusaos-ai`), and native SSH multiplexing (`nexusaos-remote`), orchestrated by a pub/sub event broker (`nexusaos-wps`) and an SQLite object store (`nexusaos-waveobj`).

**Current Status**: Pre-Alpha. The individual crates exist and compile, but the GUI (`nexusaos-gui`) is currently rendering text naively (character-by-character) and lacks proper ANSI escape sequence parsing.

## 2. Recent Work Completed
*   **UI Modernization**: Upgraded the `nexusaos-gui` code to strictly follow `iced` 0.14 layout paradigms (fixing `align_x` / `align_y` alignment compilation errors and `Space` widget initialization).
*   **AI Scaffolding**: Built the initial AI chat UI in `view.rs` and wired it to `app.rs`.
*   **Concurrency Fixes**: Resolved a critical deadlock in `nexusaos-ai/src/session.rs` where an async `Mutex` over the history array was being held open while awaiting the long-running HTTP stream.
*   **Deep Architectural Audits**: Cloned and audited 7 industry-leading terminals (Warp, Wave, Ghostty, Alacritty, Tabby, Kitty, WezTerm) using 7 dedicated subagents to extract optimization patterns. 

*Note: The detailed audit reports are available in the `.gemini/antigravity/brain/...` artifact directory as `audit_report.md` and `audit_report_v2.md`.*

## 3. The Blueprint: Architecture to Implement
Based on our audits, here are the architectural patterns you must implement to bring NexusAOS to production-grade performance:

1.  **Zero-Allocation ANSI Parsing (Priority 1)**
    *   *Problem*: `terminal.rs` manually matches characters (`match ch`), causing real CLI apps (like `vim` or `ls --color`) to print literal escape sequences (e.g. `[31m`) instead of formatting.
    *   *Solution*: Delete the manual parse loop. Import the `vte` crate (used by Alacritty) and implement the `vte::Perform` trait on the `TerminalState` struct to handle state mutations in-place.
2.  **Batched & Cached Rendering (Priority 2)**
    *   *Problem*: `iced` Canvas is currently drawing every single character individually, resulting in thousands of draw calls and ~50ms latency.
    *   *Solution*: Implement line-based caching. Track `dirty` bits per row, and only regenerate the `iced::widget::Text` layout for lines that actually changed. Eventually, you may need a custom `wgpu` widget that uses a single texture atlas and instanced `glDrawElements` calls (like WezTerm/Alacritty).
3.  **PTY Backpressure & Locking (Priority 3)**
    *   *Problem*: Polling the PTY reader blocks the main thread.
    *   *Solution*: Use a dedicated `tokio::task::spawn_blocking` thread for `portable-pty`. Read in 1MB chunks, but force the thread to yield its lock every 64KB so the `iced` GUI renderer is never starved (pattern taken from Alacritty).

## 4. Immediate Next Steps in VS Code
When you open this workspace in VS Code, here is exactly where you should start:

1.  **Open `crates/nexusaos-gui/src/terminal.rs`**: 
    *   Find the `process_output` function. 
    *   Strip out the manual `match ch` logic.
    *   Initialize a `vte::Parser` and wire it up to mutate the `Line` cells.
2.  **Open `crates/nexusaos-gui/src/view.rs`**:
    *   Look at `render_terminal`.
    *   Wrap the text rendering in a cached primitive or group it by contiguous blocks of styles (spans) to reduce draw calls.
3.  **Open `crates/nexusaos-ai/src/session.rs`**:
    *   The `send_message` function's deadlock is fixed, but you need to wire the streaming output directly to the `iced` application's `Subscription` model so the UI updates token-by-token.

Good luck! The architecture has an incredibly high ceiling once the rendering and parsing bottlenecks are cleared.
