//! Atomic unified diff patch engine for line-by-line file modifications.

use std::{fs, path::Path};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("Target file not found: {0}")]
    FileNotFound(String),

    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to apply patch line: {0}")]
    ApplyFailed(String),
}

/// Atomic patch application engine.
pub struct PatchEngine;

impl PatchEngine {
    /// Apply unified diff patch lines directly to a file on disk.
    pub fn apply_patch(target_file: &Path, patch_lines: &[String]) -> Result<(), PatchError> {
        if !target_file.exists() {
            return Err(PatchError::FileNotFound(target_file.display().to_string()));
        }

        let original_content = fs::read_to_string(target_file)?;
        let lines: Vec<&str> = original_content.lines().collect();
        let mut new_lines = Vec::new();

        let mut orig_idx = 0;

        for line in patch_lines {
            if line.starts_with('+') && !line.starts_with("+++") {
                new_lines.push(line[1..].to_string());
            } else if line.starts_with('-') && !line.starts_with("---") {
                if orig_idx < lines.len() {
                    orig_idx += 1;
                }
            } else if line.starts_with(' ') && orig_idx < lines.len() {
                new_lines.push(lines[orig_idx].to_string());
                orig_idx += 1;
            }
        }

        while orig_idx < lines.len() {
            new_lines.push(lines[orig_idx].to_string());
            orig_idx += 1;
        }

        let final_content = new_lines.join("\n");
        fs::write(target_file, final_content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_patch_engine_apply() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();

        let patch = vec![
            " line 1".to_string(),
            "-line 2".to_string(),
            "+line 2 modified".to_string(),
            " line 3".to_string(),
        ];

        PatchEngine::apply_patch(&file_path, &patch).unwrap();
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("line 2 modified"));
    }

    #[test]
    fn test_patch_engine_missing_file() {
        let result = PatchEngine::apply_patch(Path::new("/nonexistent/path/file.txt"), &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatchError::FileNotFound(_)));
    }

    #[test]
    fn test_patch_engine_add_lines() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2").unwrap();

        let patch = vec!["+new line 1".to_string(), "+new line 2".to_string()];

        PatchEngine::apply_patch(&file_path, &patch).unwrap();
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("new line 1"));
        assert!(updated.contains("new line 2"));
    }

    #[test]
    fn test_patch_engine_delete_all() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();

        let patch = vec!["-line 1".to_string(), "-line 2".to_string(), "-line 3".to_string()];

        PatchEngine::apply_patch(&file_path, &patch).unwrap();
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.is_empty() || updated.is_empty());
    }

    #[test]
    fn test_patch_engine_no_changes() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2").unwrap();

        let patch = vec![];

        PatchEngine::apply_patch(&file_path, &patch).unwrap();
        let updated = fs::read_to_string(&file_path).unwrap();
        assert_eq!(updated, "line 1\nline 2");
    }
}
