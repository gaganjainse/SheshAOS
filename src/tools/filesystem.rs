use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::executor::{ToolExecutor, ToolRequest, ToolResult};
use crate::error::ToolError;

/// Filesystem operations tool.
pub struct FilesystemTool {
    allowed_paths: Vec<PathBuf>,
    denied_patterns: Vec<String>,
}

impl FilesystemTool {
    pub fn new(allowed_paths: Vec<PathBuf>, denied_patterns: Vec<String>) -> Self {
        Self { allowed_paths, denied_patterns }
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.denied_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        for allowed in &self.allowed_paths {
            let abs_allowed = allowed.canonicalize().unwrap_or_else(|_| allowed.to_path_buf());
            if abs_path.starts_with(&abs_allowed) {
                return true;
            }
        }
        false
    }
}

#[async_trait]
impl ToolExecutor for FilesystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Filesystem operations tool for reading, writing, and managing files"
    }

    fn is_destructive(&self) -> bool {
        // Technically destructive for write/delete, but trait doesn't take context
        true
    }

    async fn execute(&self, request: &ToolRequest) -> Result<ToolResult, ToolError> {
        let action = request.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "read_file" => {
                let path_str =
                    request.arguments.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'path' argument".to_string(),
                        }
                    })?;
                let path = Path::new(path_str);

                if !self.is_path_allowed(path) {
                    return Err(ToolError::PathDenied { path: path_str.to_string() });
                }

                let content = tokio::fs::read_to_string(path).await?;
                Ok(ToolResult { success: true, output: content, data: None })
            }
            "write_file" => {
                let path_str =
                    request.arguments.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'path' argument".to_string(),
                        }
                    })?;
                let content =
                    request.arguments.get("content").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'content' argument".to_string(),
                        }
                    })?;
                let path = Path::new(path_str);

                if !self.is_path_allowed(path) {
                    return Err(ToolError::PathDenied { path: path_str.to_string() });
                }

                tokio::fs::write(path, content).await?;
                Ok(ToolResult {
                    success: true,
                    output: "File written successfully".to_string(),
                    data: None,
                })
            }
            "list_dir" => {
                let path_str =
                    request.arguments.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'path' argument".to_string(),
                        }
                    })?;
                let path = Path::new(path_str);

                if !self.is_path_allowed(path) {
                    return Err(ToolError::PathDenied { path: path_str.to_string() });
                }

                let mut entries = tokio::fs::read_dir(path).await?;
                let mut output = String::new();
                while let Some(entry) = entries.next_entry().await? {
                    output.push_str(&format!("{}\n", entry.file_name().to_string_lossy()));
                }

                Ok(ToolResult { success: true, output, data: None })
            }
            "delete_file" => {
                let path_str =
                    request.arguments.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::ExecutionFailed {
                            name: self.name().to_string(),
                            reason: "Missing 'path' argument".to_string(),
                        }
                    })?;
                let path = Path::new(path_str);

                if !self.is_path_allowed(path) {
                    return Err(ToolError::PathDenied { path: path_str.to_string() });
                }

                tokio::fs::remove_file(path).await?;
                Ok(ToolResult {
                    success: true,
                    output: "File deleted successfully".to_string(),
                    data: None,
                })
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
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_filesystem_tool() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_paths = vec![temp_dir.path().to_path_buf()];
        let denied_patterns = vec![".env".to_string()];

        let tool = FilesystemTool::new(allowed_paths, denied_patterns);

        let write_req = ToolRequest {
            tool_name: "filesystem".to_string(),
            arguments: json!({
                "action": "write_file",
                "path": temp_dir.path().join("test.txt").to_str().unwrap(),
                "content": "hello world"
            }),
        };
        let write_res = tool.execute(&write_req).await.unwrap();
        assert!(write_res.success);

        let read_req = ToolRequest {
            tool_name: "filesystem".to_string(),
            arguments: json!({
                "action": "read_file",
                "path": temp_dir.path().join("test.txt").to_str().unwrap()
            }),
        };
        let read_res = tool.execute(&read_req).await.unwrap();
        assert!(read_res.success);
        assert_eq!(read_res.output, "hello world");

        let deny_req = ToolRequest {
            tool_name: "filesystem".to_string(),
            arguments: json!({
                "action": "read_file",
                "path": temp_dir.path().join(".env").to_str().unwrap()
            }),
        };
        assert!(tool.execute(&deny_req).await.is_err());
    }
}
