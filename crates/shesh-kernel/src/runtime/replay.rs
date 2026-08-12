use crate::{
    error::NexusError,
    events::{Event, EventKind, EventPayload},
    state::{TaskRecord, TaskState},
    storage::{EventStore, TaskProjection},
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
        async fn read_since(&self, _sequence: u64) -> Result<Vec<Event>, NexusError> {
            Ok(self.events.lock().unwrap().clone())
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

    #[tokio::test]
    async fn test_replay_empty_store() {
        let store = MockEventStore::new();
        let projection = ReplayEngine::replay(&store).await.unwrap();
        assert_eq!(projection.tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_task_history() {
        let store = MockEventStore::new();
        let task_id = TaskId::new();

        let event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "kernel".into(),
        );
        store.append(event).await.unwrap();

        let history = ReplayEngine::task_history(&store, &task_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].kind, EventKind::TaskCreated);
    }

    #[tokio::test]
    async fn test_task_history_empty() {
        let store = MockEventStore::new();
        let task_id = TaskId::new();
        let history = ReplayEngine::task_history(&store, &task_id).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_replay_multiple_tasks() {
        use crate::task::{TaskInput, TaskRequest};

        let store = MockEventStore::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();

        let req1 = TaskRequest::new(TaskInput::Text("task1".into()));
        let req2 = TaskRequest::new(TaskInput::Text("task2".into()));

        let e1 = Event::new(
            t1,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::to_value(&req1).unwrap() },
            "k".into(),
        );
        let e2 = Event::new(
            t2,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::to_value(&req2).unwrap() },
            "k".into(),
        );
        let e3 = Event::new(
            t1,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged { from: "Received".into(), to: "Classified".into() },
            "k".into(),
        );

        store.append(e1).await.unwrap();
        store.append(e2).await.unwrap();
        store.append(e3).await.unwrap();

        let projection = ReplayEngine::replay(&store).await.unwrap();
        assert_eq!(projection.tasks.len(), 2);
        assert_eq!(projection.tasks.get(&t1).unwrap().current_state, TaskState::Classified);
        assert_eq!(projection.tasks.get(&t2).unwrap().current_state, TaskState::Received);
    }

    #[tokio::test]
    async fn test_replay_ignores_system_events() {
        let store = MockEventStore::new();

        // System event (no task_id) should be ignored
        let sys_event = Event::system(
            EventKind::SystemStarted,
            EventPayload::SystemEvent { message: "started".into() },
            "k".into(),
        );
        store.append(sys_event).await.unwrap();

        let projection = ReplayEngine::replay(&store).await.unwrap();
        assert_eq!(projection.tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_replay_unknown_state_skipped() {
        use crate::task::{TaskInput, TaskRequest};

        let store = MockEventStore::new();
        let task_id = TaskId::new();

        let req = TaskRequest::new(TaskInput::Text("test".into()));
        let e1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::to_value(&req).unwrap() },
            "k".into(),
        );
        let e2 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged { from: "Received".into(), to: "UnknownState".into() },
            "k".into(),
        );

        store.append(e1).await.unwrap();
        store.append(e2).await.unwrap();

        let projection = ReplayEngine::replay(&store).await.unwrap();
        let task = projection.tasks.get(&task_id).unwrap();
        // Unknown state is skipped, so state remains Received
        assert_eq!(task.current_state, TaskState::Received);
    }

    #[tokio::test]
    async fn test_replay_state_history_preserved() {
        use crate::task::{TaskInput, TaskRequest};

        let store = MockEventStore::new();
        let task_id = TaskId::new();

        let req = TaskRequest::new(TaskInput::Text("test".into()));
        let mut e1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::to_value(&req).unwrap() },
            "k".into(),
        );
        e1.sequence = crate::events::SequenceNumber(1);
        let mut e2 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged { from: "Received".into(), to: "Classified".into() },
            "k".into(),
        );
        e2.sequence = crate::events::SequenceNumber(2);
        let mut e3 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged { from: "Classified".into(), to: "Planned".into() },
            "k".into(),
        );
        e3.sequence = crate::events::SequenceNumber(3);

        store.append(e1).await.unwrap();
        store.append(e2).await.unwrap();
        store.append(e3).await.unwrap();

        let projection = ReplayEngine::replay(&store).await.unwrap();
        let task = projection.tasks.get(&task_id).unwrap();
        assert_eq!(task.current_state, TaskState::Planned);
        assert_eq!(task.state_history.len(), 3);
    }
}
