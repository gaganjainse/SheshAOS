use crate::{
    error::NexusError,
    events::{Event, EventKind, EventPayload},
    runtime::kernel::{EventStore, TaskProjection},
    state::{TaskRecord, TaskState},
    task::{TaskId, TaskRequest},
};

/// Replays events from the store to rebuild kernel state.
pub struct ReplayEngine;

impl ReplayEngine {
    /// Replay all events and rebuild the task projection.
    pub async fn replay(store: &dyn EventStore) -> Result<TaskProjection, NexusError> {
        let events = store.get_all_events().await?;
        let mut projection = TaskProjection::new();

        for event in events {
            if let Some(task_id) = event.task_id {
                match event.kind {
                    EventKind::TaskCreated => {
                        if let EventPayload::TaskCreated { request } = event.payload {
                            let Ok(req) = serde_json::from_value::<TaskRequest>(request) else {
                                continue;
                            };
                            let record = TaskRecord {
                                task_id,
                                request: req,
                                current_state: TaskState::Received,
                                assigned_role: None,
                                state_history: vec![(TaskState::Received, event.timestamp)],
                            };
                            projection.tasks.insert(task_id, record);
                        }
                    }
                    EventKind::TaskClassified => {
                        if let Some(task) = projection.tasks.get_mut(&task_id) {
                            task.current_state = TaskState::Classified;
                            task.state_history.push((TaskState::Classified, event.timestamp));
                        }
                    }
                    EventKind::TaskStateChanged => {
                        if let EventPayload::StateChanged { to, .. } = event.payload {
                            // Basic parsing of state string back to enum could go here
                            // For simplicity, we assume we map string to state properly
                            // Let's implement a rudimentary match for states:
                            let new_state = match to.as_str() {
                                "Received" => TaskState::Received,
                                "Classified" => TaskState::Classified,
                                "Planned" => TaskState::Planned,
                                "AwaitingConfirmation" => TaskState::AwaitingConfirmation,
                                "Executing" => TaskState::Executing,
                                "Blocked" => TaskState::Blocked,
                                "Failed" => TaskState::Failed,
                                "RolledBack" => TaskState::RolledBack,
                                "Completed" => TaskState::Completed,
                                "Archived" => TaskState::Archived,
                                _ => continue, // Unknown state
                            };
                            if let Some(task) = projection.tasks.get_mut(&task_id) {
                                task.current_state = new_state;
                                task.state_history.push((new_state, event.timestamp));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(projection)
    }

    /// Get the event history for a specific task.
    pub async fn task_history(
        store: &dyn EventStore,
        task_id: &TaskId,
    ) -> Result<Vec<Event>, NexusError> {
        store.get_task_events(task_id).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    struct MockEventStore {
        events: Mutex<Vec<Event>>,
    }

    impl MockEventStore {
        fn new() -> Self {
            Self { events: Mutex::new(Vec::new()) }
        }
    }

    #[async_trait]
    impl EventStore for MockEventStore {
        async fn append(&self, event: Event) -> Result<(), NexusError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        async fn get_all_events(&self) -> Result<Vec<Event>, NexusError> {
            Ok(self.events.lock().unwrap().clone())
        }
        async fn get_task_events(&self, task_id: &TaskId) -> Result<Vec<Event>, NexusError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.task_id == Some(*task_id))
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn test_replay() {
        let store = MockEventStore::new();
        let task_id = TaskId::new();
        let request = TaskRequest::new(crate::task::TaskInput::Text("test".into()));

        let event1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::to_value(&request).unwrap() },
            "kernel".into(),
        );
        let event2 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged { from: "Received".into(), to: "Classified".into() },
            "kernel".into(),
        );

        store.append(event1).await.unwrap();
        store.append(event2).await.unwrap();

        let projection = ReplayEngine::replay(&store).await.unwrap();
        let task = projection.tasks.get(&task_id).unwrap();

        assert_eq!(task.current_state, TaskState::Classified);
        assert_eq!(task.state_history.len(), 2);
    }
}
