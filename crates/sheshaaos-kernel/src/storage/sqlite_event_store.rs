use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    error::{NexusError, StorageError},
    events::{Event, SequenceNumber},
    storage::EventStore,
};

/// SQLite-backed event store.
pub struct SqliteEventStore {
    db_path: PathBuf,
    next_sequence: AtomicU64,
}

impl SqliteEventStore {
    /// Open or create a SQLite event store at the given path.
    pub async fn open(path: PathBuf) -> Result<Self, NexusError> {
        let db_path = path.join("events.db");
        let db_path_clone = db_path.clone();
        let max_seq: i64 = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path_clone)
                .map_err(|e| NexusError::Storage(StorageError::Database(e)))?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS events (
                    id TEXT PRIMARY KEY,
                    task_id TEXT,
                    sequence INTEGER,
                    data TEXT NOT NULL
                )",
                (),
            )
            .map_err(|e| NexusError::Storage(StorageError::Database(e)))?;
            // Migrate existing default-sequence rows to 0
            conn.execute("UPDATE events SET sequence = 0 WHERE sequence IS NULL OR sequence = 0", ())
                .ok();
            let max: i64 = conn
                .query_row("SELECT COALESCE(MAX(sequence), 0) FROM events", [], |r| r.get(0))
                .unwrap_or(0);
            Ok::<_, NexusError>(max)
        })
        .await
        .map_err(|e| {
            NexusError::Storage(StorageError::Io(std::io::Error::other(
                e.to_string(),
            )))
        })??;
        Ok(Self {
            db_path,
            next_sequence: AtomicU64::new(max_seq as u64 + 1),
        })
    }

    /// Read all events in sequence order.
    pub async fn read_all(&self) -> Result<Vec<Event>, StorageError> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(StorageError::Database)?;
            let mut stmt = conn
                .prepare("SELECT data FROM events ORDER BY sequence ASC")
                .map_err(StorageError::Database)?;
            let rows = stmt
                .query_map([], |row| {
                    let data: String = row.get(0)?;
                    serde_json::from_str(&data).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                    })
                })
                .map_err(StorageError::Database)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::Database)
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(
            e.to_string(),
        )))?
    }

    /// Read events for a specific task.
    pub async fn read_for_task(&self, task_id: &crate::task::TaskId) -> Result<Vec<Event>, StorageError> {
        let db_path = self.db_path.clone();
        let task_id_str = task_id.0.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(StorageError::Database)?;
            let mut stmt = conn
                .prepare("SELECT data FROM events WHERE task_id = ?1 ORDER BY sequence ASC")
                .map_err(StorageError::Database)?;
            let rows = stmt
                .query_map([&task_id_str], |row| {
                    let data: String = row.get(0)?;
                    serde_json::from_str(&data).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                    })
                })
                .map_err(StorageError::Database)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::Database)
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(
            e.to_string(),
        )))?
    }

    /// Read events since a given sequence number.
    pub async fn read_since(&self, sequence: u64) -> Result<Vec<Event>, StorageError> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(StorageError::Database)?;
            let mut stmt = conn
                .prepare("SELECT data FROM events WHERE sequence >= ?1 ORDER BY sequence ASC")
                .map_err(StorageError::Database)?;
            let rows = stmt
                .query_map([&sequence], |row| {
                    let data: String = row.get(0)?;
                    serde_json::from_str(&data).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                    })
                })
                .map_err(StorageError::Database)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::Database)
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(
            e.to_string(),
        )))?
    }

    /// Get total event count.
    pub async fn count(&self) -> Result<u64, StorageError> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(StorageError::Database)?;
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM events")
                .map_err(StorageError::Database)?;
            let count: i64 = stmt.query_row([], |row| row.get(0)).map_err(StorageError::Database)?;
            Ok(count as u64)
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(
            e.to_string(),
        )))?
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn append(&self, mut event: Event) -> Result<(), NexusError> {
        // Assign the next monotonic sequence number (matching JsonlEventStore behavior)
        event.sequence = SequenceNumber(self.next_sequence.fetch_add(1, Ordering::SeqCst));

        let data = serde_json::to_string(&event).map_err(NexusError::Serde)?;
        let db_path = self.db_path.clone();
        let sequence = event.sequence.0;
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(|e| NexusError::Storage(StorageError::Database(e)))?;
            conn.execute(
                "INSERT INTO events (id, task_id, sequence, data) VALUES (?1, ?2, ?3, ?4)",
                (
                    event.id.0.to_string(),
                    event.task_id.map(|id| id.0.to_string()),
                    sequence,
                    data,
                ),
            )
            .map_err(|e| NexusError::Storage(StorageError::Database(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| NexusError::Storage(StorageError::Io(std::io::Error::other(
            e.to_string(),
        ))))?
    }

    async fn get_all_events(&self) -> Result<Vec<Event>, NexusError> {
        Self::read_all(self).await.map_err(NexusError::Storage)
    }

    async fn get_task_events(&self, task_id: &crate::task::TaskId) -> Result<Vec<Event>, NexusError> {
        Self::read_for_task(self, task_id).await.map_err(NexusError::Storage)
    }

    async fn read_since(&self, sequence: u64) -> Result<Vec<Event>, NexusError> {
        Self::read_since(self, sequence).await.map_err(NexusError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::events::{EventKind, EventPayload, SequenceNumber};
    use crate::task::TaskId;

    #[tokio::test]
    async fn test_open() {
        let temp_dir = TempDir::new().unwrap();
        let store = SqliteEventStore::open(temp_dir.path().to_path_buf()).await.unwrap();
        let events = store.read_all().await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_append() {
        let temp_dir = TempDir::new().unwrap();
        let store = SqliteEventStore::open(temp_dir.path().to_path_buf()).await.unwrap();
        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event.sequence = SequenceNumber(1);
        store.append(event.clone()).await.unwrap();
        let events = store.read_all().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_read_for_task() {
        let temp_dir = TempDir::new().unwrap();
        let store = SqliteEventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let task_id = TaskId::new();
        let other_task_id = TaskId::new();

        let mut event1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event1.sequence = SequenceNumber(1);
        store.append(event1).await.unwrap();

        let mut event2 = Event::new(
            other_task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event2.sequence = SequenceNumber(2);
        store.append(event2).await.unwrap();

        let task_events = store.read_for_task(&task_id).await.unwrap();
        assert_eq!(task_events.len(), 1);
        assert_eq!(task_events[0].task_id, Some(task_id));

        let other_events = store.read_for_task(&other_task_id).await.unwrap();
        assert_eq!(other_events.len(), 1);
        assert_eq!(other_events[0].task_id, Some(other_task_id));
    }

    #[tokio::test]
    async fn test_read_since() {
        let temp_dir = TempDir::new().unwrap();
        let store = SqliteEventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        let mut e1 = Event::new(
            TaskId::new(),
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        e1.sequence = SequenceNumber(1);
        store.append(e1).await.unwrap();

        let mut e2 = Event::new(
            TaskId::new(),
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        e2.sequence = SequenceNumber(2);
        store.append(e2).await.unwrap();

        let mut e3 = Event::new(
            TaskId::new(),
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        e3.sequence = SequenceNumber(3);
        store.append(e3).await.unwrap();

        let since_2 = store.read_since(2).await.unwrap();
        assert_eq!(since_2.len(), 2);

        let since_3 = store.read_since(3).await.unwrap();
        assert_eq!(since_3.len(), 1);

        let since_4 = store.read_since(4).await.unwrap();
        assert!(since_4.is_empty());
    }

    #[tokio::test]
    async fn test_count() {
        let temp_dir = TempDir::new().unwrap();
        let store = SqliteEventStore::open(temp_dir.path().to_path_buf()).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 0);

        let task_id = TaskId::new();
        let mut event = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event.sequence = SequenceNumber(1);
        store.append(event).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 1);

        let mut event2 = Event::new(
            TaskId::new(),
            EventKind::TaskCreated,
            EventPayload::TaskCreated { request: serde_json::json!({}) },
            "test".to_string(),
        );
        event2.sequence = SequenceNumber(2);
        store.append(event2).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 2);
    }
}