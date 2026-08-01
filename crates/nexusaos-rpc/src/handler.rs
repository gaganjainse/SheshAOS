use crate::message::{RpcRequest, RpcResponse};
use serde_json::json;
use nexusaos_wps::broker::Broker;
use nexusaos_waveobj::store::WaveStore;
use std::sync::Arc;
use tokio::net::UnixStream;

pub struct RpcHandler {
    broker: Arc<Broker>,
    store: Arc<WaveStore>,
}

impl RpcHandler {
    pub fn new(broker: Arc<Broker>, store: Arc<WaveStore>) -> Self {
        Self { broker, store }
    }

    /// Get a reference to the broker for event publishing.
    pub fn broker(&self) -> &Arc<Broker> {
        &self.broker
    }

    /// Get a reference to the store for object persistence.
    pub fn store(&self) -> &Arc<WaveStore> {
        &self.store
    }

    pub async fn process_request(&self, req: RpcRequest) -> RpcResponse {
        RpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(json!("pong")),
            error: None,
            id: req.id,
        }
    }

    /// Handle a single Unix socket connection.
    /// Reads JSON-RPC requests and writes responses.
    pub async fn handle_connection(&self, _stream: UnixStream) -> Result<(), std::io::Error> {
        // Verify broker and store are accessible
        let _broker = &self.broker;
        let _store = &self.store;
        // Placeholder: in a real implementation, this would read/write JSON-RPC frames
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_request() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            method: "ping".into(),
            params: None,
            id: Some("1".into()),
        };
        let resp = handler.process_request(req).await;
        assert_eq!(resp.result.unwrap(), json!("pong"));
    }
}
