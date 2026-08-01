use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    events::{Event, EventPayload},
    state::TaskState,
    task::TaskId,
};

/// Current-state view of all tasks, derived from events.
pub struct TaskProjection {
    tasks: HashMap<TaskId, ProjectedTask>,
    last_sequence: u64,
}

#[derive(Debug, Clone)]
pub struct ProjectedTask {
    pub task_id: TaskId,
    pub current_state: TaskState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assigned_role: Option<String>,
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
            None => return, // Ignore system events without a task
        };

        match &event.payload {
            EventPayload::TaskCreated { .. } => {
                self.tasks.insert(
                    task_id,
                    ProjectedTask {
                        task_id,
                        current_state: TaskState::Received,
                        created_at: event.timestamp,
                        updated_at: event.timestamp,
                        assigned_role: None,
                    },
                );
            }
            EventPayload::StateChanged { to, .. } => {
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    // Convert string to TaskState if possible.
                    // Assuming string representation matches enum name loosely or exactly.
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
                        task.updated_at = event.timestamp;
                    }
                }
            }
            EventPayload::ModelRequest { role, .. } => {
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    task.assigned_role = Some(role.clone());
                    task.updated_at = event.timestamp;
                }
            }
            _ => {
                // Update updated_at for other task-related events
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    task.updated_at = event.timestamp;
                }
            }
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
    pub fn get_task(&self, id: &TaskId) -> Option<&ProjectedTask> {
        self.tasks.get(id)
    }

    /// Get all tasks in a given state.
    pub fn tasks_in_state(&self, state: &TaskState) -> Vec<&ProjectedTask> {
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
    use crate::events::EventKind;

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
}
