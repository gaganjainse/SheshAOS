use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};

use crate::{
    error::StorageError,
    events::{Event, EventId, SequenceNumber},
    task::TaskId,
};

/// Append-only event store backed by JSONL files.
///
/// Events are written as one JSON object per line to `events.jsonl`.
/// An in-memory index maps EventId -> byte offset for fast lookup.
/// SequenceNumber is monotonically increasing, assigned at append time.
pub struct EventStore {
    path: PathBuf,
    index: RwLock<HashMap<EventId, u64>>,
    next_sequence: AtomicU64,
    writer: Mutex<File>,
}

impl EventStore {
    /// Open or create an event store at the given directory.
    pub async fn open(path: PathBuf) -> Result<Self, StorageError> {
        let file_path = path.join("events.jsonl");
        let mut index = HashMap::new();
        let mut next_sequence = 1;

        let file = OpenOptions::new().create(true).append(true).open(&file_path).await?;

        // Rebuild index from existing file
        let read_file = File::open(&file_path).await?;
        let mut reader = BufReader::new(read_file);
        let mut line = String::new();
        let mut offset = 0;

        while reader.read_line(&mut line).await? > 0 {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                index.insert(event.id, offset);
                if event.sequence.0 >= next_sequence {
                    next_sequence = event.sequence.0 + 1;
                }
            }
            offset += line.len() as u64;
            line.clear();
        }

        Ok(Self {
            path,
            index: RwLock::new(index),
            next_sequence: AtomicU64::new(next_sequence),
            writer: Mutex::new(file),
        })
    }

    /// Append an event. Assigns sequence number, writes JSON line, fsyncs.
    pub async fn append(&self, event: &mut Event) -> Result<(), StorageError> {
        {
            let idx = self.index.read().unwrap_or_else(|e| e.into_inner());
            if idx.contains_key(&event.id) {
                return Err(StorageError::DuplicateEvent { id: event.id.to_string() });
            }
        }

        let seq = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        event.sequence = SequenceNumber(seq);

        let mut json = serde_json::to_string(event)?;
        json.push('\n');

        let mut writer = self.writer.lock().await;

        let metadata = writer.metadata().await?;
        let offset = metadata.len();

        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?; // fsync can be added later

        let mut idx = self.index.write().unwrap_or_else(|e| e.into_inner());
        idx.insert(event.id, offset);

        Ok(())
    }

    /// Read all events in sequence order.
    pub async fn read_all(&self) -> Result<Vec<Event>, StorageError> {
        let file_path = self.path.join("events.jsonl");
        let file = File::open(&file_path).await?;
        let mut reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                events.push(event);
            }
            line.clear();
        }

        Ok(events)
    }

    /// Read events for a specific task.
    pub async fn read_for_task(&self, task_id: &TaskId) -> Result<Vec<Event>, StorageError> {
        let events = self.read_all().await?;
        Ok(events.into_iter().filter(|e| e.task_id == Some(*task_id)).collect())
    }

    /// Read events since a given sequence number.
    pub async fn read_since(&self, sequence: u64) -> Result<Vec<Event>, StorageError> {
        let events = self.read_all().await?;
        Ok(events.into_iter().filter(|e| e.sequence.0 >= sequence).collect())
    }

    /// Get total event count.
    pub fn count(&self) -> u64 {
        self.index.read().unwrap_or_else(|e| e.into_inner()).len() as u64
    }
}

#[async_trait::async_trait]
impl crate::runtime::kernel::EventStore for EventStore {
    async fn append(&self, mut event: Event) -> Result<(), crate::error::NexusError> {
        Self::append(self, &mut event).await.map_err(crate::error::NexusError::Storage)
    }

    async fn get_all_events(&self) -> Result<Vec<Event>, crate::error::NexusError> {
        Self::read_all(self).await.map_err(crate::error::NexusError::Storage)
    }

    async fn get_task_events(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<Event>, crate::error::NexusError> {
        Self::read_for_task(self, task_id).await.map_err(crate::error::NexusError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tempfile::TempDir;

    use super::*;
    use crate::events::{EventKind, EventPayload};

    #[tokio::test]
    async fn test_event_store_append_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let store = EventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let task_id = TaskId::new();
        let mut event1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::SystemEvent { message: "test".to_string() },
            "test".to_string(),
        );

        store.append(&mut event1).await.unwrap();

        assert_eq!(store.count(), 1);
        assert_eq!(event1.sequence.0, 1);

        let events = store.read_all().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event1.id);

        let task_events = store.read_for_task(&task_id).await.unwrap();
        assert_eq!(task_events.len(), 1);

        let since_events = store.read_since(1).await.unwrap();
        assert_eq!(since_events.len(), 1);
    }

