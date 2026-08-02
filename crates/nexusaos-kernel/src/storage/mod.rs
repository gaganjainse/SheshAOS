//! Storage layer for NexusAOS.
//!
//! Provides the append-only event store, snapshots, and projections.

pub mod event_store;
pub mod projection;
pub mod snapshot;

pub use event_store::{EventStore, JsonlEventStore};
pub use projection::TaskProjection;
pub use snapshot::{Snapshot, SnapshotStore};
