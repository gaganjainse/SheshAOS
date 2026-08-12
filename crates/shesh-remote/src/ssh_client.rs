use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use russh::client::Handler;
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;

/// Known hosts file path for trust-on-first-use (TOFU) verification.
fn known_hosts_path() -> PathBuf {
    let mut path =
        std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    path.push(".ssh");
    path.push("known_hosts");
    path
}

/// Serialize a public key as the standard base64 blob for known_hosts storage.
/// (russh-keys 0.43 removed `to_bytes`; `PublicKeyBase64` is the API.)
fn key_to_string(key: &PublicKey) -> String {
    key.public_key_base64()
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
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        // append, never truncate — a write() here would drop every prior host
        let _ = file.write_all(entry.as_bytes());
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
        Self { host: String::new(), trust_new: true }
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
    fn test_client_handler_default_is_tofu_open() {
        let handler = ClientHandler::default();
        assert!(handler.host.is_empty());
        assert!(handler.trust_new);
    }

    #[test]
    fn test_client_handler_clone_debug_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        let handler = ClientHandler::default();
        let cloned = handler.clone();
        assert_send_sync(&cloned);
        assert!(format!("{:?}", cloned).contains("ClientHandler"));
    }

    /// Full TOFU lifecycle with a real ed25519 keypair, isolated HOME.
    /// One test owns the env so parallel tests never race on HOME.
    #[tokio::test]
    async fn test_tofu_full_lifecycle() {
        let tmp = std::env::temp_dir().join(format!("shesh-ssh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("HOME", &tmp);

        let keypair = russh_keys::key::KeyPair::generate_ed25519().expect("ed25519");
        let public = keypair.clone_public_key().expect("public key");

        // 1. unknown host + trust_new=false -> reject, nothing stored
        let mut strict = ClientHandler { host: "strict.test".into(), trust_new: false };
        assert!(!strict.check_server_key(&public).await.unwrap());
        assert!(!load_known_hosts().contains_key("strict.test"));

        // 2. unknown host + trust_new=true -> accept and persist
        let mut tofu = ClientHandler { host: "example.test".into(), trust_new: true };
        assert!(tofu.check_server_key(&public).await.unwrap());
        let stored = load_known_hosts().get("example.test").cloned();
        assert_eq!(stored.as_deref(), Some(key_to_string(&public).as_str()));

        // 3. known host + same key -> accept
        let mut again = ClientHandler { host: "example.test".into(), trust_new: false };
        assert!(again.check_server_key(&public).await.unwrap());

        // 4. known host + different key -> reject (MITM guard)
        let other =
            russh_keys::key::KeyPair::generate_ed25519().unwrap().clone_public_key().unwrap();
        assert!(!again.check_server_key(&other).await.unwrap());

        // 5. second save appends (no truncation of host 1)
        let mut second = ClientHandler { host: "other.test".into(), trust_new: true };
        assert!(second.check_server_key(&other).await.unwrap());
        let hosts = load_known_hosts();
        assert!(hosts.contains_key("example.test") && hosts.contains_key("other.test"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
