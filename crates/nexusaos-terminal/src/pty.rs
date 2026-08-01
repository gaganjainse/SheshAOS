//! PTY Process Manager for spawning user shells (bash, zsh, fish) via portable-pty.

use std::io::{Read, Write};

use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system};
use tracing::info;

/// Manages native pseudo-terminal (PTY) shell instances.
pub struct PtyManager {
    pair: PtyPair,
}

impl PtyManager {
    /// Spawn a native user shell inside a PTY with specified terminal dimensions.
    pub fn spawn(cols: u16, rows: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let cmd = CommandBuilder::new(shell);
        let _child = pair.slave.spawn_command(cmd)?;
        info!("Spawned PTY shell instance");

        Ok(Self { pair })
    }

    /// Read raw output bytes from the PTY master.
    pub fn read_output(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut reader = self
            .pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        reader.read(buf)
    }

    /// Write raw input bytes to the PTY master.
    pub fn write_input(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut writer =
            self.pair.master.take_writer().map_err(|e| std::io::Error::other(e.to_string()))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_spawn() {
        if let Ok(pty) = PtyManager::spawn(80, 24) {
            assert!(pty.pair.master.process_group_leader().is_some());
        }
    }
}
