use std::path::PathBuf;

use russh::client::Handler;
use russh::keys::known_hosts::{check_known_hosts_path, learn_known_hosts_path};
use russh::keys::PublicKey;

/// Known hosts file path for trust-on-first-use (TOFU) verification.
fn known_hosts_path() -> PathBuf {
    let mut path =
        std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    path.push(".ssh");
    path.push("known_hosts");
    path
}

#[derive(Clone, Debug)]
pub struct ClientHandler {
    /// The hostname being connected to, for known_hosts lookup.
    pub host: String,
    /// The port being connected to (known_hosts keys on host+port).
    pub port: u16,
    /// If true, accept any new (unknown) server key immediately (TOFU).
    pub trust_new: bool,
}

impl Default for ClientHandler {
    fn default() -> Self {
        Self { host: String::new(), port: 22, trust_new: true }
    }
}

impl Handler for ClientHandler {
    type Error = russh::Error;

    // russh 0.6x uses native async-fn-in-trait (no async_trait attribute) and
    // ships its own OpenSSH-format known_hosts handling — previously we kept a
    // hand-rolled two-field format that could not interoperate with entries
    // written by OpenSSH (three fields: host keytype key).
    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let path = known_hosts_path();
        let known = check_known_hosts_path(&self.host, self.port, server_public_key, &path)
            .unwrap_or(false);

        if known {
            return Ok(true);
        }

        // Present in file with a different key, or absent altogether?
        let recorded = russh::keys::known_hosts::known_host_keys_path(&self.host, self.port, &path)
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        if recorded {
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
            if let Err(e) = learn_known_hosts_path(&self.host, self.port, server_public_key, &path)
            {
                tracing::error!("failed to persist known host key for {}: {}", self.host, e);
                return Ok(false);
            }
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

    fn test_pubkey() -> PublicKey {
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)
            .expect("ed25519 keygen")
            .public_key()
            .clone()
    }

    #[test]
    fn test_client_handler_default_is_tofu_open() {
        let handler = ClientHandler::default();
        assert!(handler.host.is_empty());
        assert_eq!(handler.port, 22);
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

    /// Full TOFU lifecycle with real upstream known_hosts handling, isolated HOME.
    /// One tokio test owns the env so parallel tests never race on HOME.
    #[tokio::test]
    async fn test_tofu_full_lifecycle() {
        let tmp = std::env::temp_dir().join(format!("shesh-ssh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("HOME", &tmp);

        let public = test_pubkey();

        // 1. unknown host + trust_new=false -> reject, nothing stored
        let mut strict = ClientHandler { host: "strict.test".into(), port: 22, trust_new: false };
        assert!(!strict.check_server_key(&public).await.unwrap());
        let keys =
            russh::keys::known_hosts::known_host_keys_path("strict.test", 22, known_hosts_path())
                .unwrap();
        assert!(keys.is_empty());

        // 2. unknown host + trust_new=true -> accept and persist
        let mut tofu = ClientHandler { host: "example.test".into(), port: 22, trust_new: true };
        assert!(tofu.check_server_key(&public).await.unwrap());
        let keys =
            russh::keys::known_hosts::known_host_keys_path("example.test", 22, known_hosts_path())
                .unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].1, public);

        // 3. known host + same key -> accept even in strict mode
        let mut again = ClientHandler { host: "example.test".into(), port: 22, trust_new: false };
        assert!(again.check_server_key(&public).await.unwrap());

        // 4. known host + different key -> reject (MITM guard)
        let other = test_pubkey();
        assert!(!again.check_server_key(&other).await.unwrap());

        // 5. second host appends (no truncation of host 1)
        let mut second = ClientHandler { host: "other.test".into(), port: 22, trust_new: true };
        assert!(second.check_server_key(&other).await.unwrap());
        assert!(!russh::keys::known_hosts::known_host_keys_path(
            "example.test",
            22,
            known_hosts_path()
        )
        .unwrap()
        .is_empty());
        assert!(!russh::keys::known_hosts::known_host_keys_path(
            "other.test",
            22,
            known_hosts_path()
        )
        .unwrap()
        .is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
