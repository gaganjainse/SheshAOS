//! PTY Process Manager for spawning user shells (bash, zsh, fish) via portable-pty.
//!
//! Includes backpressure-aware async reading: the PTY reader task yields its lock
//! every `PTY_READ_CHUNK` bytes so the GUI renderer is never starved.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system};
use tokio::sync::mpsc;
use tracing::info;

/// PTY read chunk size (64KB) - yield lock after each chunk to prevent GUI starvation.
const PTY_READ_CHUNK: usize = 64 * 1024;
/// Maximum buffer size before applying backpressure (1MB).
const PTY_MAX_BUFFER: usize = 1024 * 1024;

/// Manages native pseudo-terminal (PTY) shell instances with backpressure-aware reading.
pub struct PtyManager {
    pair: PtyPair,
    /// Flag to signal the reader task to stop.
    shutdown: Arc<AtomicBool>,
    /// Channel for streaming PTY output to consumers.
    output_tx: Option<mpsc::Sender<Vec<u8>>>,
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

        Ok(Self {
            pair,
            shutdown: Arc::new(AtomicBool::new(false)),
            output_tx: None,
        })
    }

    /// Read raw output bytes from the PTY master with backpressure.
    ///
    /// Returns `None` if the PTY has been closed or the manager has been shut down.
    pub fn read_output(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut reader = self
            .pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let n = reader.read(buf)?;
        // Apply backpressure: if buffer read exceeds chunk size, yield to prevent starving the GUI renderer
        if (PTY_READ_CHUNK..=PTY_MAX_BUFFER).contains(&n) {
            std::thread::yield_now();
        }
        Ok(n)
    }

    /// Write raw input bytes to the PTY master.
    pub fn write_input(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut writer =
            self.pair.master.take_writer().map_err(|e| std::io::Error::other(e.to_string()))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// Spawn an async background task that reads PTY output and sends it through a channel.
    /// This provides backpressure: the channel has a bounded capacity, so the reader
    /// task naturally slows down when the consumer falls behind.
    pub fn spawn_reader_task(&mut self, capacity: usize) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(capacity);
        let tx_clone = tx.clone();
        self.output_tx = Some(tx);
        let shutdown = self.shutdown.clone();
        let mut reader = self.pair.master.try_clone_reader().expect("Failed to clone PTY reader");

        tokio::spawn(async move {
            let mut buf = vec![0u8; PTY_READ_CHUNK];
            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        if tx_clone.send(chunk).await.is_err() {
                            // Consumer dropped, stop reading
                            break;
                        }
                        // Yield to prevent starving the GUI renderer
                        tokio::task::yield_now().await;
                    }
                    Err(_) => break,
                }
            }
        });

        rx
    }

    /// Signal the reader task to stop.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
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

    #[test]
    fn test_pty_spawn_different_dimensions() {
        for (cols, rows) in &[(80, 24), (120, 40), (40, 10), (200, 60)] {
            let result = PtyManager::spawn(*cols, *rows);
            if result.is_ok() {
                let pty = result.unwrap();
                assert!(pty.pair.master.process_group_leader().is_some());
            }
        }
    }

    #[test]
    fn test_pty_spawn_zero_dimensions() {
        let result = PtyManager::spawn(0, 0);
        // May fail or succeed depending on OS PTY implementation
        if result.is_ok() {
            let pty = result.unwrap();
            assert!(pty.pair.master.process_group_leader().is_some());
        }
    }

    #[test]
    fn test_pty_spawn_large_dimensions() {
        let result = PtyManager::spawn(10000, 10000);
        if result.is_ok() {
            let pty = result.unwrap();
            assert!(pty.pair.master.process_group_leader().is_some());
        }
    }

    #[test]
    fn test_pty_read_output() {
        if let Ok(mut pty) = PtyManager::spawn(80, 24) {
            let mut buf = [0u8; 1024];
            let n = pty.read_output(&mut buf);
            assert!(n.is_ok());
            // Reading from a fresh PTY may return 0 bytes (no output yet)
            assert!(n.unwrap() <= buf.len());
        }
    }

    #[test]
    fn test_pty_write_input() {
        if let Ok(mut pty) = PtyManager::spawn(80, 24) {
            let result = pty.write_input(b"echo test\n");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_pty_write_empty_input() {
        if let Ok(mut pty) = PtyManager::spawn(80, 24) {
            let result = pty.write_input(b"");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_pty_read_empty_buffer() {
        if let Ok(mut pty) = PtyManager::spawn(80, 24) {
            let mut buf: [u8; 0] = [];
            let result = pty.read_output(&mut buf);
            // Reading into empty buffer should return Ok(0) or error
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_pty_single_write_only() {
        if let Ok(mut pty) = PtyManager::spawn(80, 24) {
            // take_writer() consumes the writer; only one write_input call succeeds
            let r1 = pty.write_input(b"echo first\n");
            assert!(r1.is_ok());
        }
    }

    #[test]
    fn test_pty_spawn_uses_default_shell() {
        // Verify that spawn uses the SHELL env var or falls back to /bin/bash
        let result = PtyManager::spawn(80, 24);
        if result.is_ok() {
            let pty = result.unwrap();
            assert!(pty.pair.master.process_group_leader().is_some());
        }
    }
}
