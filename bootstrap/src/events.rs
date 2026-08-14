use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum EventKind {
    TaskCreated,
    StateChanged,
    ToolCalled,
    ModelInferenceRequested,
    PolicyViolation,
    SystemAlert,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

impl Event {
    pub fn new(kind: EventKind, payload: serde_json::Value) -> Self {
        Self {
            id: EventId::new(),
            kind,
            timestamp: Utc::now(),
            payload,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventKind::TaskCreated, serde_json::json!({"task_id": "123"}));
        assert_eq!(event.kind, EventKind::TaskCreated);
        assert!(event.metadata.is_empty());
    }
}
