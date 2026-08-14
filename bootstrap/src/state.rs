use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Received,
    Classified,
    Planned,
    Authorized,
    Executing,
    Blocked,
    Failed,
    Completed,
    RolledBack,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum KernelStatus {
    Idle,
    Busy,
    Recovering,
    Maintenance,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Received
    }
}

impl Default for KernelStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_states() {
        assert_eq!(TaskStatus::default(), TaskStatus::Received);
        assert_eq!(KernelStatus::default(), KernelStatus::Idle);
    }
}
