use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Capability {
    Filesystem,
    Git,
    Terminal,
    Network,
    Docker,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeaseId(Uuid);

impl LeaseId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapabilityLease {
    pub id: LeaseId,
    pub capability: Capability,
    pub scope: String,
    pub expiry: DateTime<Utc>,
}

impl CapabilityLease {
    pub fn is_expired(&self) -> bool {
        self.expiry < Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_lease_expiry() {
        let lease = CapabilityLease {
            id: LeaseId(Uuid::new_v4()),
            capability: Capability::Network,
            scope: "*".to_string(),
            expiry: Utc::now() - Duration::minutes(1),
        };
        assert!(lease.is_expired());
    }
}
