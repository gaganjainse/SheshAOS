//! Storage layer for NexusAOS.
//!
//! Provides the append-only event store, snapshots, and projections.

pub mod event_store;
pub mod projection;
pub mod snapshot;

pub use event_store::EventStore;
pub use projection::{ProjectedTask, TaskProjection};
pub use snapshot::{Snapshot, SnapshotStore};
