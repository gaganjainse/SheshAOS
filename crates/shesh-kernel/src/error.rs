//! Unified error types for SheshAOS.
//!
//! Each subsystem defines its own error type. `KernelError` wraps them all
//! for propagation through the kernel and CLI.

use thiserror::Error;

/// Top-level error type for SheshAOS operations.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum KernelError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Task error: {0}")]
    Task(#[from] TaskError),

    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

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

    #[error("TOML serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to the event store and snapshots.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StorageError {
    #[error("Event store I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

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

    #[error("Provider API error: {0}")]
    Api(String),

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
    use crate::config::AppConfig;

    #[test]
    fn test_error_display() {
        let err =
            KernelError::Config(ConfigError::NotFound { path: "/tmp/missing.toml".to_string() });
        assert!(err.to_string().contains("missing.toml"));
    }

    #[test]
    fn test_error_from_storage() {
        let storage_err = StorageError::DuplicateEvent { id: "abc-123".to_string() };
        let kernel_err: KernelError = storage_err.into();
        assert!(kernel_err.to_string().contains("abc-123"));
    }

    #[test]
    fn test_error_from_policy() {
        let policy_err = PolicyError::Denied { reason: "no write access".to_string() };
        let kernel_err: KernelError = policy_err.into();
        assert!(kernel_err.to_string().contains("no write access"));
    }

    #[test]
    fn test_error_from_provider() {
        let provider_err = ProviderError::Timeout { timeout_secs: 30 };
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("30s"));
    }

    #[test]
    fn test_error_from_task() {
        let task_err = TaskError::QueueFull { max_depth: 32 };
        let kernel_err: KernelError = task_err.into();
        assert!(kernel_err.to_string().contains("32"));
    }

    #[test]
    fn test_error_from_resource() {
        let resource_err = ResourceError::InsufficientRam { needed_mb: 8000, available_mb: 4000 };
        let kernel_err: KernelError = resource_err.into();
        assert!(kernel_err.to_string().contains("8000"));
    }

    #[test]
    fn test_error_display_config_invalid() {
        let err = KernelError::Config(ConfigError::Invalid { message: "bad config".to_string() });
        assert!(err.to_string().contains("bad config"));
        assert!(err.to_string().contains("Configuration error"));
    }

    #[test]
    fn test_error_display_storage_event_not_found() {
        let err = KernelError::Storage(StorageError::EventNotFound { id: "evt-1".to_string() });
        assert!(err.to_string().contains("evt-1"));
        assert!(err.to_string().contains("Storage error"));
    }

    #[test]
    fn test_error_display_storage_corrupted() {
        let err = KernelError::Storage(StorageError::CorruptedSnapshot {
            message: "bad data".to_string(),
        });
        assert!(err.to_string().contains("bad data"));
    }

    #[test]
    fn test_error_display_storage_not_writable() {
        let err = KernelError::Storage(StorageError::NotWritable { path: "/ro".to_string() });
        assert!(err.to_string().contains("/ro"));
    }

    #[test]
    fn test_error_from_storage_serialization() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad json").unwrap_err();
        let storage_err = StorageError::Serialization(json_err);
        let kernel_err: KernelError = storage_err.into();
        assert!(kernel_err.to_string().contains("Storage error"));
    }

    #[test]
    fn test_error_from_storage_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let storage_err = StorageError::Io(io_err);
        let kernel_err: KernelError = storage_err.into();
        assert!(kernel_err.to_string().contains("Storage error"));
        assert!(kernel_err.to_string().contains("file missing"));
    }

    #[test]
    fn test_error_from_policy_missing_capability() {
        let policy_err = PolicyError::MissingCapability { capability: "fs.read".to_string() };
        let kernel_err: KernelError = policy_err.into();
        assert!(kernel_err.to_string().contains("fs.read"));
    }

    #[test]
    fn test_error_from_policy_expired() {
        let policy_err = PolicyError::ExpiredCapability { capability: "fs.write".to_string() };
        let kernel_err: KernelError = policy_err.into();
        assert!(kernel_err.to_string().contains("fs.write"));
    }

    #[test]
    fn test_error_from_policy_invalid_rule() {
        let policy_err = PolicyError::InvalidRule { message: "bad rule syntax".to_string() };
        let kernel_err: KernelError = policy_err.into();
        assert!(kernel_err.to_string().contains("bad rule syntax"));
    }

    #[test]
    fn test_error_from_provider_unavailable() {
        let provider_err = ProviderError::Unavailable { name: "gpt4".to_string() };
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("gpt4"));
    }

    #[test]
    fn test_error_from_provider_health_check_failed() {
        let provider_err = ProviderError::HealthCheckFailed {
            name: "ollama".to_string(),
            reason: "connection refused".to_string(),
        };
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("ollama"));
        assert!(kernel_err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_error_from_provider_inference_failed() {
        let provider_err = ProviderError::InferenceFailed("model error".to_string());
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("model error"));
    }

    #[test]
    fn test_error_from_provider_malformed_response() {
        let provider_err = ProviderError::MalformedResponse("invalid json".to_string());
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("invalid json"));
    }

    #[test]
    fn test_error_from_provider_cancelled() {
        let provider_err = ProviderError::Cancelled;
        let kernel_err: KernelError = provider_err.into();
        assert!(
            kernel_err.to_string().contains("cancelled")
                || kernel_err.to_string().contains("Cancelled")
        );
    }

    #[test]
    fn test_error_from_provider_http() {
        let provider_err = ProviderError::Http("502 Bad Gateway".to_string());
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("502"));
    }

    #[test]
    fn test_error_from_provider_no_role() {
        let provider_err = ProviderError::NoProviderForRole { role: "coder".to_string() };
        let kernel_err: KernelError = provider_err.into();
        assert!(kernel_err.to_string().contains("coder"));
    }

    #[test]
    fn test_error_from_tool_not_found() {
        let tool_err = ToolError::NotFound { name: "shell".to_string() };
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("shell"));
    }

    #[test]
    fn test_error_from_tool_execution_failed() {
        let tool_err = ToolError::ExecutionFailed {
            name: "fs".to_string(),
            reason: "permission denied".to_string(),
        };
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_error_from_tool_timeout() {
        let tool_err = ToolError::Timeout { name: "term".to_string(), timeout_secs: 5 };
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("5"));
    }

    #[test]
    fn test_error_from_tool_path_denied() {
        let tool_err = ToolError::PathDenied { path: "/etc/shadow".to_string() };
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("/etc/shadow"));
    }

    #[test]
    fn test_error_from_tool_command_denied() {
        let tool_err = ToolError::CommandDenied { command: "rm -rf /".to_string() };
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("rm -rf /"));
    }

    #[test]
    fn test_error_from_tool_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        let tool_err: ToolError = io_err.into();
        let kernel_err: KernelError = tool_err.into();
        assert!(kernel_err.to_string().contains("Tool error"));
    }

    #[test]
    fn test_error_from_task_invalid_transition() {
        let task_err =
            TaskError::InvalidTransition { from: "Received".into(), to: "Completed".into() };
        let kernel_err: KernelError = task_err.into();
        assert!(kernel_err.to_string().contains("Received"));
        assert!(kernel_err.to_string().contains("Completed"));
    }

    #[test]
    fn test_error_from_task_not_found() {
        let task_err = TaskError::NotFound { id: "task-123".to_string() };
        let kernel_err: KernelError = task_err.into();
        assert!(kernel_err.to_string().contains("task-123"));
    }

    #[test]
    fn test_error_from_task_queue_full() {
        let task_err = TaskError::QueueFull { max_depth: 100 };
        let kernel_err: KernelError = task_err.into();
        assert!(kernel_err.to_string().contains("100"));
    }

    #[test]
    fn test_error_from_task_duplicate() {
        let task_err = TaskError::Duplicate;
        let kernel_err: KernelError = task_err.into();
        assert!(
            kernel_err.to_string().contains("Duplicate")
                || kernel_err.to_string().contains("duplicate")
        );
    }

    #[test]
    fn test_error_from_task_cancelled() {
        let task_err = TaskError::Cancelled;
        let kernel_err: KernelError = task_err.into();
        assert!(
            kernel_err.to_string().contains("cancel")
                || kernel_err.to_string().contains("Cancelled")
        );
    }

    #[test]
    fn test_error_from_resource_insufficient_vram() {
        let res_err = ResourceError::InsufficientVram { needed_mb: 8000, available_mb: 2000 };
        let kernel_err: KernelError = res_err.into();
        assert!(kernel_err.to_string().contains("8000"));
        assert!(kernel_err.to_string().contains("2000"));
    }

    #[test]
    fn test_error_from_resource_insufficient_disk() {
        let res_err = ResourceError::InsufficientDisk { needed_gb: 50, available_gb: 10 };
        let kernel_err: KernelError = res_err.into();
        assert!(kernel_err.to_string().contains("50"));
        assert!(kernel_err.to_string().contains("10"));
    }

    #[test]
    fn test_error_from_resource_context_budget_exceeded() {
        let res_err =
            ResourceError::ContextBudgetExceeded { requested: 100000, max_allowed: 32000 };
        let kernel_err: KernelError = res_err.into();
        assert!(kernel_err.to_string().contains("100000"));
        assert!(kernel_err.to_string().contains("32000"));
    }

    #[test]
    fn test_error_from_serde() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad json").unwrap_err();
        let kernel_err: KernelError = json_err.into();
        assert!(kernel_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let kernel_err: KernelError = io_err.into();
        assert!(kernel_err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_config_error_from_toml() {
        let toml_err = toml::from_str::<AppConfig>("invalid").unwrap_err();
        let config_err: ConfigError = toml_err.into();
        assert!(
            config_err.to_string().contains("TOML parse error")
                || config_err.to_string().contains("parse")
        );
    }

    #[test]
    fn test_storage_error_from_io() {
        let io_err = std::io::Error::other("disk full");
        let storage_err: StorageError = io_err.into();
        assert!(storage_err.to_string().contains("Event store I/O error"));
    }

    #[test]
    fn test_storage_error_from_serde() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let storage_err: StorageError = json_err.into();
        assert!(storage_err.to_string().contains("Event serialization error"));
    }

    #[test]
    fn test_kernel_error_debug() {
        let err = KernelError::Task(TaskError::NotFound { id: "x".into() });
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Task"));
    }
}
