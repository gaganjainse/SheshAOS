//! `shesh status` — Show current kernel state.

use tracing::info;

use crate::{
    config::AppConfig,
    error::KernelError,
    resource::ResourceMonitor,
    storage::{SqliteEventStore, TaskProjection},
};

/// Run the status command: display kernel state and resource pressure.
pub fn run(config_path: &str) -> Result<(), KernelError> {
    info!("Checking system status");

    let config = AppConfig::load(config_path)?;
    let pressure = ResourceMonitor::snapshot();
    let data_dir = config.resolved_data_dir();

    println!("SheshAOS Status\n");
    println!("  Data dir:       {}", data_dir.display());
    println!("  RAM available:  {} MB", pressure.ram_available_mb);
    println!("  VRAM available: {} MB", pressure.vram_available_mb);
    println!("  Disk available: {} GB", pressure.disk_available_gb);
    println!("  Queue depth:    {}", pressure.queue_depth);

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        KernelError::Config(crate::error::ConfigError::Invalid { message: e.to_string() })
    })?;
    rt.block_on(async {
        let events_dir = data_dir.join("events");
        match SqliteEventStore::open(events_dir).await {
            Ok(store) => match store.read_all().await {
                Ok(events) => {
                    println!("\n  Event Log:      {} events", events.len());
                    let projection = TaskProjection::rebuild(&events);
                    println!("  Total tasks:    {}", projection.task_count());

                    use crate::state::TaskState;
                    let states = [
                        TaskState::Received,
                        TaskState::Classified,
                        TaskState::Planned,
                        TaskState::AwaitingConfirmation,
                        TaskState::Executing,
                        TaskState::Blocked,
                        TaskState::Failed,
                        TaskState::RolledBack,
                        TaskState::Completed,
                        TaskState::Archived,
                    ];

                    let mut has_tasks = false;
                    println!("\n  Tasks by State:");
                    for state in states {
                        let count = projection.tasks_in_state(&state).len();
                        if count > 0 {
                            has_tasks = true;
                            println!("    {:<20} : {}", format!("{:?}", state), count);
                        }
                    }
                    if !has_tasks {
                        println!("    (no tasks found)");
                    }
                }
                Err(e) => {
                    println!("\n  Active tasks:   (failed to read events: {})", e);
                }
            },
            Err(e) => {
                println!("\n  Active tasks:   (failed to open event store: {})", e);
            }
        }
    });

    Ok(())
}
