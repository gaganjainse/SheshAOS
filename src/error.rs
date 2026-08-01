//! Unified error types for NexusAOS.
//!
//! Each subsystem defines its own error type. `NexusError` wraps them all
//! for propagation through the kernel and CLI.

use thiserror::Error;

/// Top-level error type for NexusAOS operations.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NexusError {
    /// Configuration loading or validation error.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Event store or snapshot error.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Policy engine error.
    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),

    /// Model provider error.
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Tool execution error.
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// Task lifecycle error.
    #[error("Task error: {0}")]
    Task(#[from] TaskError),

    /// Resource or context budget error.
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization / deserialization error.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Errors related to configuration loading and validation.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    NotFound { path: String },

    #[error("Invalid configuration: {message}")]
    Invalid { message: String },

    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to the event store and snapshots.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StorageError {
    #[error("Event store I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Event serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Duplicate event ID: {id}")]
    DuplicateEvent { id: String },

    #[error("Event not found: {id}")]
    EventNotFound { id: String },

    #[error("Snapshot corrupted: {message}")]
    CorruptedSnapshot { message: String },

    #[error("Storage path not writable: {path}")]
    NotWritable { path: String },
}

/// Errors related to policy enforcement.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum PolicyError {
    #[error("Action denied by policy: {reason}")]
    Denied { reason: String },

    #[error("Capability not held: {capability}")]
    MissingCapability { capability: String },

    #[error("Capability expired: {capability}")]
    ExpiredCapability { capability: String },

    #[error("Invalid policy rule: {message}")]
    InvalidRule { message: String },
}

/// Errors related to model providers.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ProviderError {
    #[error("Provider not available: {name}")]
    Unavailable { name: String },

    #[error("Provider health check failed: {name}: {reason}")]
    HealthCheckFailed { name: String, reason: String },

    #[error("Model inference failed: {0}")]
    InferenceFailed(String),

    #[error("Model response malformed: {0}")]
    MalformedResponse(String),

    #[error("Inference request timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("Inference cancelled")]
    Cancelled,

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("No provider registered for role: {role}")]
    NoProviderForRole { role: String },
}

/// Errors related to tool execution.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ToolError {
    #[error("Tool not found: {name}")]
    NotFound { name: String },

    #[error("Tool execution failed: {name}: {reason}")]
    ExecutionFailed { name: String, reason: String },

    #[error("Tool timed out: {name} after {timeout_secs}s")]
    Timeout { name: String, timeout_secs: u64 },

    #[error("Tool I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not allowed: {path}")]
    PathDenied { path: String },

    #[error("Command denied: {command}")]
    CommandDenied { command: String },
}

/// Errors related to task lifecycle.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TaskError {
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Task not found: {id}")]
    NotFound { id: String },

    #[error("Task queue full (max {max_depth})")]
    QueueFull { max_depth: usize },

    #[error("Duplicate task detected")]
    Duplicate,

    #[error("Task cancelled")]
    Cancelled,
}

/// Errors related to resource monitoring and context budgeting.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ResourceError {
    #[error("Insufficient RAM: need {needed_mb} MB, available {available_mb} MB")]
    InsufficientRam { needed_mb: u64, available_mb: u64 },

    #[error("Insufficient VRAM: need {needed_mb} MB, available {available_mb} MB")]
    InsufficientVram { needed_mb: u64, available_mb: u64 },

    #[error("Insufficient disk space: need {needed_gb} GB, available {available_gb} GB")]
    InsufficientDisk { needed_gb: u64, available_gb: u64 },

    #[error("Context budget refused: requested {requested} tokens, max allowed {max_allowed}")]
    ContextBudgetExceeded { requested: usize, max_allowed: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err =
            NexusError::Config(ConfigError::NotFound { path: "/tmp/missing.toml".to_string() });
        assert!(err.to_string().contains("missing.toml"));
    }

    #[test]
    fn test_error_from_storage() {
        let storage_err = StorageError::DuplicateEvent { id: "abc-123".to_string() };
        let nexus_err: NexusError = storage_err.into();
        assert!(nexus_err.to_string().contains("abc-123"));
    }

    #[test]
    fn test_error_from_policy() {
        let policy_err = PolicyError::Denied { reason: "no write access".to_string() };
        let nexus_err: NexusError = policy_err.into();
        assert!(nexus_err.to_string().contains("no write access"));
    }

    #[test]
    fn test_error_from_provider() {
        let provider_err = ProviderError::Timeout { timeout_secs: 30 };
        let nexus_err: NexusError = provider_err.into();
        assert!(nexus_err.to_string().contains("30s"));
    }

    #[test]
    fn test_error_from_task() {
        let task_err = TaskError::QueueFull { max_depth: 32 };
        let nexus_err: NexusError = task_err.into();
        assert!(nexus_err.to_string().contains("32"));
    }

    #[test]
    fn test_error_from_resource() {
        let resource_err = ResourceError::InsufficientRam { needed_mb: 8000, available_mb: 4000 };
        let nexus_err: NexusError = resource_err.into();
        assert!(nexus_err.to_string().contains("8000"));
    }
}
