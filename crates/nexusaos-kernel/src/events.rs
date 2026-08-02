// src/events.rs - Event sourcing types for NexusAOS
// All types derive Debug, Clone, Serialize, Deserialize

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::TaskId;

pub const MAX_TOOL_OUTPUT_SIZE: usize = 1_048_576;

/// Unique event identifier (UUIDv7)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Create a new EventId using UUIDv7
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Monotonically increasing sequence number within the event store
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SequenceNumber(pub u64);

/// Categories of events
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    // Task lifecycle
    TaskCreated,
    TaskClassified,
    TaskStateChanged,
    // Model interactions
    ModelRequested,
    ModelResponded,
    ModelFailed,
    // Tool interactions
    ToolRequested,
    ToolCompleted,
    ToolFailed,
    // Policy
    PolicyChecked,
    PolicyDenied,
    ConfirmationRequested,
    ConfirmationGranted,
    ConfirmationDenied,
    // System
    CheckpointCreated,
    SnapshotCreated,
    SystemStarted,
    SystemShutdown,
    Error,
}

/// The payload of an event — what actually happened
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPayload {
    TaskCreated { request: serde_json::Value },
    StateChanged { from: String, to: String },
    ModelRequest { role: String, prompt_tokens: usize, context_budget: usize },
    ModelResponse { role: String, response_tokens: usize, content: String },
    ModelFailure { role: String, error: String },
    ToolCall { tool_name: String, arguments: serde_json::Value },
    ToolResult { tool_name: String, success: bool, output: String },
    PolicyCheck { action: String, decision: String, reason: Option<String> },
    Checkpoint { snapshot_path: String },
    SystemEvent { message: String },
    ErrorEvent { message: String, details: Option<String> },
}

/// Metadata attached to every event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: String,
    pub correlation_id: Option<String>,
}

