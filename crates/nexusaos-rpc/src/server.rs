use std::sync::Arc;
use tokio::net::UnixListener;
use crate::handler::RpcHandler;
use std::path::PathBuf;

pub struct RpcServer {
    handler: Arc<RpcHandler>,
    socket_path: PathBuf,
}

impl RpcServer {
    pub fn new(handler: Arc<RpcHandler>, socket_path: PathBuf) -> Self {
        Self { handler, socket_path }
    }

    pub async fn run(&self) -> Result<(), std::io::Error> {
        tokio::fs::remove_file(&self.socket_path).await.ok();
        let listener = UnixListener::bind(&self.socket_path)?;
        
        loop {
            let (stream, _addr) = listener.accept().await?;
            let handler = self.handler.clone();
            // Spawn a task to handle each connection
            tokio::spawn(async move {
                if let Err(e) = handler.handle_connection(stream).await {
                    eprintln!("[RPC] Connection error: {}", e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use nexusaos_wps::broker::Broker;
    use nexusaos_waveobj::store::WaveStore;

    #[tokio::test]
    async fn test_server_new() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = Arc::new(RpcHandler::new(broker, store));
        let server = RpcServer::new(handler, PathBuf::from("/tmp/test_server.sock"));
        assert_eq!(server.socket_path.to_str().unwrap(), "/tmp/test_server.sock");
    }

    #[tokio::test]
    async fn test_server_new_with_relative_path() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = Arc::new(RpcHandler::new(broker, store));
        let server = RpcServer::new(handler, PathBuf::from("relative/path.sock"));
        assert_eq!(server.socket_path, PathBuf::from("relative/path.sock"));
    }

    #[tokio::test]
    async fn test_server_new_with_empty_path() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = Arc::new(RpcHandler::new(broker, store));
        let server = RpcServer::new(handler, PathBuf::from(""));
        assert_eq!(server.socket_path, PathBuf::from(""));
    }

    #[tokio::test]
    async fn test_server_socket_path_stored_correctly() {
        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = Arc::new(RpcHandler::new(broker, store));
        let path = PathBuf::from("/var/run/nexusaos/rpc.sock");
        let server = RpcServer::new(handler, path.clone());
        assert_eq!(server.socket_path, path);
    }

    #[tokio::test]
    async fn test_server_run_creates_listener() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let broker = Broker::new(10);
        let store = Arc::new(WaveStore::open_in_memory().unwrap());
        let handler = Arc::new(RpcHandler::new(broker, store));
        let server = RpcServer::new(handler, socket_path.clone());

        let server_handle = tokio::spawn(async move {
            let result = server.run().await;
            result
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        server_handle.abort();
        let _ = tokio::fs::remove_file(&socket_path).await;
    }
}
