# Kernel Decomposition and Fix Plan

## Remaining Issues (11 total)

### Critical Priority

1. **Critical #1**: Wrap `SqliteEventStore` rusqlite calls in `tokio::task::spawn_blocking`
   - Current code uses `std::sync::Mutex` directly, blocking the async runtime
   - Change `conn` field from `std::sync::Mutex<rusqlite::Connection>` to `rusqlite::Connection`
   - Wrap every rusqlite call in `tokio::task::spawn_blocking(move || { ... })`

2. **Critical #2**: Change `INSERT OR REPLACE INTO events` → `INSERT INTO events`
   - Silent overwrites can mask bugs in event ordering
   - Change line 106 in `sqlite_event_store.rs`

3. **Critical #3**: Change `edition = "2024"` → `edition = "2021"` in `Cargo.toml`
   - Rust edition 2024 is not supported by the toolchain

### High Priority

4. **High #7**: Simplify `query_intel_vram()` to return `(0, 0)`
   - Complex parsing of `intel_gpu_top` output is unreliable
   - Replace function body with `fn query_intel_vram() -> (u64, u64) { (0, 0) }`

5. **High #8**: Remove deprecated `wmic` Windows block from `query_disk_space()`
   - `wmic` is deprecated on Windows; use `sysinfo::Disks` only
   - Remove the `#[cfg(target_os = "windows")]` block

6. **High #9**: Remove `event_store` TODO comments
   - Search for TODO comments in sqlite_event_store.rs and remove them

### Medium Priority

7. **Medium #12**: Change `health_check_all` to use `catch_unwind` instead of `tokio::task::spawn`
   - `tokio::task::spawn` can panic the runtime if a provider misbehaves
   - Use `futures::future::catch_unwind` for isolation

8. **Medium #14**: Collapse identical if blocks in `context.rs`
   - Find and collapse duplicated match/arm patterns

9. **Medium #15**: Collapse if blocks in `resource.rs`
   - Find and collapse duplicated match/arm patterns

10. **Medium #16**: Remove `#[allow(unreachable_patterns)]` from `state.rs`
    - The `_ => vec![]` catch-all makes this attribute unnecessary

11. **Medium #17**: `Kernel::new` should take `Arc<RwLock<PolicyEngine>>` directly
    - Currently takes `Arc<PolicyEngine>` and wraps in a new RwLock internally