/// A single event — the atomic unit of the event store
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub task_id: Option<TaskId>,
    pub sequence: SequenceNumber,
    pub kind: EventKind,
    pub payload: EventPayload,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    /// Creates a new event associated with a task
    pub fn new(task_id: TaskId, kind: EventKind, payload: EventPayload, source: String) -> Self {
        Self {
            id: EventId::new(),
            task_id: Some(task_id),
            sequence: SequenceNumber(0),
            kind,
            payload,
            metadata: EventMetadata { source, correlation_id: None },
            timestamp: Utc::now(),
        }
    }

    /// Creates a new system-level event without a task
    pub fn system(kind: EventKind, payload: EventPayload, source: String) -> Self {
        Self {
            id: EventId::new(),
            task_id: None,
            sequence: SequenceNumber(0),
            kind,
            payload,
            metadata: EventMetadata { source, correlation_id: None },
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_event_id_display() {
        let id = EventId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36); // UUID string format length
    }

    #[test]
    fn test_sequence_ordering() {
        let s1 = SequenceNumber(1);
        let s2 = SequenceNumber(2);
        assert!(s1 < s2);
        assert_eq!(s1, SequenceNumber(1));
    }

    #[test]
    fn test_event_creation() {
        let task_id = TaskId::new();
        let event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: json!({ "prompt": "test" }) },
            "test_source".to_string(),
        );

        assert_eq!(event.task_id, Some(task_id));
        assert_eq!(event.metadata.source, "test_source");
        assert_eq!(event.sequence, SequenceNumber(0));
    }

    #[test]
    fn test_system_event_creation() {
        let event = Event::system(
            EventKind::SystemStarted,
            EventPayload::SystemEvent { message: "started".to_string() },
            "sys".to_string(),
        );

        assert_eq!(event.task_id, None);
        assert_eq!(event.metadata.source, "sys");
    }

    #[test]
    fn test_serde_round_trip() {
        let payloads = vec![
            EventPayload::TaskCreated { request: json!({"k": "v"}) },
            EventPayload::StateChanged { from: "A".to_string(), to: "B".to_string() },
            EventPayload::ModelRequest {
                role: "user".to_string(),
                prompt_tokens: 10,
                context_budget: 100,
            },
            EventPayload::ModelResponse {
                role: "assistant".to_string(),
                response_tokens: 20,
                content: "ok".to_string(),
            },
            EventPayload::ModelFailure { role: "system".to_string(), error: "timeout".to_string() },
            EventPayload::ToolCall { tool_name: "ls".to_string(), arguments: json!({}) },
            EventPayload::ToolResult {
                tool_name: "ls".to_string(),
                success: true,
                output: ".".to_string(),
            },
            EventPayload::PolicyCheck {
                action: "read".to_string(),
                decision: "allow".to_string(),
                reason: None,
            },
            EventPayload::Checkpoint { snapshot_path: "/tmp/a".to_string() },
            EventPayload::SystemEvent { message: "msg".to_string() },
            EventPayload::ErrorEvent {
                message: "err".to_string(),
                details: Some("dbg".to_string()),
            },
        ];

        for payload in payloads {
            let serialized = serde_json::to_string(&payload).unwrap();
            let deserialized: EventPayload = serde_json::from_str(&serialized).unwrap();
            assert_eq!(payload, deserialized);
        }
    }

    #[test]
    fn test_event_id_default() {
        let id = EventId::default();
        assert_eq!(id.to_string().len(), 36);
    }

    #[test]
    fn test_event_id_new_unique() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_id_ordering() {
        let id1 = EventId::new();
        // Sleep briefly to ensure different timestamps for UUIDv7
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = EventId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_event_metadata_correlation_id() {
        let task_id = TaskId::new();
        let event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: json!({}) },
            "src".to_string(),
        );
        // Default correlation_id is None
        assert!(event.metadata.correlation_id.is_none());
    }

    #[test]
    fn test_event_kind_variants() {
        // Ensure all EventKind variants are constructible and debuggable
        let kinds = vec![
            EventKind::TaskCreated,
            EventKind::TaskClassified,
            EventKind::TaskStateChanged,
            EventKind::ModelRequested,
            EventKind::ModelResponded,
            EventKind::ModelFailed,
            EventKind::ToolRequested,
            EventKind::ToolCompleted,
            EventKind::ToolFailed,
            EventKind::PolicyChecked,
            EventKind::PolicyDenied,
            EventKind::ConfirmationRequested,
            EventKind::ConfirmationGranted,
            EventKind::ConfirmationDenied,
            EventKind::CheckpointCreated,
            EventKind::SnapshotCreated,
            EventKind::SystemStarted,
            EventKind::SystemShutdown,
            EventKind::Error,
        ];
        for kind in kinds {
            let debug = format!("{:?}", kind);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_event_policy_check_with_reason() {
        let payload = EventPayload::PolicyCheck {
            action: "filesystem.write".to_string(),
            decision: "deny".to_string(),
            reason: Some("not allowed".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: EventPayload = serde_json::from_str(&json).unwrap();
        match back {
            EventPayload::PolicyCheck { reason, .. } => assert_eq!(reason, Some("not allowed".to_string())),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_event_error_event_with_details() {
        let payload = EventPayload::ErrorEvent {
            message: "fatal".to_string(),
            details: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: EventPayload = serde_json::from_str(&json).unwrap();
        match back {
            EventPayload::ErrorEvent { message, details } => {
                assert_eq!(message, "fatal");
                assert!(details.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_event_sequence_number_default() {
        let seq = SequenceNumber(0);
        assert_eq!(seq, SequenceNumber(0));
        assert!(seq < SequenceNumber(1));
    }

    #[test]
    fn test_event_kind_serde() {
        let kind = EventKind::SystemStarted;
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn test_event_new_with_all_fields() {
        let task_id = TaskId::new();
        let event = Event::new(
            task_id,
            EventKind::ModelRequested,
            EventPayload::ModelRequest {
                role: "planner".to_string(),
                prompt_tokens: 100,
                context_budget: 4096,
            },
            "kernel".to_string(),
        );
        assert_eq!(event.task_id, Some(task_id));
        assert_eq!(event.kind, EventKind::ModelRequested);
        assert_eq!(event.sequence, SequenceNumber(0));
        assert_eq!(event.metadata.source, "kernel");
    }

    #[test]
    fn test_event_system_new_with_all_fields() {
        let event = Event::system(
            EventKind::SystemShutdown,
            EventPayload::SystemEvent { message: "bye".to_string() },
            "kernel".to_string(),
        );
        assert!(event.task_id.is_none());
        assert_eq!(event.kind, EventKind::SystemShutdown);
        assert_eq!(event.metadata.source, "kernel");
    }
}
