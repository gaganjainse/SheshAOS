use std::collections::HashMap;

use crate::{
    events::{Event, EventPayload},
    state::{TaskRecord, TaskState},
    task::TaskId,
};

/// Current-state view of all tasks, derived from events.
pub struct TaskProjection {
    pub tasks: HashMap<TaskId, TaskRecord>,
    pub last_sequence: u64,
}

impl TaskProjection {
    pub fn new() -> Self {
        Self { tasks: HashMap::new(), last_sequence: 0 }
    }

    /// Apply an event to update the projection.
    pub fn apply(&mut self, event: &Event) {
        self.last_sequence = event.sequence.0;

        let task_id = match event.task_id {
            Some(id) => id,
            None => return,
        };

        match &event.payload {
            EventPayload::TaskCreated { request } => {
                let req = serde_json::from_value::<crate::task::TaskRequest>(request.clone())
                    .unwrap_or_else(|_| crate::task::TaskRequest::new(crate::task::TaskInput::Text("(unknown)".into())));
                self.tasks.insert(
                    task_id,
                    TaskRecord {
                        task_id,
                        request: req,
                        current_state: TaskState::Received,
                        assigned_role: None,
                        state_history: vec![(TaskState::Received, event.timestamp)],
                    },
                );
            }
            EventPayload::StateChanged { to, .. } => {
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    let new_state = match to.as_str() {
                        "Received" => Some(TaskState::Received),
                        "Classified" => Some(TaskState::Classified),
                        "Planned" => Some(TaskState::Planned),
                        "AwaitingConfirmation" => Some(TaskState::AwaitingConfirmation),
                        "Executing" => Some(TaskState::Executing),
                        "Blocked" => Some(TaskState::Blocked),
                        "Failed" => Some(TaskState::Failed),
                        "RolledBack" => Some(TaskState::RolledBack),
                        "Completed" => Some(TaskState::Completed),
                        "Archived" => Some(TaskState::Archived),
                        _ => None,
                    };

                    if let Some(state) = new_state {
                        task.current_state = state;
                        task.state_history.push((state, event.timestamp));
                    }
                }
            }
            EventPayload::ModelRequest { role, .. } => {
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    let role_enum = match role.as_str() {
                        "Planner" => crate::state::ModelRole::Planner,
                        "Coder" => crate::state::ModelRole::Coder,
                        "Vision" => crate::state::ModelRole::Vision,
                        "Reviewer" => crate::state::ModelRole::Reviewer,
                        _ => crate::state::ModelRole::Planner,
                    };
                    task.assigned_role = Some(role_enum);
                }
            }
            _ => {}
        }
    }

    /// Rebuild from a slice of events.
    pub fn rebuild(events: &[Event]) -> Self {
        let mut proj = Self::new();
        for event in events {
            proj.apply(event);
        }
        proj
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: &TaskId) -> Option<&TaskRecord> {
        self.tasks.get(id)
    }

    /// Get all tasks in a given state.
    pub fn tasks_in_state(&self, state: &TaskState) -> Vec<&TaskRecord> {
        self.tasks.values().filter(|t| t.current_state == *state).collect()
    }

    /// Get total task count.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