    #[tokio::test]
    async fn test_event_store_open_new_directory() {
        let temp_dir = TempDir::new().unwrap();
        let new_path = temp_dir.path().join("new_store");
        let store = EventStore::open(new_path).await.unwrap();
        assert_eq!(store.count(), 0);
    }

    #[tokio::test]
    async fn test_event_store_duplicate_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let store = EventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );

        store.append(&mut event).await.unwrap();
        // Try to append the same event again
        let result = store.append(&mut event).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::DuplicateEvent { .. } => {}
            _ => panic!("Expected DuplicateEvent"),
        }
    }

    #[tokio::test]
    async fn test_event_store_read_for_task_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let store = EventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        store.append(&mut event).await.unwrap();

        let other_id = TaskId::new();
        let events = store.read_for_task(&other_id).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_event_store_read_since() {
        let temp_dir = TempDir::new().unwrap();
        let store = EventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let mut e1 = Event::new(TaskId::new(), EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e1.sequence = crate::events::SequenceNumber(1);
        store.append(&mut e1).await.unwrap();

        let mut e2 = Event::new(TaskId::new(), EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e2.sequence = crate::events::SequenceNumber(2);
        store.append(&mut e2).await.unwrap();

        let mut e3 = Event::new(TaskId::new(), EventKind::TaskCreated, EventPayload::TaskCreated { request: serde_json::json!({}) }, "test".to_string());
        e3.sequence = crate::events::SequenceNumber(3);
        store.append(&mut e3).await.unwrap();

        let since_2 = store.read_since(2).await.unwrap();
        assert_eq!(since_2.len(), 2); // seq 2 and 3

        let since_3 = store.read_since(3).await.unwrap();
        assert_eq!(since_3.len(), 1); // seq 3 only

        let since_4 = store.read_since(4).await.unwrap();
        assert!(since_4.is_empty());
    }

    #[tokio::test]
    async fn test_event_store_multiple_events() {
        let temp_dir = TempDir::new().unwrap();
        let store = EventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        for i in 0..10 {
            let mut event = Event::new(
                TaskId::new(),
                EventKind::TaskCreated,
                EventPayload::TaskCreated { request: serde_json::json!({"i": i}) },
                "test".to_string(),
            );
            store.append(&mut event).await.unwrap();
        }

        assert_eq!(store.count(), 10);
        let all = store.read_all().await.unwrap();
        assert_eq!(all.len(), 10);
    }

    #[tokio::test]
    async fn test_event_store_reopen_preserves_index() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let store = EventStore::open(path.clone()).await.unwrap();
            let mut event = Event::new(
                TaskId::new(),
                EventKind::TaskCreated,
                EventPayload::TaskCreated { request: serde_json::json!({}) },
                "test".to_string(),
            );
            store.append(&mut event).await.unwrap();
            assert_eq!(store.count(), 1);
        }

        // Reopen the store
        let store2 = EventStore::open(path).await.unwrap();
        assert_eq!(store2.count(), 1);
        let events = store2.read_all().await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_event_store_get_all_events_trait() {
        let temp_dir = TempDir::new().unwrap();
        let store: Arc<dyn crate::runtime::kernel::EventStore> = Arc::new(
            EventStore::open(temp_dir.path().to_path_buf()).await.unwrap()
        );

        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        store.append(event).await.unwrap();

        let all = store.get_all_events().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_event_store_get_task_events_trait() {
        let temp_dir = TempDir::new().unwrap();
        let store: Arc<dyn crate::runtime::kernel::EventStore> = Arc::new(
            EventStore::open(temp_dir.path().to_path_buf()).await.unwrap()
        );

        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        store.append(event).await.unwrap();

        let task_events = store.get_task_events(&task_id).await.unwrap();
        assert_eq!(task_events.len(), 1);
    }

    #[tokio::test]
    async fn test_event_store_invalid_json_line_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Write a bad JSON line directly to the file
        let file_path = path.join("events.jsonl");
        tokio::fs::write(&file_path, "not json at all\n").await.unwrap();

        let store = EventStore::open(path).await.unwrap();
        assert_eq!(store.count(), 0);
    }
}
