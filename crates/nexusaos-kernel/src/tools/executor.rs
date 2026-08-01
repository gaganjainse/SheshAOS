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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_request_construction() {
        let req = ToolRequest {
            tool_name: "test_tool".to_string(),
            arguments: json!({"key": "value"}),
        };
        assert_eq!(req.tool_name, "test_tool");
        assert_eq!(req.arguments["key"], "value");
    }

    #[test]
    fn test_tool_request_serde() {
        let req = ToolRequest {
            tool_name: "fs".to_string(),
            arguments: json!({"action": "read", "path": "/tmp/test"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: ToolRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.tool_name, back.tool_name);
        assert_eq!(req.arguments, back.arguments);
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult {
            success: true,
            output: "done".into(),
            data: Some(json!({"bytes": 1024})),
        };
        assert!(result.success);
        assert_eq!(result.output, "done");
        assert!(result.data.is_some());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult {
            success: false,
            output: "error msg".into(),
            data: None,
        };
        assert!(!result.success);
        assert_eq!(result.output, "error msg");
        assert!(result.data.is_none());
    }

    #[test]
    fn test_tool_result_serde() {
        let result = ToolResult {
            success: true,
            output: "output".into(),
            data: Some(json!({"key": "val"})),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.success, back.success);
        assert_eq!(result.output, back.output);
    }

    #[test]
    fn test_tool_result_empty_output() {
        let result = ToolResult {
            success: true,
            output: String::new(),
            data: None,
        };
        assert!(result.output.is_empty());
    }
}
