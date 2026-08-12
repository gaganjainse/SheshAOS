// src/capability.rs - Capability-based security types
// All types derive Debug, Clone, Serialize, Deserialize

use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lexically normalizes a path, resolving `.` and `..` components.
/// Returns `None` if the path attempts to escape above its root.
fn normalize_lexical(path: &Path) -> Option<PathBuf> {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if components.last().is_some_and(|c: &Component| matches!(c, Component::Normal(_)))
                {
                    components.pop();
                } else if components.is_empty() {
                    // Path escapes above root
                    return None;
                }
                // If the last is a RootDir or Prefix, we don't pop — reject
            }
            Component::RootDir => {
                if components.is_empty() {
                    components.push(comp);
                }
                // Only allow one root at the start
            }
            Component::Prefix(_) => {
                components.push(comp);
            }
            Component::Normal(_) => {
                components.push(comp);
            }
        }
    }
    if components.is_empty() {
        return Some(PathBuf::new());
    }
    Some(components.iter().fold(PathBuf::new(), |mut acc, c| {
        acc.push(c.as_os_str());
        acc
    }))
}

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
            Scope::Path(p) => {
                let normalized = normalize_lexical(path);
                match normalized {
                    Some(n) => n.starts_with(p),
                    None => false,
                }
            }
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

    #[test]
    fn test_scope_equality() {
        assert_eq!(Scope::Path(PathBuf::from("/tmp")), Scope::Path(PathBuf::from("/tmp")));
        assert_eq!(Scope::Global, Scope::Global);
        assert_eq!(Scope::Command("ls".into()), Scope::Command("ls".into()));
        assert_ne!(Scope::Path(PathBuf::from("/tmp")), Scope::Path(PathBuf::from("/var")));
    }

    #[test]
    fn test_capability_equality() {
        let cap1 = Capability {
            name: "read".into(),
            scope: Scope::Path(PathBuf::from("/etc")),
            description: "read etc".into(),
        };
        let cap2 = Capability {
            name: "read".into(),
            scope: Scope::Path(PathBuf::from("/etc")),
            description: "read etc".into(),
        };
        assert_eq!(cap1, cap2);
    }

    #[test]
    fn test_lease_valid_no_expiry() {
        let cap =
            Capability { name: "perm".into(), scope: Scope::Global, description: "perm".into() };
        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability: cap,
            granted_at: Utc::now(),
            expires_at: None,
            granted_by: "admin".into(),
            revoked: false,
        };
        assert!(lease.is_valid());
    }

    #[test]
    fn test_lease_valid_future_expiry() {
        let cap =
            Capability { name: "perm".into(), scope: Scope::Global, description: "perm".into() };
        let now = Utc::now();
        let future = now + chrono::Duration::days(1);
        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability: cap,
            granted_at: now,
            expires_at: Some(future),
            granted_by: "admin".into(),
            revoked: false,
        };
        assert!(lease.is_valid());
    }

    #[test]
    fn test_lease_expires_at_now_boundary() {
        let cap =
            Capability { name: "perm".into(), scope: Scope::Global, description: "perm".into() };
        let now = Utc::now();
        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability: cap,
            granted_at: now,
            expires_at: Some(now),
            granted_by: "admin".into(),
            revoked: false,
        };
        assert!(!lease.is_valid());
    }

    #[test]
    fn test_capability_set_empty() {
        let set = CapabilitySet::new();
        assert!(set.leases.is_empty());
        assert!(!set.has_capability("anything"));
        assert!(!set.check_path(Path::new("/any")));
        assert!(!set.check_command("anything"));
    }

    #[test]
    fn test_grant_with_ttl() {
        let cap =
            Capability { name: "temp".into(), scope: Scope::Global, description: "temp".into() };
        let mut set = CapabilitySet::new();
        let ttl = Duration::from_secs(3600);
        let lease = set.grant(cap, "admin".into(), Some(ttl));
        assert!(lease.is_valid());
        assert!(set.has_capability("temp"));
        assert_eq!(set.leases.len(), 1);
    }

    #[test]
    fn test_revoke_nonexistent_lease() {
        let mut set = CapabilitySet::new();
        let fake_id = Uuid::new_v4();
        set.revoke(&fake_id);
        assert!(set.leases.is_empty());
    }

    #[test]
    fn test_revoke_partial_set() {
        let cap1 =
            Capability { name: "a".to_string(), scope: Scope::Global, description: "".into() };
        let cap2 =
            Capability { name: "b".to_string(), scope: Scope::Global, description: "".into() };
        let mut set = CapabilitySet::new();
        let id1 = set.grant(cap1, "admin".to_string(), None).id;
        let _id2 = set.grant(cap2, "admin".to_string(), None).id;
        assert_eq!(set.leases.len(), 2);

        set.revoke(&id1);
        assert!(set.has_capability("b"));
        assert!(!set.has_capability("a"));
        assert_eq!(set.leases.len(), 2); // not cleaned up yet
    }

    #[test]
    fn test_cleanup_mixed_expired_and_valid() {
        let now = Utc::now();
        let past = now - chrono::Duration::days(1);
        let future = now + chrono::Duration::days(1);

        let mut set = CapabilitySet::new();
        set.leases.push(CapabilityLease {
            id: Uuid::new_v4(),
            capability: Capability {
                name: "expired".into(),
                scope: Scope::Global,
                description: "".into(),
            },
            granted_at: past,
            expires_at: Some(past),
            granted_by: "admin".into(),
            revoked: false,
        });
        set.leases.push(CapabilityLease {
            id: Uuid::new_v4(),
            capability: Capability {
                name: "valid".into(),
                scope: Scope::Global,
                description: "".into(),
            },
            granted_at: now,
            expires_at: Some(future),
            granted_by: "admin".into(),
            revoked: false,
        });
        set.leases.push(CapabilityLease {
            id: Uuid::new_v4(),
            capability: Capability {
                name: "revoked".into(),
                scope: Scope::Global,
                description: "".into(),
            },
            granted_at: now,
            expires_at: Some(future),
            granted_by: "admin".into(),
            revoked: true,
        });

        set.cleanup_expired();
        assert_eq!(set.leases.len(), 1);
        assert!(set.has_capability("valid"));
    }

    #[test]
    fn test_covers_path_non_path_scope() {
        let cap = Capability {
            name: "cmd".into(),
            scope: Scope::Command("ls".into()),
            description: "".into(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".into(), None);
        assert!(!set.check_path(Path::new("/etc/passwd")));
    }

    #[test]
    fn test_covers_command_non_command_scope() {
        let cap = Capability {
            name: "path".into(),
            scope: Scope::Path(PathBuf::from("/etc")),
            description: "".into(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".into(), None);
        assert!(!set.check_command("ls -la"));
    }

    #[test]
    fn test_path_coverage_edge_cases() {
        let cap = Capability {
            name: "fs".into(),
            scope: Scope::Path(PathBuf::from("/etc")),
            description: "".into(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".into(), None);

        assert!(set.check_path(Path::new("/etc")));
        assert!(set.check_path(Path::new("/etc/nginx/nginx.conf")));
        assert!(!set.check_path(Path::new("/etc_"))); // prefix match, not contains
        assert!(!set.check_path(Path::new("/var/etc")));
    }

    #[test]
    fn test_command_coverage_edge_cases() {
        let cap = Capability {
            name: "cmd".into(),
            scope: Scope::Command("git".into()),
            description: "".into(),
        };
        let mut set = CapabilitySet::new();
        set.grant(cap, "admin".into(), None);

        assert!(set.check_command("git"));
        assert!(set.check_command("git status"));
        assert!(set.check_command("git log --oneline"));
        assert!(set.check_command("gittxt")); // starts with "git"
    }

    #[test]
    fn test_has_capability_multiple_grants() {
        let mut set = CapabilitySet::new();
        set.grant(
            Capability { name: "a".into(), scope: Scope::Global, description: "".into() },
            "admin".into(),
            None,
        );
        set.grant(
            Capability { name: "b".into(), scope: Scope::Global, description: "".into() },
            "admin".into(),
            None,
        );
        set.grant(
            Capability { name: "c".into(), scope: Scope::Global, description: "".into() },
            "admin".into(),
            None,
        );

        assert!(set.has_capability("a"));
        assert!(set.has_capability("b"));
        assert!(set.has_capability("c"));
        assert!(!set.has_capability("d"));
    }

    #[test]
    fn test_scope_serde_roundtrip() {
        let scopes = vec![
            Scope::Path(PathBuf::from("/tmp")),
            Scope::Command("ls".into()),
            Scope::Model("gpt4".into()),
            Scope::Tool("editor".into()),
            Scope::Global,
        ];
        for scope in scopes {
            let json = serde_json::to_string(&scope).unwrap();
            let back: Scope = serde_json::from_str(&json).unwrap();
            assert_eq!(scope, back);
        }
    }

    #[test]
    fn test_capability_lease_serde_roundtrip() {
        let lease = CapabilityLease {
            id: Uuid::new_v4(),
            capability: Capability {
                name: "test".into(),
                scope: Scope::Global,
                description: "desc".into(),
            },
            granted_at: Utc::now(),
            expires_at: None,
            granted_by: "admin".into(),
            revoked: false,
        };
        let json = serde_json::to_string(&lease).unwrap();
        let back: CapabilityLease = serde_json::from_str(&json).unwrap();
        assert_eq!(lease.id, back.id);
        assert_eq!(lease.capability.name, back.capability.name);
        assert_eq!(lease.granted_by, back.granted_by);
    }

    #[test]
    fn test_grant_returns_reference_to_last_lease() {
        let mut set = CapabilitySet::new();
        let cap = Capability { name: "ref".into(), scope: Scope::Global, description: "".into() };
        let id1 = set.grant(cap.clone(), "admin".into(), None).id;
        let id2 = set.grant(cap, "admin".into(), None).id;
        assert_eq!(id1, set.leases[0].id);
        assert_eq!(id2, set.leases[1].id);
    }
}
