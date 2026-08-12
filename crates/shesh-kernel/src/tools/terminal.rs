use std::time::Duration;

use async_trait::async_trait;
use tokio::{process::Command, time::timeout};

use super::executor::{ToolExecutor, ToolRequest, ToolResult};
use crate::error::ToolError;

/// Sandboxed terminal command execution.
pub struct TerminalTool {
    timeout_secs: u64,
    denied_prefixes: Vec<String>,
    /// If true, refuse to run commands when bwrap is unavailable.
    require_sandbox: bool,
}

impl TerminalTool {
    pub fn new(timeout_secs: u64, denied_prefixes: Vec<String>) -> Self {
        Self { timeout_secs, denied_prefixes, require_sandbox: false }
    }

    /// Resolve the bwrap binary through PATH.
    fn resolve_bwrap() -> Option<std::path::PathBuf> {
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                let candidate = std::path::Path::new(dir).join("bwrap");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        None
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

        let bwrap_path = Self::resolve_bwrap();
        let has_bwrap = bwrap_path.is_some();

        if self.require_sandbox && !has_bwrap {
            return Err(ToolError::ExecutionFailed {
                name: self.name().to_string(),
                reason: "Sandbox required but bwrap not found".to_string(),
            });
        }

        let mut cmd = if let Some(bwrap_bin) = bwrap_path {
            let mut bwrap = Command::new(bwrap_bin);
            bwrap
                .arg("--ro-bind")
                .arg("/")
                .arg("/")
                .arg("--dev")
                .arg("/dev")
                .arg("--proc")
                .arg("/proc")
                .arg("--tmpfs")
                .arg("/tmp")
                .arg("--unshare-all")
                .arg("--")
                .arg("sh")
                .arg("-c")
                .arg(command);
            bwrap
        } else {
            let mut sh = Command::new("sh");
            sh.arg("-c").arg(command);
            sh
        };

        cmd.kill_on_drop(true);
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

    #[tokio::test]
    async fn test_terminal_missing_command() {
        let tool = TerminalTool::new(5, vec![]);

        let req = ToolRequest { tool_name: "terminal".to_string(), arguments: json!({}) };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::ExecutionFailed { reason, .. } => {
                assert!(reason.contains("Missing 'command' argument"))
            }
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[tokio::test]
    async fn test_terminal_command_denied_prefix() {
        let tool = TerminalTool::new(5, vec!["sudo".to_string(), "dd".to_string()]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "sudo apt update" }),
        };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::CommandDenied { command } => assert_eq!(command, "sudo apt update"),
            _ => panic!("Expected CommandDenied"),
        }
    }

    #[tokio::test]
    async fn test_terminal_command_denied_partial_match() {
        let tool = TerminalTool::new(5, vec!["rm -rf".to_string()]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "rm -rf /tmp/foo" }),
        };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::CommandDenied { command } => assert_eq!(command, "rm -rf /tmp/foo"),
            _ => panic!("Expected CommandDenied"),
        }
    }

    #[tokio::test]
    async fn test_terminal_no_denied_prefixes() {
        let tool = TerminalTool::new(5, vec![]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "echo hello" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
    }

    #[tokio::test]
    async fn test_terminal_timeout() {
        let tool = TerminalTool::new(1, vec![]); // 1 second timeout

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "sleep 10" }),
        };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::Timeout { timeout_secs, .. } => assert_eq!(timeout_secs, 1),
            _ => panic!("Expected Timeout, got: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_terminal_command_fails_nonzero_exit() {
        let tool = TerminalTool::new(5, vec![]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "exit 1" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(!res.success);
    }

    #[tokio::test]
    async fn test_terminal_empty_output() {
        let tool = TerminalTool::new(5, vec![]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "true" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
        assert!(res.output.is_empty() || res.output.trim().is_empty());
    }

    #[tokio::test]
    async fn test_terminal_with_stderr() {
        let tool = TerminalTool::new(5, vec![]);

        // A command that writes to stderr but succeeds
        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "echo out" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("out"));
    }

    #[tokio::test]
    async fn test_terminal_no_match_denied_prefix() {
        let tool = TerminalTool::new(5, vec!["rm -rf /".to_string()]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "echo safe" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
        assert_eq!(res.output.trim(), "safe");
    }

    #[tokio::test]
    async fn test_terminal_exact_denied_prefix() {
        let tool = TerminalTool::new(5, vec!["ls".to_string()]);

        let req = ToolRequest {
            tool_name: "terminal".to_string(),
            arguments: json!({ "command": "ls" }),
        };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::CommandDenied { command } => assert_eq!(command, "ls"),
            _ => panic!("Expected CommandDenied"),
        }
    }
}