impl Default for TaskProjection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{Event, EventKind, EventPayload};
    use chrono::Utc;

    #[test]
    fn test_projection_rebuild() {
        let task_id = TaskId::new();

        let mut event1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event1.sequence = crate::events::SequenceNumber(1);

        let mut event2 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged {
                from: "Received".to_string(),
                to: "Classified".to_string(),
            },
            "test".to_string(),
        );
        event2.sequence = crate::events::SequenceNumber(2);

        let projection = TaskProjection::rebuild(&[event1, event2]);

        assert_eq!(projection.task_count(), 1);
        let task = projection.get_task(&task_id).unwrap();
        assert_eq!(task.current_state, TaskState::Classified);

        let classified_tasks = projection.tasks_in_state(&TaskState::Classified);
        assert_eq!(classified_tasks.len(), 1);
    }

    #[test]
    fn test_projection_new() {
        let proj = TaskProjection::new();
        assert_eq!(proj.task_count(), 0);
        assert_eq!(proj.last_sequence, 0);
    }

    #[test]
    fn test_projection_default() {
        let proj = TaskProjection::default();
        assert_eq!(proj.task_count(), 0);
    }

    #[test]
    fn test_apply_task_created() {
        let mut proj = TaskProjection::new();
        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({"prompt": "test"}) },
            "test".to_string(),
        );
        event.sequence = crate::events::SequenceNumber(1);

        proj.apply(&event);
        assert_eq!(proj.task_count(), 1);
        let task = proj.get_task(&task_id).unwrap();
        assert_eq!(task.current_state, TaskState::Received);
        assert_eq!(task.assigned_role, None);
    }

    #[test]
    fn test_apply_state_changed() {
        let mut proj = TaskProjection::new();
        let task_id = TaskId::new();

        let mut e1 = Event::new(task_id, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.sequence = crate::events::SequenceNumber(1);
        proj.apply(&e1);

        let mut e2 = Event::new(task_id, EventKind::TaskStateChanged, EventPayload::StateChanged { from: "Received".into(), to: "Executing".into() }, "test".into());
        e2.sequence = crate::events::SequenceNumber(2);
        proj.apply(&e2);

        let task = proj.get_task(&task_id).unwrap();
        assert_eq!(task.current_state, TaskState::Executing);
    }

    #[test]
    fn test_apply_model_request_sets_assigned_role() {
        let mut proj = TaskProjection::new();
        let task_id = TaskId::new();

        let mut e1 = Event::new(task_id, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.sequence = crate::events::SequenceNumber(1);
        proj.apply(&e1);

        let mut e2 = Event::new(task_id, EventKind::ModelRequested, EventPayload::ModelRequest { role: "Coder".into(), prompt_tokens: 0, context_budget: 100 }, "test".into());
        e2.sequence = crate::events::SequenceNumber(2);
        proj.apply(&e2);

        let task = proj.get_task(&task_id).unwrap();
        assert_eq!(task.assigned_role, Some(crate::state::ModelRole::Coder));
    }

    #[test]
    fn test_apply_system_event_ignored() {
        let mut proj = TaskProjection::new();
        let mut event = Event::system(
            EventKind::SystemStarted,
            EventPayload::SystemEvent { message: "started".into() },
            "test".into(),
        );
        event.sequence = crate::events::SequenceNumber(1);
        proj.apply(&event);
        assert_eq!(proj.task_count(), 0);
    }

    #[test]
    fn test_apply_unknown_state_skipped() {
        let mut proj = TaskProjection::new();
        let task_id = TaskId::new();

        let mut e1 = Event::new(task_id, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.sequence = crate::events::SequenceNumber(1);
        proj.apply(&e1);

        let mut e2 = Event::new(task_id, EventKind::TaskStateChanged, EventPayload::StateChanged { from: "Received".into(), to: "FakeState".into() }, "test".into());
        e2.sequence = crate::events::SequenceNumber(2);
        proj.apply(&e2);

        let task = proj.get_task(&task_id).unwrap();
        assert_eq!(task.current_state, TaskState::Received); // unchanged
    }

    #[test]
    fn test_get_task_not_found() {
        let proj = TaskProjection::new();
        let fake_id = TaskId::new();
        assert!(proj.get_task(&fake_id).is_none());
    }

    #[test]
    fn test_tasks_in_state_empty_projection() {
        let proj = TaskProjection::new();
        let result = proj.tasks_in_state(&TaskState::Received);
        assert!(result.is_empty());
    }

    #[test]
    fn test_tasks_in_state_multiple_tasks() {
        let proj = TaskProjection::rebuild(&[
            Event::new(TaskId::new(), EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string()),
            Event::new(TaskId::new(), EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string()),
        ]);
        let received = proj.tasks_in_state(&TaskState::Received);
        assert_eq!(received.len(), 2);
    }

    #[test]
    fn test_rebuild_multiple_tasks_different_states() {
        let t1 = TaskId::new();
        let t2 = TaskId::new();

        let mut e1 = Event::new(t1, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.sequence = crate::events::SequenceNumber(1);
        let mut e2 = Event::new(t2, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e2.sequence = crate::events::SequenceNumber(2);
        let mut e3 = Event::new(t1, EventKind::TaskStateChanged, EventPayload::StateChanged { from: "Received".into(), to: "Completed".into() }, "test".into());
        e3.sequence = crate::events::SequenceNumber(3);

        let proj = TaskProjection::rebuild(&[e1, e2, e3]);
        assert_eq!(proj.task_count(), 2);
        assert_eq!(proj.get_task(&t1).unwrap().current_state, TaskState::Completed);
        assert_eq!(proj.get_task(&t2).unwrap().current_state, TaskState::Received);
    }

    #[test]
    fn test_projection_last_sequence_updated() {
        let mut proj = TaskProjection::new();
        let mut event = Event::new(
            TaskId::new(),
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event.sequence = crate::events::SequenceNumber(42);
        proj.apply(&event);
        assert_eq!(proj.last_sequence, 42);
    }

    #[test]
    fn test_apply_updates_updated_at() {
        let mut proj = TaskProjection::new();
        let task_id = TaskId::new();
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(10);

        let mut e1 = Event::new(task_id, EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.timestamp = ts1;
        e1.sequence = crate::events::SequenceNumber(1);
        proj.apply(&e1);

        let mut e2 = Event::new(task_id, EventKind::TaskStateChanged, EventPayload::StateChanged { from: "Received".into(), to: "Classified".into() }, "test".into());
        e2.timestamp = ts2;
        e2.sequence = crate::events::SequenceNumber(2);
        proj.apply(&e2);

        let task = proj.get_task(&task_id).unwrap();
        assert_eq!(task.state_history.len(), 2);
        assert_eq!(task.state_history[1].0, TaskState::Classified);
        assert_eq!(task.state_history[1].1, ts2);
    }
}
