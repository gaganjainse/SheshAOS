use std::{fmt, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique task identifier (UUIDv7 for time-ordering)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Creates a new time-ordered TaskId using Uuidv7
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TaskId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u = Uuid::parse_str(s)?;
        Ok(Self(u))
    }
}

/// Priority levels for task scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// What the task contains
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum TaskInput {
    Text(String),
    Vision { text: String, image_paths: Vec<PathBuf> },
    Multi { parts: Vec<TaskInput> },
}

/// A request to execute a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskRequest {
    pub id: TaskId,
    pub input: TaskInput,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub parent_task_id: Option<TaskId>,
    pub metadata: serde_json::Value,
}

impl TaskRequest {
    /// Creates a new TaskRequest with default values for priority, timestamps, and metadata
    pub fn new(input: TaskInput) -> Self {
        Self {
            id: TaskId::new(),
            input,
            priority: Priority::default(),
            created_at: Utc::now(),
            parent_task_id: None,
            metadata: serde_json::Value::Null,
        }
    }
}

/// Outcome of a completed task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskOutcome {
    pub task_id: TaskId,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub completed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_display_and_creation() {
        let id = TaskId::new();
        assert_eq!(id.to_string(), id.0.to_string());

        let id2 = TaskId::new();
        assert!(id != id2);

        // UUIDv7 should be time-ordered
        assert!(id < id2);
    }

    #[test]
    fn test_priority_ordering() {
        // Enums with explicit ordering top-to-bottom or specific values
        // Note: Default partialOrd and Ord are top to bottom in declaration.
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_task_request_new() {
        let input = TaskInput::Text("Write a test".to_string());
        let req = TaskRequest::new(input.clone());
        assert_eq!(req.input, input);
        assert_eq!(req.priority, Priority::Normal);
        assert!(req.parent_task_id.is_none());
        assert_eq!(req.metadata, serde_json::Value::Null);
    }

    #[test]
    fn test_serde_roundtrip() {
        let input = TaskInput::Multi {
            parts: vec![
                TaskInput::Text("Find bugs".to_string()),
                TaskInput::Vision {
                    text: "Check this".to_string(),
                    image_paths: vec![PathBuf::from("/tmp/image.png")],
                },
            ],
        };
        let request = TaskRequest::new(input);

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: TaskRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request, deserialized);
    }
}
