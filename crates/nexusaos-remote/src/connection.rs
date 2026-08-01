use crate::ssh_client::ClientHandler;
use russh::client::{Config, Handle};
use std::sync::Arc;

use nexusaos_wps::broker::Broker;
use nexusaos_wps::events::{WaveEvent, EVENT_CONN_CHANGE};
use serde_json::json;

pub struct ConnectionManager {
    broker: Arc<Broker>,
    config: Arc<Config>,
}

impl ConnectionManager {
    pub fn new(broker: Arc<Broker>) -> Self {
        Self {
            broker,
            config: Arc::new(Config::default()),
        }
    }
    
    pub async fn connect(&self, user: &str, host: &str, port: u16) -> Result<Handle<ClientHandler>, russh::Error> {
        let mut handle = russh::client::connect(self.config.clone(), (host, port), ClientHandler {}).await?;
        
        let event = WaveEvent::global(
            EVENT_CONN_CHANGE,
            json!({
                "connection_id": format!("{}:{}", host, port),
                "status": "connecting"
            })
        );
        self.broker.publish(event);
        
        let _ = handle.authenticate_password(user, "test").await;
        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conn_manager() {
        let broker = Broker::new(10);
        let _ = ConnectionManager::new(broker);
    }
}
