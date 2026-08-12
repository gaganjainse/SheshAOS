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

impl TaskInput {
    /// Returns a text representation of the input, preserving semantic structure
    /// for Multi variants by joining parts with a separator.
    pub fn text(&self) -> String {
        match self {
            TaskInput::Text(t) => t.clone(),
            TaskInput::Vision { text, .. } => text.clone(),
            TaskInput::Multi { parts } => {
                parts.iter().map(|p| p.text()).collect::<Vec<_>>().join("\n---\n")
            }
        }
    }
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
    /// If true, the task requires user confirmation before proceeding
    pub requires_confirmation: bool,
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

    #[test]
    fn test_task_id_default() {
        let id = TaskId::default();
        assert_eq!(id.to_string(), id.0.to_string());
    }

    #[test]
    fn test_task_id_from_str() {
        let uuid_str = "01958104-7a9c-7a5a-9c5a-3a5a7a9c7a5a";
        let id: TaskId = uuid_str.parse().expect("should parse");
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_task_id_from_str_invalid() {
        let result: Result<TaskId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_task_id_ordering_v7() {
        let id1 = TaskId::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = TaskId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_priority_all_variants_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
        assert!(Priority::Low < Priority::High);
        assert!(Priority::Low < Priority::Critical);
        assert!(Priority::Normal < Priority::Critical);
    }

    #[test]
    fn test_priority_equality() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_ne!(Priority::Low, Priority::Normal);
    }

    #[test]
    fn test_priority_default_is_normal() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_task_input_text() {
        let input = TaskInput::Text("hello".to_string());
        match input {
            TaskInput::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_task_input_vision() {
        let input = TaskInput::Vision {
            text: "describe this".to_string(),
            image_paths: vec![PathBuf::from("/tmp/a.png"), PathBuf::from("/tmp/b.png")],
        };
        match input {
            TaskInput::Vision { text, image_paths } => {
                assert_eq!(text, "describe this");
                assert_eq!(image_paths.len(), 2);
            }
            _ => panic!("Expected Vision variant"),
        }
    }

    #[test]
    fn test_task_input_multi() {
        let input = TaskInput::Multi {
            parts: vec![TaskInput::Text("a".into()), TaskInput::Text("b".into())],
        };
        match input {
            TaskInput::Multi { parts } => assert_eq!(parts.len(), 2),
            _ => panic!("Expected Multi variant"),
        }
    }

    #[test]
    fn test_task_input_serde_text() {
        let input = TaskInput::Text("test".into());
        let json = serde_json::to_string(&input).unwrap();
        let back: TaskInput = serde_json::from_str(&json).unwrap();
        match back {
            TaskInput::Text(t) => assert_eq!(t, "test"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_task_input_serde_vision() {
        let input =
            TaskInput::Vision { text: "desc".into(), image_paths: vec![PathBuf::from("/img.png")] };
        let json = serde_json::to_string(&input).unwrap();
        let back: TaskInput = serde_json::from_str(&json).unwrap();
        match back {
            TaskInput::Vision { text, image_paths } => {
                assert_eq!(text, "desc");
                assert_eq!(image_paths, vec![PathBuf::from("/img.png")]);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_task_input_serde_multi() {
        let input = TaskInput::Multi { parts: vec![TaskInput::Text("a".into())] };
        let json = serde_json::to_string(&input).unwrap();
        let back: TaskInput = serde_json::from_str(&json).unwrap();
        match back {
            TaskInput::Multi { parts } => assert_eq!(parts.len(), 1),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_task_request_fields() {
        let input = TaskInput::Text("task".into());
        let req = TaskRequest::new(input.clone());
        assert_eq!(req.input, input);
        assert_eq!(req.priority, Priority::Normal);
        assert!(req.parent_task_id.is_none());
        assert_eq!(req.metadata, serde_json::Value::Null);
        assert!(req.created_at <= Utc::now());
    }

    #[test]
    fn test_task_outcome_construction() {
        let task_id = TaskId::new();
        let outcome = TaskOutcome {
            task_id,
            success: true,
            output: Some("done".into()),
            error: None,
            completed_at: Utc::now(),
            requires_confirmation: false,
        };
        assert!(outcome.success);
        assert_eq!(outcome.output, Some("done".into()));
        assert!(outcome.error.is_none());
    }

    #[test]
    fn test_task_outcome_failure() {
        let task_id = TaskId::new();
        let outcome = TaskOutcome {
            task_id,
            success: false,
            output: None,
            error: Some("boom".into()),
            completed_at: Utc::now(),
            requires_confirmation: false,
        };
        assert!(!outcome.success);
        assert!(outcome.output.is_none());
        assert_eq!(outcome.error, Some("boom".into()));
    }

    #[test]
    fn test_task_outcome_serde() {
        let task_id = TaskId::new();
        let outcome = TaskOutcome {
            task_id,
            success: true,
            output: Some("result".into()),
            error: None,
            completed_at: Utc::now(),
            requires_confirmation: false,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let back: TaskOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome.task_id, back.task_id);
        assert_eq!(outcome.success, back.success);
    }

    #[test]
    fn test_task_request_with_high_priority() {
        let req = TaskRequest {
            id: TaskId::new(),
            input: TaskInput::Text("urgent".into()),
            priority: Priority::Critical,
            created_at: Utc::now(),
            parent_task_id: None,
            metadata: serde_json::json!({"tag": "urgent"}),
        };
        assert_eq!(req.priority, Priority::Critical);
        assert_eq!(req.metadata["tag"], "urgent");
    }

    #[test]
    fn test_task_request_with_parent() {
        let parent_id = TaskId::new();
        let req = TaskRequest {
            id: TaskId::new(),
            input: TaskInput::Text("subtask".into()),
            priority: Priority::Normal,
            created_at: Utc::now(),
            parent_task_id: Some(parent_id),
            metadata: serde_json::Value::Null,
        };
        assert_eq!(req.parent_task_id, Some(parent_id));
    }
}
