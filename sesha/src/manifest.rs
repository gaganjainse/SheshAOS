use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ManifestStatus {
    Draft,
    Validated,
    Signed,
    Active,
    Superseded,
    Retired,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub version: String,
    pub status: ManifestStatus,
    pub agent_roles: HashMap<String, AgentRoleDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRoleDefinition {
    pub name: String,
    pub capabilities: Vec<String>,
    pub description: String,
}

impl Default for ManifestStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_status_default() {
        assert_eq!(ManifestStatus::default(), ManifestStatus::Draft);
    }
}
