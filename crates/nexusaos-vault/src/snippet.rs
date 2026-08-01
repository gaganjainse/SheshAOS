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
}
