//! `sheshaaos-terminal` — Native Zig VT100 Terminal Parsing Expert & PTY Manager.

pub mod ffi;
pub mod pty;

pub use ffi::ZigVt100Parser;
pub use pty::PtyManager;
