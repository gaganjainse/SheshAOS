//! Storage layer for NexusAOS.
//!
//! Provides the append-only event store, snapshots, and projections.

pub mod event_store;
pub mod projection;
pub mod snapshot;
pub mod sqlite_event_store;

pub use event_store::{EventStore, JsonlEventStore};
pub use projection::TaskProjection;
pub use snapshot::{Snapshot, SnapshotStore};
pub use sqlite_event_store::SqliteEventStore;
