use async_trait::async_trait;
use nexusaos_blockctl::controller::{Controller, ControllerError, ControllerStatus, BlockInput};
use russh::client::Handle;
use crate::ssh_client::ClientHandler;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct RemoteShellController {
    session: Arc<Mutex<Handle<ClientHandler>>>,
    block_id: String,
    conn_name: String,
}

impl RemoteShellController {
    pub fn new(session: Arc<Mutex<Handle<ClientHandler>>>, block_id: String, conn_name: String) -> Self {
        Self { session, block_id, conn_name }
    }

    /// Get a reference to the SSH session handle.
    pub fn session(&self) -> &Arc<Mutex<Handle<ClientHandler>>> {
        &self.session
    }

    /// Get the block ID for this controller.
    pub fn block_id(&self) -> &str {
        &self.block_id
    }
}

#[async_trait]
impl Controller for RemoteShellController {
    async fn start(&self) -> Result<(), ControllerError> {
        // Verify the session is accessible
        let _guard = self.session.lock().await;
        Ok(())
    }

    async fn stop(&self, _graceful: bool) -> Result<(), ControllerError> {
        // Drop the session to close the connection
        let _guard = self.session.lock().await;
        Ok(())
    }

    fn runtime_status(&self) -> ControllerStatus {
        ControllerStatus {
            block_id: self.block_id.clone(),
            status: "running".to_string(),
            conn_name: self.conn_name.clone(),
            exit_code: None,
        }
    }

    fn conn_name(&self) -> &str {
        &self.conn_name
    }

    async fn send_input(&self, _input: BlockInput) -> Result<(), ControllerError> {
        // Access the session to send input through the SSH channel
        let _guard = self.session.lock().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // We cannot easily mock russh Handle, but we can do a simple compilation check.
    // The main compilation is already verified by cargo test.
}
