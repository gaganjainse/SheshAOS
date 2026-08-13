//! nexus_ingest — consume kernel events written by the Python Soma layer.
//!
//! shesh-audit's `KernelBridge` appends events to a shared JSONL file
//! (`kernel-events.jsonl`) in the shape:
//! `{"event_id","sequence","kind","timestamp","payload"}` — the "Nexus
//! bridge" P1: the Rust kernel actually consuming those events.
//!
//! This module reads that file, validates each line honestly (bad lines and
//! unknown kinds are counted, not silently dropped), enforces monotonic
//! sequence, and exposes a typed tail view the rest of the kernel can use.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;

use crate::events::EventKind;

/// One event as written by the Python bridge.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NexusEvent {
    pub event_id: String,
    pub sequence: u64,
    pub kind: String,
    pub timestamp: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl NexusEvent {
    /// Parse the kind string into the kernel's EventKind enum.
    pub fn parsed_kind(&self) -> Option<EventKind> {
        serde_json::from_value::<EventKind>(serde_json::Value::String(self.kind.clone())).ok()
    }
}

/// Result of ingesting a nexus JSONL file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IngestResult {
    /// Events accepted and ordered by file order.
    pub events: Vec<NexusEvent>,
    /// Lines that were not valid JSON objects (counted, never silent).
    pub skipped_bad_lines: usize,
    /// Events whose kind string is unknown to the kernel enum.
    pub skipped_unknown_kind: usize,
    /// Events breaking monotonic sequence (counted; the event is kept).
    pub non_monotonic: usize,
    /// Highest sequence number seen.
    pub max_sequence: u64,
}

/// Read and validate a nexus JSONL file.
pub fn ingest(path: &Path) -> std::io::Result<IngestResult> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = IngestResult::default();
    let mut last_seq: Option<u64> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: NexusEvent = match serde_json::from_str(trimmed) {
            Ok(ev) => ev,
            Err(_) => {
                result.skipped_bad_lines += 1;
                continue;
            }
        };
        if event.parsed_kind().is_none() {
            result.skipped_unknown_kind += 1;
        }
        if let Some(last) = last_seq {
            if event.sequence <= last {
                result.non_monotonic += 1;
            }
        }
        last_seq = Some(event.sequence);
        result.max_sequence = result.max_sequence.max(event.sequence);
        result.events.push(event);
    }
    Ok(result)
}

/// Convenience: tail of the ingested events (most recent `n`).
pub fn tail(result: &IngestResult, n: usize) -> &[NexusEvent] {
    let len = result.events.len();
    let start = len.saturating_sub(n);
    &result.events[start..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn write_tmp(lines: &[&str]) -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("nexus-test-{}-{n}.jsonl", std::process::id()));
        let mut f = File::create(&path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        path
    }

    #[test]
    fn ingests_valid_events_in_order() {
        let p = write_tmp(&[
            r#"{"event_id":"a","sequence":1,"kind":"ToolRequested","timestamp":"2026-08-13T00:00:00Z","payload":{"tool":"git"}}"#,
            r#"{"event_id":"b","sequence":2,"kind":"ToolCompleted","timestamp":"2026-08-13T00:00:01Z","payload":{"tool":"git","success":true}}"#,
        ]);
        let r = ingest(&p).unwrap();
        assert_eq!(r.events.len(), 2);
        assert_eq!(r.skipped_bad_lines, 0);
        assert_eq!(r.skipped_unknown_kind, 0);
        assert_eq!(r.non_monotonic, 0);
        assert_eq!(r.max_sequence, 2);
        assert_eq!(r.events[0].parsed_kind(), Some(EventKind::ToolRequested));
        assert_eq!(r.events[1].parsed_kind(), Some(EventKind::ToolCompleted));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn counts_bad_lines_and_unknown_kinds() {
        let p = write_tmp(&[
            r#"{"event_id":"a","sequence":1,"kind":"ToolRequested","timestamp":"t","payload":{}}"#,
            "this is not json",
            r#"{"event_id":"b","sequence":2,"kind":"TotallyUnknown","timestamp":"t","payload":{}}"#,
            r#"{"event_id":"c","sequence":3,"kind":"PolicyDenied","timestamp":"t","payload":{}}"#,
        ]);
        let r = ingest(&p).unwrap();
        assert_eq!(r.events.len(), 3);
        assert_eq!(r.skipped_bad_lines, 1);
        assert_eq!(r.skipped_unknown_kind, 1); // TotallyUnknown
        assert_eq!(r.events[2].parsed_kind(), Some(EventKind::PolicyDenied));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn flags_non_monotonic_sequences() {
        let p = write_tmp(&[
            r#"{"event_id":"a","sequence":5,"kind":"ToolRequested","timestamp":"t","payload":{}}"#,
            r#"{"event_id":"b","sequence":3,"kind":"ToolCompleted","timestamp":"t","payload":{}}"#,
            r#"{"event_id":"c","sequence":3,"kind":"ToolFailed","timestamp":"t","payload":{}}"#,
        ]);
        let r = ingest(&p).unwrap();
        assert_eq!(r.non_monotonic, 2); // 3<=5 and 3<=3
        assert_eq!(r.events.len(), 3); // kept, but flagged
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn tail_returns_most_recent() {
        let p = write_tmp(&[
            r#"{"event_id":"a","sequence":1,"kind":"ToolRequested","timestamp":"t","payload":{}}"#,
            r#"{"event_id":"b","sequence":2,"kind":"ToolCompleted","timestamp":"t","payload":{}}"#,
            r#"{"event_id":"c","sequence":3,"kind":"PolicyChecked","timestamp":"t","payload":{}}"#,
        ]);
        let r = ingest(&p).unwrap();
        let t = tail(&r, 2);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].event_id, "b");
        assert_eq!(t[1].event_id, "c");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn empty_and_blank_lines_are_fine() {
        let p = write_tmp(&["", "   "]);
        let r = ingest(&p).unwrap();
        assert_eq!(r.events.len(), 0);
        assert_eq!(r.skipped_bad_lines, 0);
        let _ = std::fs::remove_file(&p);
    }
}
