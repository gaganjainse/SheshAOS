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
}
