#![no_main]
// Fuzz target: config TOML parsing must never panic on arbitrary input.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = shesh_kernel::config::AppConfig::parse_toml(s);
    }
});
