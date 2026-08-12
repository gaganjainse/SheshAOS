use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use shesh_blockctl::controller::{BlockInput, ControllerRegistry};
use std::sync::Arc;
use tracing::warn;

pub async fn handle_key_event(
    key: KeyEvent,
    active_block_id: &str,
    registry: Arc<ControllerRegistry>,
) -> bool {
    // If the user hits Ctrl+C we might want to exit the TUI, but here we just route it.
    // Well, wait. We should return a boolean indicating whether to quit the app or not?
    // The prompt just says "route the input". Let's return true if it routed, or maybe we just return an indicator.

    // For now we map some basic keys to their byte representations, or just use crossterm's representation.
    // In a real terminal emulator, we would translate KeyEvent into ANSI escape sequences.
    // For simplicity in this step, we do a basic translation.

    let mut bytes = Vec::new();
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+c = \x03
                let b = (c as u8).to_ascii_uppercase().saturating_sub(64);
                bytes.push(b);
            } else {
                let mut b = [0; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
            }
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Backspace => bytes.push(0x7F),
        KeyCode::Esc => bytes.push(0x1B),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        _ => return false,
    }

    if bytes.is_empty() {
        return false;
    }

    let result = registry.send_input(active_block_id, BlockInput::Data(bytes)).await;
    if let Err(e) = result {
        warn!("Failed to send input to block {}: {}", active_block_id, e);
        return false;
    }

    true
}
