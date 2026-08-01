use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("block not found: {0}")]
    BlockNotFound(String),
    #[error("controller already exists for block: {0}")]
    AlreadyExists(String),
    #[error("controller not running for block: {0}")]
    NotRunning(String),
    #[error("shell error: {0}")]
    Shell(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Runtime status of a block controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerStatus {
    pub block_id: String,
    pub status: String,  // "init", "running", "done", "error"
    pub conn_name: String,
    pub exit_code: Option<i32>,
}

/// Input sent to a block controller (keyboard input or resize)
#[derive(Debug, Clone)]
pub enum BlockInput {
    /// Raw terminal input bytes (keystrokes)
    Data(Vec<u8>),
    /// Terminal resize event
    Resize { rows: u16, cols: u16 },
    /// Signal (e.g., SIGINT)
    Signal(i32),
}

/// The Controller trait — implemented by ShellController (and future DurableShellController, etc.)
#[async_trait::async_trait]
pub trait Controller: Send + Sync {
    /// Start the controller (spawn shell process, etc.)
    async fn start(&self) -> Result<(), ControllerError>;
    /// Stop the controller gracefully
    async fn stop(&self, graceful: bool) -> Result<(), ControllerError>;
    /// Get current runtime status
    fn runtime_status(&self) -> ControllerStatus;
    /// Get connection name
    fn conn_name(&self) -> &str;
    /// Send input to the running process
    async fn send_input(&self, input: BlockInput) -> Result<(), ControllerError>;
}

/// Global registry of active controllers, keyed by block_id.
pub struct ControllerRegistry {
    controllers: RwLock<HashMap<String, Arc<dyn Controller>>>,
}

impl ControllerRegistry {
    pub fn new() -> Self {
        Self {
            controllers: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, block_id: &str, controller: Arc<dyn Controller>) -> Result<(), ControllerError> {
        let mut controllers = self.controllers.write().unwrap_or_else(|e| e.into_inner());
        if controllers.contains_key(block_id) {
            return Err(ControllerError::AlreadyExists(block_id.to_string()));
        }
        controllers.insert(block_id.to_string(), controller);
        Ok(())
    }

    pub fn get(&self, block_id: &str) -> Option<Arc<dyn Controller>> {
        let controllers = self.controllers.read().unwrap_or_else(|e| e.into_inner());
        controllers.get(block_id).cloned()
    }

    pub fn remove(&self, block_id: &str) -> Option<Arc<dyn Controller>> {
        let mut controllers = self.controllers.write().unwrap_or_else(|e| e.into_inner());
        controllers.remove(block_id)
    }

    pub async fn send_input(&self, block_id: &str, input: BlockInput) -> Result<(), ControllerError> {
        let controller = self.get(block_id).ok_or_else(|| ControllerError::BlockNotFound(block_id.to_string()))?;
        controller.send_input(input).await
    }

    pub fn list(&self) -> Vec<ControllerStatus> {
        let controllers = self.controllers.read().unwrap_or_else(|e| e.into_inner());
        controllers.values().map(|c| c.runtime_status()).collect()
    }

    pub fn stop_all(&self) {
        let controllers = {
            let controllers = self.controllers.read().unwrap_or_else(|e| e.into_inner());
            controllers.values().cloned().collect::<Vec<_>>()
        };
        for controller in controllers {
            tokio::spawn(async move {
                let _ = controller.stop(true).await;
            });
        }
    }
}

impl Default for ControllerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockController {
        block_id: String,
        started: AtomicBool,
    }

    #[async_trait::async_trait]
    impl Controller for MockController {
        async fn start(&self) -> Result<(), ControllerError> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }
        async fn stop(&self, _graceful: bool) -> Result<(), ControllerError> {
            self.started.store(false, Ordering::SeqCst);
            Ok(())
        }
        fn runtime_status(&self) -> ControllerStatus {
            ControllerStatus {
                block_id: self.block_id.clone(),
                status: if self.started.load(Ordering::SeqCst) { "running".to_string() } else { "init".to_string() },
                conn_name: "mock".to_string(),
                exit_code: None,
            }
        }
        fn conn_name(&self) -> &str {
            "mock"
        }
        async fn send_input(&self, _input: BlockInput) -> Result<(), ControllerError> {
            Ok(())
        }
    }

    #[test]
    fn test_registry() {
        let registry = ControllerRegistry::new();
        let controller = Arc::new(MockController {
            block_id: "blk1".to_string(),
            started: AtomicBool::new(false),
        });

        assert!(registry.register("blk1", controller.clone()).is_ok());
        assert!(registry.register("blk1", controller.clone()).is_err());

        assert!(registry.get("blk1").is_some());
        assert!(registry.get("blk2").is_none());

        let statuses = registry.list();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].block_id, "blk1");

        assert!(registry.remove("blk1").is_some());
        assert!(registry.remove("blk1").is_none());
    }
}
