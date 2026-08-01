//! NexusAOS v2 — Governance-first, event-sourced AI operating environment.
//!
//! This crate implements a microkernel-like system that routes tasks to specialist
//! local AI models, enforces policy on every action, and maintains an append-only
//! audit trail of all state changes.

pub mod capability;
pub mod cli;
pub mod config;
pub mod context;
pub mod error;
pub mod events;
pub mod model;
pub mod policy;
pub mod resource;
pub mod router;
pub mod runtime;
pub mod state;
pub mod storage;
pub mod task;
pub mod tools;

// Re-export commonly used types at crate root
pub use error::NexusError;
pub type Result<T> = std::result::Result<T, NexusError>;
