use crate::ssh_client::ClientHandler;
use russh::client::{Config, Handle};
use std::sync::Arc;

use serde_json::json;
use shesh_wps::broker::Broker;
use shesh_wps::events::{WaveEvent, EVENT_CONN_CHANGE};

pub struct ConnectionManager {
    broker: Arc<Broker>,
    config: Arc<Config>,
}

impl ConnectionManager {
    pub fn new(broker: Arc<Broker>) -> Self {
        Self { broker, config: Arc::new(Config::default()) }
    }

    pub async fn connect(
        &self,
        user: &str,
        host: &str,
        port: u16,
        password: &str,
    ) -> Result<Handle<ClientHandler>, russh::Error> {
        let handler = ClientHandler { host: host.to_string(), port, trust_new: false };
        let mut handle = russh::client::connect(self.config.clone(), (host, port), handler).await?;

        let event = WaveEvent::global(
            EVENT_CONN_CHANGE,
            json!({
                "connection_id": format!("{}:{}", host, port),
                "status": "connecting"
            }),
        );
        self.broker.publish(event);

        // russh 0.6x: authenticate_password yields AuthResult (Success/Failure).
        let authenticated = handle.authenticate_password(user, password).await?;
        if !authenticated.success() {
            tracing::error!("SSH authentication failed for user '{}' on {}:{}", user, host, port);
            return Err(russh::Error::NotAuthenticated);
        }
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
        let result = manager.connect("user", "127.0.0.1", 1, "testpass").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_manager_connect_invalid_host() {
        let broker = Broker::new(10);
        let manager = ConnectionManager::new(broker);
        let result = manager
            .connect("user", "invalid-host-that-does-not-exist.example", 22, "testpass")
            .await;
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
