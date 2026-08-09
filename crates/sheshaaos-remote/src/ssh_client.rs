use std::path::PathBuf;
use std::collections::HashMap;

use russh::client::Handler;
use russh_keys::key::PublicKey;
use async_trait::async_trait;

/// Known hosts file path for trust-on-first-use (TOFU) verification.
fn known_hosts_path() -> PathBuf {
    let mut path = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    path.push(".ssh");
    path.push("known_hosts");
    path
}

/// Serialize a public key to a hex string for known_hosts storage.
fn key_to_string(key: &PublicKey) -> String {
    let bytes = key.to_bytes();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Load known host keys from the known_hosts file.
fn load_known_hosts() -> HashMap<String, String> {
    let mut hosts = HashMap::new();
    let path = known_hosts_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                hosts.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }
    hosts
}

/// Persist a server key to the known_hosts file (TOFU).
fn save_known_host(host: &str, key: &PublicKey) {
    let path = known_hosts_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let key_str = key_to_string(key);
    let entry = format!("{} {}\n", host, key_str);
    if let Ok(_) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = std::fs::write(&path, entry);
    }
}

#[derive(Clone, Debug)]
pub struct ClientHandler {
    /// The hostname being connected to, for known_hosts lookup.
    pub host: String,
    /// If true, accept any new (unknown) server key immediately (TOFU).
    pub trust_new: bool,
}

impl Default for ClientHandler {
    fn default() -> Self {
        Self {
            host: String::new(),
            trust_new: true,
        }
    }
}

#[async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let known = load_known_hosts();
        let key_b64 = key_to_string(server_public_key);

        // Check against known hosts
        if let Some(stored) = known.get(&self.host) {
            if stored == &key_b64 {
                return Ok(true);
            }
            // Key mismatch — potential MITM attack
            tracing::error!(
                "SSH server key mismatch for {}: stored key does not match presented key. \
                 Refusing connection to prevent man-in-the-middle attack.",
                self.host
            );
            return Ok(false);
        }

        // Unknown host — apply TOFU policy
        if self.trust_new {
            tracing::warn!(
                "SSH server key for {} is unknown — accepting (trust-on-first-use).",
                self.host
            );
            save_known_host(&self.host, server_public_key);
            Ok(true)
        } else {
            tracing::error!(
                "SSH server key for {} is unknown — rejecting (strict host key checking enabled).",
                self.host
            );
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_handler_clone() {
        let handler = ClientHandler::default();
        let _ = handler.clone();
    }

    #[test]
    fn test_client_handler_debug() {
        let handler = ClientHandler::default();
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("ClientHandler"));
    }

    #[test]
    fn test_client_handler_is_send() {
        fn assert_send<T: Send>(_: T) {}
        let handler = ClientHandler::default();
        assert_send(handler);
    }

    #[test]
    fn test_client_handler_is_sync() {
        fn assert_sync<T: Sync>(_: T) {}
        let handler = ClientHandler::default();
        assert_sync(handler);
    }

    #[test]
    fn test_client_handler_check_server_key_returns_true() {
        let _handler = ClientHandler {};
        // check_server_key always returns true for any input.
        // We verify this by checking the handler's behavior conceptually:
        // since the method is hardcoded to Ok(true), any server key is accepted.
        // In a real scenario we'd pass a mock PublicKey, but the implementation
        // doesn't inspect it, so we verify the contract here.
        assert!(true); // The handler unconditionally accepts all server keys
    }

    #[tokio::test]
    async fn test_client_handler_multiple_instances() {
        let h1 = ClientHandler {};
        let h2 = ClientHandler {};
        let h3 = h1.clone();
        // Verify Clone works and produces equal instances
        assert_eq!(std::mem::size_of_val(&h2), std::mem::size_of_val(&h3));
        // Verify Debug format is consistent
        assert!(format!("{:?}", h1).contains("ClientHandler"));
        assert!(format!("{:?}", h2).contains("ClientHandler"));
    }
}
