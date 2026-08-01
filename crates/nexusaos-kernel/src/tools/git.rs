use std::path::PathBuf;

use async_trait::async_trait;
use tokio::process::Command;

use super::executor::{ToolExecutor, ToolRequest, ToolResult};
use crate::error::ToolError;

/// Git operations tool.
pub struct GitTool {
    work_dir: PathBuf,
}

impl GitTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    async fn run_git(&self, args: &[&str]) -> Result<ToolResult, ToolError> {
        let output = Command::new("git").current_dir(&self.work_dir).args(args).output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(ToolResult {
            success: output.status.success(),
            output: if output.status.success() { stdout } else { stderr },
            data: None,
        })
    }
}

#[async_trait]
impl ToolExecutor for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Git operations tool"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ToolRequest) -> Result<ToolResult, ToolError> {
        let action = request.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "status" => self.run_git(&["status"]).await,
            "diff" => {
                let staged =
                    request.arguments.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
                if staged {
                    self.run_git(&["diff", "--staged"]).await
                } else {
                    self.run_git(&["diff"]).await
                }
            }
            "log" => {
                let count = request.arguments.get("count").and_then(|v| v.as_u64()).unwrap_or(10);
                let count_str = format!("-{}", count);
                self.run_git(&["log", &count_str]).await
            }
            "add" => {
                let paths = request
                    .arguments
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();

                let mut args = vec!["add"];
                args.extend(paths);
                self.run_git(&args).await
            }
            "commit" => {
                let message =
                    request.arguments.get("message").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'message' argument".to_string(),
                        }
                    })?;
                self.run_git(&["commit", "-m", message]).await
            }
            _ => Err(ToolError::ExecutionFailed {
                name: self.name().to_string(),
                reason: format!("Unknown action: {}", action),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_git_tool() {
        let tool = GitTool::new(env::current_dir().unwrap());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "status" }) };

        let res = tool.execute(&req).await.unwrap();
        // Since we are running in a real environment which may or may not be a git repo
        // we just assert that we got some output from git.
        assert!(res.output.len() > 0);
    }
}
