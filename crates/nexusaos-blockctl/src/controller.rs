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

    #[test]
    fn test_registry_default_constructor() {
        let registry = ControllerRegistry::default();
        assert!(registry.list().is_empty());
        assert!(registry.get("any").is_none());
    }

    #[test]
    fn test_registry_multiple_controllers() {
        let registry = ControllerRegistry::new();
        let c1 = Arc::new(MockController {
            block_id: "blk1".to_string(),
            started: AtomicBool::new(false),
        });
        let c2 = Arc::new(MockController {
            block_id: "blk2".to_string(),
            started: AtomicBool::new(false),
        });

        assert!(registry.register("blk1", c1).is_ok());
        assert!(registry.register("blk2", c2).is_ok());

        let statuses = registry.list();
        assert_eq!(statuses.len(), 2);
        assert!(statuses.iter().any(|s| s.block_id == "blk1"));
        assert!(statuses.iter().any(|s| s.block_id == "blk2"));
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let registry = ControllerRegistry::new();
        assert!(registry.remove("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list_empty() {
        let registry = ControllerRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[tokio::test]
    async fn test_registry_send_input_not_found() {
        let registry = ControllerRegistry::new();
        let result = registry.send_input("nonexistent", BlockInput::Data(b"hello".to_vec())).await;
        assert!(result.is_err());
        match result {
            Err(ControllerError::BlockNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("expected BlockNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_registry_send_input_found() {
        use std::sync::atomic::AtomicBool;
        let registry = ControllerRegistry::new();
        let controller = Arc::new(MockController {
            block_id: "blk1".to_string(),
            started: AtomicBool::new(false),
        });
        registry.register("blk1", controller).unwrap();

        let result = registry.send_input("blk1", BlockInput::Data(b"test".to_vec())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_stop_all() {
        use std::sync::atomic::AtomicBool;
        let registry = ControllerRegistry::new();
        let c1 = Arc::new(MockController {
            block_id: "blk1".to_string(),
            started: AtomicBool::new(true),
        });
        let c2 = Arc::new(MockController {
            block_id: "blk2".to_string(),
            started: AtomicBool::new(true),
        });
        registry.register("blk1", c1.clone()).unwrap();
        registry.register("blk2", c2.clone()).unwrap();

        registry.stop_all();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let statuses = registry.list();
        assert_eq!(statuses.len(), 2);
        assert!(statuses.iter().all(|s| s.status == "init"));
    }

    #[test]
    fn test_controller_status_serialization() {
        let status = ControllerStatus {
            block_id: "blk1".to_string(),
            status: "running".to_string(),
            conn_name: "local".to_string(),
            exit_code: Some(0),
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: ControllerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.block_id, "blk1");
        assert_eq!(parsed.status, "running");
        assert_eq!(parsed.conn_name, "local");
        assert_eq!(parsed.exit_code, Some(0));
    }

    #[test]
    fn test_controller_status_deserialization() {
        let json = r#"{"block_id":"blk1","status":"done","conn_name":"remote","exit_code":1}"#;
        let status: ControllerStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.block_id, "blk1");
        assert_eq!(status.status, "done");
        assert_eq!(status.conn_name, "remote");
        assert_eq!(status.exit_code, Some(1));
    }

    #[test]
    fn test_block_input_data_variant() {
        let input = BlockInput::Data(b"hello".to_vec());
        match input {
            BlockInput::Data(d) => assert_eq!(d, b"hello"),
            _ => panic!("expected Data variant"),
        }
    }

    #[test]
    fn test_block_input_resize_variant() {
        let input = BlockInput::Resize { rows: 24, cols: 80 };
        match input {
            BlockInput::Resize { rows, cols } => {
                assert_eq!(rows, 24);
                assert_eq!(cols, 80);
            }
            _ => panic!("expected Resize variant"),
        }
    }

    #[test]
    fn test_block_input_signal_variant() {
        let input = BlockInput::Signal(2);
        match input {
            BlockInput::Signal(sig) => assert_eq!(sig, 2),
            _ => panic!("expected Signal variant"),
        }
    }

    #[test]
    fn test_controller_error_display() {
        let err = ControllerError::BlockNotFound("blk1".to_string());
        assert_eq!(err.to_string(), "block not found: blk1");

        let err = ControllerError::AlreadyExists("blk1".to_string());
        assert_eq!(err.to_string(), "controller already exists for block: blk1");

        let err = ControllerError::NotRunning("blk1".to_string());
        assert_eq!(err.to_string(), "controller not running for block: blk1");

        let err = ControllerError::Shell("pty error".to_string());
        assert_eq!(err.to_string(), "shell error: pty error");
    }

    #[test]
    fn test_controller_error_io_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let controller_err: ControllerError = io_err.into();
        match controller_err {
            ControllerError::Io(_) => {}
            _ => panic!("expected Io variant"),
        }
    }

    #[test]
    fn test_registry_get_returns_cloned_arc() {
        use std::sync::atomic::AtomicBool;
        let registry = ControllerRegistry::new();
        let controller = Arc::new(MockController {
            block_id: "blk1".to_string(),
            started: AtomicBool::new(false),
        });
        registry.register("blk1", controller.clone()).unwrap();

        let retrieved = registry.get("blk1").unwrap();
        assert_eq!(retrieved.runtime_status().block_id, "blk1");
        assert_eq!(retrieved.conn_name(), "mock");
    }
}
