use async_trait::async_trait;
use std::path::PathBuf;

use crate::{
    error::{NexusError, StorageError},
    events::Event,
    storage::EventStore,
};

/// SQLite-backed event store.
pub struct SqliteEventStore {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteEventStore {
    /// Open or create a SQLite event store at the given path.
    pub async fn open(path: PathBuf) -> Result<Self, NexusError> {
        let db_path = path.join("events.db");
        let conn = rusqlite::Connection::open(db_path).map_err(|e| NexusError::Storage(StorageError::Database(e)))?;
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
        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }

    /// Read all events in sequence order.
    pub async fn read_all(&self) -> Result<Vec<Event>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM events ORDER BY sequence ASC")
            .map_err(|e| StorageError::Database(e.into()))?;
        let rows = stmt
            .query_map([], |row| {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                })
            })
            .map_err(|e| StorageError::Database(e.into()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Database(e.into()))
    }

    /// Read events for a specific task.
    pub async fn read_for_task(&self, task_id: &crate::task::TaskId) -> Result<Vec<Event>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM events WHERE task_id = ?1 ORDER BY sequence ASC")
            .map_err(|e| StorageError::Database(e.into()))?;
        let rows = stmt
            .query_map([&task_id.0.to_string()], |row| {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                })
            })
            .map_err(|e| StorageError::Database(e.into()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Database(e.into()))
    }

    /// Read events since a given sequence number.
    pub async fn read_since(&self, sequence: u64) -> Result<Vec<Event>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM events WHERE sequence >= ?1 ORDER BY sequence ASC")
            .map_err(|e| StorageError::Database(e.into()))?;
        let rows = stmt
            .query_map([&sequence], |row| {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                })
            })
            .map_err(|e| StorageError::Database(e.into()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Database(e.into()))
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn append(&self, event: Event) -> Result<(), NexusError> {
        let data = serde_json::to_string(&event).map_err(NexusError::Serde)?;
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT OR REPLACE INTO events (id, task_id, sequence, data) VALUES (?1, ?2, ?3, ?4)",
                (
                    event.id.0.to_string(),
                    event.task_id.map(|id| id.0.to_string()),
                    event.sequence.0,
                    data,
                ),
            )
            .map_err(|e| NexusError::Storage(StorageError::Database(e.into())))?;
        Ok(())
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
