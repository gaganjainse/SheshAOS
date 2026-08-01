use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::controller::{BlockInput, Controller, ControllerError, ControllerStatus};
use crate::filestore::BlockFileStore;

fn detect_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| {
        if std::path::Path::new("/bin/bash").exists() {
            "/bin/bash".to_string()
        } else {
            "/bin/sh".to_string()
        }
    })
}

pub struct ShellController {
    block_id: String,
    conn_name: String,
    status: Arc<Mutex<String>>,
    exit_code: Arc<Mutex<Option<i32>>>,
    input_tx: Arc<Mutex<Option<mpsc::Sender<BlockInput>>>>,
    file_store: Arc<BlockFileStore>,
    shell_path: String,
}

impl ShellController {
    pub fn new(
        block_id: String,
        file_store: Arc<BlockFileStore>,
        shell_path: Option<String>,
    ) -> Self {
        let shell = shell_path.unwrap_or_else(detect_shell);
        Self {
            block_id,
            conn_name: "local".to_string(),
            status: Arc::new(Mutex::new("init".to_string())),
            exit_code: Arc::new(Mutex::new(None)),
            input_tx: Arc::new(Mutex::new(None)),
            file_store,
            shell_path: shell,
        }
    }
}

#[async_trait::async_trait]
impl Controller for ShellController {
    async fn start(&self) -> Result<(), ControllerError> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| ControllerError::Shell(e.to_string()))?;

        let cmd = CommandBuilder::new(&self.shell_path);
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| ControllerError::Shell(e.to_string()))?;

        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = "running".to_string();

        let (tx, mut rx) = mpsc::channel::<BlockInput>(256);
        *self.input_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

        let mut reader = pair.master.try_clone_reader().map_err(|e| ControllerError::Shell(e.to_string()))?;
        let block_id = self.block_id.clone();
        let file_store = self.file_store.clone();
        
        // Reader task
        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        file_store.append(&block_id, &buf[..n]);
                    }
                    Err(_) => break, // Error
                }
            }
        });

        // Writer task
        let mut writer = pair.master.take_writer().map_err(|e| ControllerError::Shell(e.to_string()))?;
        let master = pair.master;
        
        tokio::task::spawn_blocking(move || {
            while let Some(input) = rx.blocking_recv() {
                match input {
                    BlockInput::Data(bytes) => {
                        let _ = writer.write_all(&bytes);
                        let _ = writer.flush();
                    }
                    BlockInput::Resize { rows, cols } => {
                        let _ = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                    BlockInput::Signal(_) => {}
                }
            }
        });

        // Waiter task
        let status = self.status.clone();
        let exit_code = self.exit_code.clone();
        
        tokio::task::spawn_blocking(move || {
            if let Ok(exit_status) = child.wait() {
                let code = exit_status.exit_code();
                *exit_code.lock().unwrap_or_else(|e| e.into_inner()) = Some(code as i32);
            }
            *status.lock().unwrap_or_else(|e| e.into_inner()) = "done".to_string();
        });

        Ok(())
    }

    async fn stop(&self, graceful: bool) -> Result<(), ControllerError> {
        let mut tx_guard = self.input_tx.lock().unwrap_or_else(|e| e.into_inner());
        *tx_guard = None; // Drop the sender to close the channel

        if !graceful {
            // Best effort without OS specific kill
        }

        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = "done".to_string();
        Ok(())
    }

    fn runtime_status(&self) -> ControllerStatus {
        ControllerStatus {
            block_id: self.block_id.clone(),
            status: self.status.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            conn_name: self.conn_name.clone(),
            exit_code: *self.exit_code.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }

    fn conn_name(&self) -> &str {
        &self.conn_name
    }

    async fn send_input(&self, input: BlockInput) -> Result<(), ControllerError> {
        let tx = {
            let guard = self.input_tx.lock().unwrap_or_else(|e| e.into_inner());
            guard.clone()
        };
        
        if let Some(tx) = tx {
            tx.send(input).await.map_err(|_| ControllerError::NotRunning(self.block_id.clone()))?;
            Ok(())
        } else {
            Err(ControllerError::NotRunning(self.block_id.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell() {
        let shell = detect_shell();
        assert!(!shell.is_empty());
    }

    #[test]
    fn test_controller_init() {
        let store = Arc::new(BlockFileStore::new());
        let controller = ShellController::new("blk1".to_string(), store, None);
        let status = controller.runtime_status();
        assert_eq!(status.status, "init");
        assert_eq!(status.conn_name, "local");
        assert_eq!(status.block_id, "blk1");
    }

    #[tokio::test]
    #[ignore]
    async fn test_integration_shell() {
        let store = Arc::new(BlockFileStore::new());
        let controller = ShellController::new("blk1".to_string(), store.clone(), None);
        controller.start().await.unwrap();

        controller.send_input(BlockInput::Data(b"echo hello\n".to_vec())).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let output = store.read_all("blk1").unwrap_or_default();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("hello"));

        controller.send_input(BlockInput::Data(b"exit\n".to_vec())).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let status = controller.runtime_status();
        assert_eq!(status.status, "done");
    }
}
