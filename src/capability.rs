// src/capability.rs - Capability-based security types
// All types derive Debug, Clone, Serialize, Deserialize

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a capability grants access to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// Access to a filesystem path (and its children)
    Path(PathBuf),
    /// Access to run a specific command pattern
    Command(String),
    /// Access to use a specific model
    Model(String),
    /// Access to a specific tool
    Tool(String),
    /// Unrestricted (dangerous — requires explicit grant)
    Global,
}

/// A named capability with a defined scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub scope: Scope,
    pub description: String,
}

/// A time-bound lease granting a capability
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityLease {
    pub id: Uuid,
    pub capability: Capability,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub granted_by: String,
    pub revoked: bool,
}

impl CapabilityLease {
    /// Checks if the lease is valid (not revoked and not expired)
    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }
        if matches!(self.expires_at, Some(exp) if Utc::now() >= exp) {
            return false;
        }
        true
    }

    /// Revokes this lease
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Checks if this lease grants access to the specified path
    pub fn covers_path(&self, path: &Path) -> bool {
        if !self.is_valid() {
            return false;
        }
        match &self.capability.scope {
            Scope::Global => true,
            Scope::Path(p) => path.starts_with(p),
            _ => false,
        }
    }

    /// Checks if this lease grants access to the specified command
    pub fn covers_command(&self, cmd: &str) -> bool {
        if !self.is_valid() {
            return false;
        }
        match &self.capability.scope {
            Scope::Global => true,
            Scope::Command(c) => cmd.starts_with(c),
            _ => false,
        }
    }
}

/// A set of active capability leases
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub leases: Vec<CapabilityLease>,
}

impl CapabilitySet {
    /// Creates a new empty capability set
    pub fn new() -> Self {
        Self { leases: Vec::new() }
    }

    /// Grants a new capability, appending it to the set, and returning a reference to it
    pub fn grant(
        &mut self,
        capability: Capability,
        granted_by: String,
        ttl: Option<Duration>,
    ) -> &CapabilityLease {
        let now = Utc::now();
        let expires_at = ttl.and_then(|d| chrono::Duration::from_std(d).ok()).map(|d| now + d);

        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability,
            granted_at: now,
            expires_at,
            granted_by,
            revoked: false,
        };

        self.leases.push(lease);
        let len = self.leases.len();
        &self.leases[len - 1]
    }

    /// Revokes a capability lease by its ID
    pub fn revoke(&mut self, lease_id: &Uuid) {
        for lease in &mut self.leases {
            if lease.id == *lease_id {
                lease.revoke();
            }
        }
    }

    /// Checks if there is a valid capability with the exact given name
    pub fn has_capability(&self, name: &str) -> bool {
        self.leases.iter().any(|l| l.is_valid() && l.capability.name == name)
    }

    /// Checks if any valid lease covers the given path
    pub fn check_path(&self, path: &Path) -> bool {
        self.leases.iter().any(|l| l.covers_path(path))
    }

    /// Checks if any valid lease covers the given command
    pub fn check_command(&self, cmd: &str) -> bool {
        self.leases.iter().any(|l| l.covers_command(cmd))
    }

    /// Removes all expired or revoked leases from the set
    pub fn cleanup_expired(&mut self) {
        self.leases.retain(|l| l.is_valid());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_creation_and_validity() {
        let cap = Capability {
            name: "test".to_string(),
            scope: Scope::Global,
            description: "test".to_string(),
        };
        let mut set = CapabilitySet::new();
        let lease = set.grant(cap.clone(), "admin".to_string(), None);
        assert!(lease.is_valid());
        assert!(set.has_capability("test"));
    }

    #[test]
    fn test_lease_revocation() {
        let cap = Capability {
            name: "test".to_string(),
            scope: Scope::Global,
            description: "test".to_string(),
        };
        let mut set = CapabilitySet::new();
        let lease = set.grant(cap, "admin".to_string(), None);
        let id = lease.id;

        assert!(set.has_capability("test"));
        set.revoke(&id);
        assert!(!set.has_capability("test"));

        set.cleanup_expired();
        assert_eq!(set.leases.len(), 0);
    }

    #[test]
    fn test_path_coverage() {
        let cap = Capability {
            name: "fs_read".to_string(),
            scope: Scope::Path(PathBuf::from("/etc")),
            description: "read etc".to_string(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".to_string(), None);

        assert!(set.check_path(Path::new("/etc/passwd")));
        assert!(set.check_path(Path::new("/etc")));
        assert!(!set.check_path(Path::new("/var/log")));
    }

    #[test]
    fn test_command_coverage() {
        let cap = Capability {
            name: "cmd".to_string(),
            scope: Scope::Command("ls".to_string()),
            description: "run ls".to_string(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".to_string(), None);

        assert!(set.check_command("ls -la"));
        assert!(!set.check_command("cat file.txt"));
    }

    #[test]
    fn test_expiration() {
        let cap = Capability {
            name: "expiring".to_string(),
            scope: Scope::Global,
            description: "expires immediately".to_string(),
        };
        let mut set = CapabilitySet::new();
        // Since we can't easily mock time, we manually set an expired time
        let now = Utc::now();
        let past = now - chrono::Duration::days(1);

        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability: cap,
            granted_at: past,
            expires_at: Some(past),
            granted_by: "admin".to_string(),
            revoked: false,
        };
        set.leases.push(lease);

        assert!(!set.has_capability("expiring"));
        set.cleanup_expired();
        assert!(set.leases.is_empty());
    }
}
