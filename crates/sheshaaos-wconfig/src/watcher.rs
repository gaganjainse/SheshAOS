use notify::{Watcher, RecursiveMode, Event as NotifyEvent, EventKind};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use sheshaaos_wps::broker::Broker;
use sheshaaos_wps::events::{WaveEvent, EVENT_CONFIG};

pub struct ConfigWatcher {
    config_path: PathBuf,
    broker: Arc<Broker>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf, broker: Arc<Broker>) -> Self {
        Self {
            config_path,
            broker,
        }
    }

    pub fn start(&self) {
        let (tx, mut rx) = mpsc::channel::<notify::Result<NotifyEvent>>(100);

        let config_path = self.config_path.clone();
        let broker = self.broker.clone();

        tokio::spawn(async move {
            let mut watcher = match notify::recommended_watcher(move |res| {
                let _ = tx.blocking_send(res);
            }) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to create watcher: {:?}", e);
                    return;
                }
            };

            let watch_dir = config_path.parent().unwrap_or(&config_path);
            if let Err(e) = watcher.watch(watch_dir, RecursiveMode::NonRecursive) {
                error!("Failed to watch directory: {:?}", e);
                return;
            }

            info!("Started watching config file at {:?}", config_path);

            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        // Check if the event matches the config_path
                        if event.paths.iter().any(|p| p == &config_path) {
                            match event.kind {
                                EventKind::Modify(_) | EventKind::Create(_) => {
                                    debug!("Config file changed: {:?}", config_path);
                                    
                                    // Try reading the file
                                    match tokio::fs::read_to_string(&config_path).await {
                                        Ok(content) => {
                                            match serde_json::from_str::<serde_json::Value>(&content) {
                                                Ok(parsed_json) => {
                                                    let wave_event = WaveEvent::global(EVENT_CONFIG, parsed_json);
                                                    broker.publish(wave_event);
                                                }
                                                Err(e) => {
                                                    error!("Failed to parse config file JSON: {:?}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to read config file: {:?}", e);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {:?}", e);
                    }
                }
            }
            
            // Keep watcher alive
            drop(watcher);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;
    use std::time::Duration;
    use tokio::time::timeout;
    use serde_json::json;
    use sheshaaos_wps::events::SubscriptionRequest;

    #[tokio::test]
    async fn test_config_watcher() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        
        // Initially empty
        std::fs::File::create(&config_path).unwrap();

        let broker = Broker::new(10);
        
        broker.subscribe("test_route", SubscriptionRequest {
            topic: EVENT_CONFIG.to_string(),
            scopes: vec![],
        });
        
        let mut subscriber = broker.receiver();

        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        watcher.start();

        // Give watcher some time to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        let test_json = json!({
            "term:theme": "light",
            "ai:model": "claude-3"
        });

        // Modify file
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(serde_json::to_string(&test_json).unwrap().as_bytes()).unwrap();
        file.sync_all().unwrap();

        // Await event
        let (route, event) = timeout(Duration::from_secs(1), subscriber.recv())
            .await
            .expect("Timeout waiting for event")
            .expect("Channel closed");

        assert_eq!(route, "test_route");
        assert_eq!(event.topic, EVENT_CONFIG);
        assert_eq!(event.data, test_json);
    }

    #[tokio::test]
    async fn test_config_watcher_new_stores_path_and_broker() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let broker = Broker::new(10);

        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        // ConfigWatcher stores the path and broker; we verify start doesn't panic
        watcher.start();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_config_watcher_does_not_panic_on_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad.json");
        std::fs::write(&config_path, "not valid json {{{").unwrap();

        let broker = Broker::new(10);
        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        watcher.start();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Modify file with invalid JSON - watcher should log error but not panic
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"also invalid").unwrap();
        file.sync_all().unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn test_config_watcher_multiple_subscribers() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        std::fs::File::create(&config_path).unwrap();

        let broker = Broker::new(10);
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: EVENT_CONFIG.to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: EVENT_CONFIG.to_string(),
            scopes: vec![],
        });
        
        let mut subscriber = broker.receiver();

        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        watcher.start();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let test_json = json!({"key": "value"});
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(serde_json::to_string(&test_json).unwrap().as_bytes()).unwrap();
        file.sync_all().unwrap();

        let mut routes = Vec::new();
        for _ in 0..2 {
            let (r, ev) = timeout(Duration::from_secs(1), subscriber.recv())
                .await
                .expect("Timeout")
                .expect("Channel closed");
            routes.push(r);
            assert_eq!(ev.data, test_json);
        }
        routes.sort();
        assert_eq!(routes, vec!["route1", "route2"]);
    }

    #[tokio::test]
    async fn test_config_watcher_without_subscribers() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        std::fs::File::create(&config_path).unwrap();

        let broker = Broker::new(10);
        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        watcher.start();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let test_json = json!({"key": "value"});
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(serde_json::to_string(&test_json).unwrap().as_bytes()).unwrap();
        file.sync_all().unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;
        // Should not panic even with no subscribers
    }

    #[tokio::test]
    async fn test_config_watcher_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested/config/settings.json");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::File::create(&config_path).unwrap();

        let broker = Broker::new(10);
        broker.subscribe("test_route", SubscriptionRequest {
            topic: EVENT_CONFIG.to_string(),
            scopes: vec![],
        });
        
        let mut subscriber = broker.receiver();

        let watcher = ConfigWatcher::new(config_path.clone(), broker.clone());
        watcher.start();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let test_json = json!({"nested": true});
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(serde_json::to_string(&test_json).unwrap().as_bytes()).unwrap();
        file.sync_all().unwrap();

        let (route, event) = timeout(Duration::from_secs(1), subscriber.recv())
            .await
            .expect("Timeout")
            .expect("Channel closed");

        assert_eq!(route, "test_route");
        assert_eq!(event.data, test_json);
    }
}
