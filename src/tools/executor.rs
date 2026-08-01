use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ToolError;

/// A request to execute a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Name of the tool to invoke.
    pub tool_name: String,
    /// Arguments as a JSON value.
    pub arguments: serde_json::Value,
}

/// The result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool succeeded.
    pub success: bool,
    /// Output text.
    pub output: String,
    /// Optional structured data.
    pub data: Option<serde_json::Value>,
}

/// Trait that all tool executors must implement.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// The name of this tool.
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Whether this tool performs destructive/write operations.
    fn is_destructive(&self) -> bool;

    /// Execute the tool with the given request.
    async fn execute(&self, request: &ToolRequest) -> Result<ToolResult, ToolError>;
}
