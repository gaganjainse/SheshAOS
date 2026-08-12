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
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_git_tool() {
        let tool = GitTool::new(env::current_dir().unwrap());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "status" }) };

        let res = tool.execute(&req).await.unwrap();
        // Since we are running in a real environment which may or may not be a git repo
        // we just assert that we got some output from git.
        assert!(!res.output.is_empty());
    }

    #[tokio::test]
    async fn test_git_diff() {
        let temp_dir = TempDir::new().unwrap();
        // Initialize a git repo
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "diff" }) };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success); // diff with no changes returns empty but succeeds
    }

    #[tokio::test]
    async fn test_git_diff_staged() {
        let temp_dir = TempDir::new().unwrap();
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let _ = Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .await;

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req = ToolRequest {
            tool_name: "git".to_string(),
            arguments: json!({ "action": "diff", "staged": true }),
        };
        let res = tool.execute(&req).await.unwrap();
        // staged diff should show the added file
        assert!(res.success);
    }

    #[tokio::test]
    async fn test_git_log() {
        let temp_dir = TempDir::new().unwrap();
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let _ = Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req = ToolRequest {
            tool_name: "git".to_string(),
            arguments: json!({ "action": "log", "count": 1 }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("initial"));
    }

    #[tokio::test]
    async fn test_git_add() {
        let temp_dir = TempDir::new().unwrap();
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req = ToolRequest {
            tool_name: "git".to_string(),
            arguments: json!({ "action": "add", "paths": ["test.txt"] }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
    }

    #[tokio::test]
    async fn test_git_commit() {
        let temp_dir = TempDir::new().unwrap();
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let _ = Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .await;

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req = ToolRequest {
            tool_name: "git".to_string(),
            arguments: json!({ "action": "commit", "message": "add test file" }),
        };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("add test file") || res.output.contains("1 file"));
    }

    #[tokio::test]
    async fn test_git_unknown_action() {
        let temp_dir = TempDir::new().unwrap();
        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "push" }) };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::ExecutionFailed { reason, .. } => assert!(reason.contains("Unknown action")),
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[tokio::test]
    async fn test_git_missing_message_for_commit() {
        let temp_dir = TempDir::new().unwrap();
        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "commit" }) };
        let err = tool.execute(&req).await.unwrap_err();
        match err {
            ToolError::ExecutionFailed { reason, .. } => {
                assert!(reason.contains("Missing 'message' argument"))
            }
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[tokio::test]
    async fn test_git_in_non_repo_directory() {
        let temp_dir = TempDir::new().unwrap();
        let tool = GitTool::new(temp_dir.path().to_path_buf());

        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "status" }) };
        let res = tool.execute(&req).await.unwrap();
        // In a non-git directory, git status will fail
        assert!(!res.success || res.output.contains("fatal") || res.output.contains("not a git"));
    }

    #[tokio::test]
    async fn test_git_log_default_count() {
        let temp_dir = TempDir::new().unwrap();
        let _ = Command::new("git").arg("init").current_dir(temp_dir.path()).output().await;
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let _ = Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .await;
        let _ = Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp_dir.path())
            .output()
            .await;

        let tool = GitTool::new(temp_dir.path().to_path_buf());

        // Test default count (should use 10)
        let req =
            ToolRequest { tool_name: "git".to_string(), arguments: json!({ "action": "log" }) };
        let res = tool.execute(&req).await.unwrap();
        assert!(res.success);
    }
}
