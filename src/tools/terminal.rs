use std::time::Duration;

use async_trait::async_trait;
use tokio::{process::Command, time::timeout};

use super::executor::{ToolExecutor, ToolRequest, ToolResult};
use crate::error::ToolError;

/// Sandboxed terminal command execution.
pub struct TerminalTool {
    timeout_secs: u64,
    denied_prefixes: Vec<String>,
}

impl TerminalTool {
    pub fn new(timeout_secs: u64, denied_prefixes: Vec<String>) -> Self {
        Self { timeout_secs, denied_prefixes }
    }

    fn is_command_denied(&self, command: &str) -> bool {
        self.denied_prefixes.iter().any(|prefix| command.starts_with(prefix))
    }
}

#[async_trait]
impl ToolExecutor for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Sandboxed terminal command execution"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ToolRequest) -> Result<ToolResult, ToolError> {
        let command =
            request.arguments.get("command").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolError::ExecutionFailed {
                    name: self.name().to_string(),
                    reason: "Missing 'command' argument".to_string(),
                }
            })?;

        if self.is_command_denied(command) {
            return Err(ToolError::CommandDenied { command: command.to_string() });
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        let future = cmd.output();

        match timeout(Duration::from_secs(self.timeout_secs), future).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

                Ok(ToolResult {
                    success: output.status.success(),
                    output: if output.status.success() { stdout } else { stderr },
                    data: None,
                })
            }
            Ok(Err(e)) => Err(ToolError::Io(e)),
            Err(_) => Err(ToolError::Timeout {
                name: self.name().to_string(),
                timeout_secs: self.timeout_secs,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_terminal_tool() {
        let tool = TerminalTool::new(5, vec!["rm -rf".to_string()]);

        // test denied
        let req_denied = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "rm -rf /" }),
        };

        let err = tool.execute(&req_denied).await.unwrap_err();
        match err {
            ToolError::CommandDenied { command } => assert_eq!(command, "rm -rf /"),
            _ => panic!("Expected CommandDenied"),
        }

        // test allowed
        let req_allowed = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "echo test" }),
        };

        let res = tool.execute(&req_allowed).await.unwrap();
        assert!(res.success);
        assert_eq!(res.output.trim(), "test");
    }
}
