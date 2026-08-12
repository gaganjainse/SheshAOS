use crate::message::{RpcRequest, RpcResponse};
use serde_json::json;
use shesh_waveobj::store::WaveStore;
use shesh_wps::broker::Broker;
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
        RpcResponse { jsonrpc: "2.0".into(), result: Some(json!("pong")), error: None, id: req.id }
    }

    /// Handle a single Unix socket connection.
    /// Reads JSON-RPC 2.0 frames and writes responses.
    pub async fn handle_connection(&self, stream: UnixStream) -> Result<(), std::io::Error> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::time::{timeout, Duration};

        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            // Read with timeout to avoid blocking forever on idle connections
            let read_result = timeout(Duration::from_secs(5), reader.read_line(&mut line)).await;
            let bytes_read = match read_result {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(e),
                Err(_) => break, // timeout -> connection idle, close gracefully
            };

            if bytes_read == 0 {
                // Connection closed
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let req: RpcRequest = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let resp = RpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(crate::message::RpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                        }),
                        id: None,
                    };
                    let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                    let _ = writer.write_all(resp_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    continue;
                }
            };

            let resp = self.process_request(req).await;
            let resp_json = serde_json::to_string(&resp).unwrap_or_default();
            writer.write_all(resp_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::UnixStream;

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

    #[tokio::test]
    async fn test_handler_new_constructs() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let _handler = RpcHandler::new(broker, store);
    }

    #[tokio::test]
    async fn test_handler_broker_accessor_returns_same_arc() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker.clone(), store);
        let broker_ref = handler.broker();
        assert!(Arc::ptr_eq(broker_ref, &broker));
    }

    #[tokio::test]
    async fn test_handler_store_accessor_returns_same_arc() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store.clone());
        let store_ref = handler.store();
        assert!(Arc::ptr_eq(store_ref, &store));
    }

    #[tokio::test]
    async fn test_process_request_with_none_id() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);
        let req =
            RpcRequest { jsonrpc: "2.0".into(), method: "notify".into(), params: None, id: None };
        let resp = handler.process_request(req).await;
        assert!(resp.id.is_none());
        assert_eq!(resp.result.unwrap(), json!("pong"));
    }

    #[tokio::test]
    async fn test_process_request_preserves_id() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            method: "echo".into(),
            params: None,
            id: Some("42".into()),
        };
        let resp = handler.process_request(req).await;
        assert_eq!(resp.id, Some("42".into()));
    }

    #[tokio::test]
    async fn test_process_request_different_methods() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);

        for method in &["ping", "health", "version", "status"] {
            let req = RpcRequest {
                jsonrpc: "2.0".into(),
                method: (*method).into(),
                params: None,
                id: Some("1".into()),
            };
            let resp = handler.process_request(req).await;
            assert_eq!(resp.jsonrpc, "2.0");
            assert_eq!(resp.result.unwrap(), json!("pong"));
        }
    }

    #[tokio::test]
    async fn test_handle_connection_success() {
        use tokio::io::AsyncWriteExt;
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);

        let (stream1, mut stream2) = UnixStream::pair().unwrap();
        // Write a valid JSON-RPC request and half-close to signal EOF
        let _ =
            stream2.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":\"1\"}\n").await;
        let _ = stream2.shutdown().await;
        let result = handler.handle_connection(stream1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_does_not_panic_on_drop() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = RpcHandler::new(broker, store);

        let (stream1, mut stream2) = UnixStream::pair().unwrap();
        // Half-close to signal EOF without dropping
        let _ = stream2.shutdown().await;
        let result = handler.handle_connection(stream1).await;
        assert!(result.is_ok());
    }
}
