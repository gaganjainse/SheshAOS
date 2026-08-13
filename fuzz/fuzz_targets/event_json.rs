#![no_main]
// Fuzz target: event JSON deserialization must never panic on arbitrary input.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<shesh_kernel::events::Event, _> = serde_json::from_str(s);
        let _: Result<shesh_kernel::events::EventPayload, _> = serde_json::from_str(s);
        let _: Result<shesh_kernel::events::EventKind, _> = serde_json::from_str(s);
        let _: Result<shesh_kernel::kernel_ingest::NexusEvent, _> = serde_json::from_str(s);
    }
});
