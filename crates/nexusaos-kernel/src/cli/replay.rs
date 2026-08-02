//! `nexusaos replay` — Replay event history for a task.

use std::str::FromStr;

use tracing::info;

use crate::{config::AppConfig, error::NexusError, storage::{EventStore, JsonlEventStore}, task::TaskId};

/// Replay and display the event history for a given task ID.
pub fn run(config_path: &str, task_id: &str) -> Result<(), NexusError> {
    info!(task_id = task_id, "Replaying task history");

    let config = AppConfig::load(config_path)?;
    let data_dir = config.resolved_data_dir();

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        NexusError::Config(crate::error::ConfigError::Invalid { message: e.to_string() })
    })?;
    rt.block_on(async {
        let events_dir = data_dir.join("events");
        let store = JsonlEventStore::open(events_dir).await?;

        let events = if task_id == "all" {
            store.read_all().await?
        } else {
            let id = TaskId::from_str(task_id).map_err(|_| {
                NexusError::Task(crate::error::TaskError::NotFound { id: task_id.to_string() })
            })?;
            store.read_for_task(&id).await?
        };

        println!("Replaying {} events for task: {}", events.len(), task_id);
        for e in events {
            println!("[{}] {:?} (Seq: {}) - {:?}", e.timestamp, e.kind, e.sequence.0, e.payload);
        }

        Ok::<(), NexusError>(())
    })
}
