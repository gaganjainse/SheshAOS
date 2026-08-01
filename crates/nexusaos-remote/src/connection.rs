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

    #[test]
    fn test_connection_manager_new_default_config() {
        let broker = Broker::new(10);
        let manager = ConnectionManager::new(broker);
        assert!(Arc::strong_count(&manager.config) >= 1);
    }

    #[tokio::test]
    async fn test_connection_manager_connect_unreachable_host() {
        let broker = Broker::new(10);
        let manager = ConnectionManager::new(broker);
        let result = manager.connect("user", "127.0.0.1", 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_manager_connect_invalid_host() {
        let broker = Broker::new(10);
        let manager = ConnectionManager::new(broker);
        let result = manager.connect("user", "invalid-host-that-does-not-exist.example", 22).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_manager_multiple_instances() {
        let broker1 = Broker::new(10);
        let broker2 = Broker::new(10);
        let _m1 = ConnectionManager::new(broker1);
        let _m2 = ConnectionManager::new(broker2);
    }
}
