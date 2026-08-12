//! Command snippet schema and storage for the Command Vault.

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stored shell command template with placeholder variables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSnippet {
    pub id: Uuid,
    pub name: String,
    pub template: String,
    pub description: String,
    pub tags: Vec<String>,
}

impl CommandSnippet {
    pub fn new(name: &str, template: &str, description: &str, tags: Vec<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.to_string(),
            template: template.to_string(),
            description: description.to_string(),
            tags,
        }
    }
}

/// Persistent store for command snippets.
pub struct VaultStore {
    path: PathBuf,
}

impl VaultStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load all stored command snippets.
    pub fn load_all(&self) -> Result<Vec<CommandSnippet>, std::io::Error> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut snippets = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(snippet) = serde_json::from_str::<CommandSnippet>(&line) {
                snippets.push(snippet);
            }
        }

        Ok(snippets)
    }

    /// Save a snippet to the vault.
    pub fn save(&self, snippet: &CommandSnippet) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let json = serde_json::to_string(snippet)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_snippet_creation_and_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("vault.jsonl");
        let store = VaultStore::new(store_path);

        let snippet = CommandSnippet::new(
            "docker-bash",
            "docker exec -it <container> /bin/bash",
            "Open bash inside a running container",
            vec!["docker".into(), "dev".into()],
        );

        store.save(&snippet).unwrap();
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "docker-bash");
    }

    #[test]
    fn test_snippet_new_generates_uuid() {
        let snippet = CommandSnippet::new("test-cmd", "echo hello", "A test command", vec![]);
        assert!(!snippet.id.is_nil());
    }

    #[test]
    fn test_snippet_new_fields() {
        let snippet = CommandSnippet::new(
            "deploy",
            "kubectl apply -f <manifest>",
            "Deploy to Kubernetes",
            vec!["k8s".into(), "prod".into()],
        );
        assert_eq!(snippet.name, "deploy");
        assert_eq!(snippet.template, "kubectl apply -f <manifest>");
        assert_eq!(snippet.description, "Deploy to Kubernetes");
        assert_eq!(snippet.tags, vec!["k8s", "prod"]);
    }

    #[test]
    fn test_snippet_new_empty_tags() {
        let snippet = CommandSnippet::new("cmd", "ls", "list files", vec![]);
        assert!(snippet.tags.is_empty());
    }

    #[test]
    fn test_snippet_equality() {
        let s1 = CommandSnippet::new("cmd", "ls", "desc", vec!["a".into()]);
        let s2 = CommandSnippet::new("cmd", "ls", "desc", vec!["a".into()]);
        // UUIDs will differ, so snippets with different IDs are not equal
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_vault_store_new() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store.jsonl");
        let store = VaultStore::new(store_path);
        assert_eq!(store.path, temp_dir.path().join("store.jsonl"));
    }

    #[test]
    fn test_vault_store_load_all_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nonexistent.jsonl");
        let store = VaultStore::new(store_path);
        let loaded = store.load_all().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_vault_store_load_all_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("empty.jsonl");
        std::fs::write(&store_path, "").unwrap();
        let store = VaultStore::new(store_path);
        let loaded = store.load_all().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_vault_store_load_all_invalid_json_lines() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("invalid.jsonl");
        std::fs::write(&store_path, "not json\nalso not json\n").unwrap();
        let store = VaultStore::new(store_path);
        let loaded = store.load_all().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_vault_store_load_all_mixed_valid_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("mixed.jsonl");

        let snippet = CommandSnippet::new("cmd", "echo hi", "desc", vec![]);
        let valid_json = serde_json::to_string(&snippet).unwrap();

        std::fs::write(&store_path, format!("invalid\n{}\ninvalid2\n", valid_json)).unwrap();
        let store = VaultStore::new(store_path);
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "cmd");
    }

    #[test]
    fn test_vault_store_save_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nested/dir/vault.jsonl");
        let store = VaultStore::new(store_path.clone());

        let snippet = CommandSnippet::new("cmd", "echo hi", "desc", vec![]);
        store.save(&snippet).unwrap();
        assert!(store_path.exists());
    }

    #[test]
    fn test_vault_store_multiple_saves() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("multi.jsonl");
        let store = VaultStore::new(store_path);

        for i in 0..5 {
            let snippet = CommandSnippet::new(
                &format!("cmd{}", i),
                &format!("echo {}", i),
                &format!("Command {}", i),
                vec![],
            );
            store.save(&snippet).unwrap();
        }

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 5);
    }

    #[test]
    fn test_vault_store_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("roundtrip.jsonl");
        let store = VaultStore::new(store_path);

        let original =
            CommandSnippet::new("roundtrip", "cat <file>", "Roundtrip test", vec!["test".into()]);
        store.save(&original).unwrap();

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "roundtrip");
        assert_eq!(loaded[0].template, "cat <file>");
        assert_eq!(loaded[0].description, "Roundtrip test");
        assert_eq!(loaded[0].tags, vec!["test"]);
    }

    #[test]
    fn test_vault_store_append_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("append.jsonl");
        let store = VaultStore::new(store_path.clone());

        let s1 = CommandSnippet::new("cmd1", "echo 1", "first", vec![]);
        store.save(&s1).unwrap();

        let s2 = CommandSnippet::new("cmd2", "echo 2", "second", vec![]);
        store.save(&s2).unwrap();

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_vault_store_whitespace_lines_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("whitespace.jsonl");

        let snippet = CommandSnippet::new("cmd", "echo", "desc", vec![]);
        let json_str = serde_json::to_string(&snippet).unwrap();

        std::fs::write(&store_path, format!("\n  \n{}\n\t\n", json_str)).unwrap();
        let store = VaultStore::new(store_path);
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_command_snippet_serialization() {
        let snippet = CommandSnippet::new(
            "serialize",
            "docker run <image>",
            "Serialize test",
            vec!["docker".into()],
        );
        let json = serde_json::to_string(&snippet).unwrap();
        let decoded: CommandSnippet = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, snippet.name);
        assert_eq!(decoded.template, snippet.template);
        assert_eq!(decoded.tags, snippet.tags);
    }

    #[test]
    fn test_command_snippet_clone() {
        let snippet =
            CommandSnippet::new("clone-test", "cmd", "desc", vec!["a".into(), "b".into()]);
        let cloned = snippet.clone();
        assert_eq!(cloned.name, snippet.name);
        assert_eq!(cloned.template, snippet.template);
        assert_eq!(cloned.tags, snippet.tags);
        assert_eq!(cloned.id, snippet.id);
    }
}
