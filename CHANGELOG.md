# 📝 Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added ✨

- Real Criterion benches for the three hot paths: kernel event store
  (append/read), WaveObj store (insert/get), WPS broker (publish/route
  matching) — replacing a *claimed* bench suite that lived in a dead,
  non-workspace test crate and never ran in CI.
- Workspace-wide lint opt-in: every crate now carries `[lints] workspace = true`,
  so the no-unwrap/expect/panic policy (workspace lints + clippy.toml's
  allow-*-in-tests) applies everywhere, enforced tool-natively by clippy.

### Changed 🔄

- `shesh` CLI is headless: `tui` and `pty` subcommands removed (bare `shesh`
  prints help). The interactive mission-control surface is stock Wave
  Terminal (ADR-0016).
- `vault` command now resolves `~` properly and reports an unreadable vault
  instead of silently reporting zero snippets.
- CI security job uses `rustsec/audit-check@v2` with explicit
  `checks: write` permission (the previous step invoked `cargo audit`
  without ever installing it).
- CI architecture-validation gate quoting fixed (backtick pattern was parsed
  as command substitution by bash — it never could have passed).

### Removed 🗑️

- **crates/shesh-tui, crates/shesh-gui**: bespoke ratatui/iced frontends
  superseded by the Wave-adoption decision (ADR-0016); `shesh-gui` had zero
  consumers, `shesh-tui` only fed the CLI's default command.
- **crates/shesh-terminal (incl. build.rs + zig_src/)**: the "native Zig VT100
  parser" was a 71-line line/byte counter called only from a demo `shesh pty`
  printout; `PtyManager` had no callers (blockctl uses portable-pty directly).
- **top-level zig/**: orphaned pre-Rust kernel attempt (event_store /
  scheduler / snapshot reimplemented in Zig), referenced by nothing.
  Removing it and shesh-terminal restores ADR-0001 ("no Zig/FFI in the main
  build") and drops the Zig toolchain requirement from CI and bootstrap.
- **tests/ (`shesh-tests` dead crate)**: not a workspace member, unbuilt and
  unrun — hardcoded `/home/gagan/Workspace/SheshAOS` paths, referenced the
  removed crates. Real coverage lives in-crate; CLI black-box tests will be
  rebuilt with assert_cmd when the CLI surface settles.
- **metadata.json, scratch.rs**: unreferenced root cruft (an abandoned iced
  playground file).
- `NexusError`/`NexusApp` identifiers renamed to `KernelError`/`SheshApp`
  (canonical naming; no grandfathered aliases).

### Fixed 🐛

- Production `unwrap()` in kernel task execution replaced by `let-else` with
  an honest `Coder provider not available` failure event.
- `WaveStore::open*` documentation clarified (tables ensured on open).


---

## [v0.1.0] - 2026-08-01

### Added ✨

- **Kernel**: Governance microkernel with policy engine
- **WaveObj**: Object store with SQLite persistence
- **WPS**: Pub/Sub event broker with scoping
- **BlockCtl**: PTY shell controller
- **Terminal**: Zig VT100 parser + PTY bridge
- **AI**: OpenAI/Anthropic streaming providers
- **Remote**: SSH client with connection monitoring
- **RPC**: Unix socket JSON-RPC 2.0
- **GUI**: Iced native desktop interface
- **TUI**: Ratatui terminal interface
- **Vault**: Command snippets and flag inspector
- **WConfig**: Config watcher with live reload

### Documentation 📚

- README with architecture overview
- CONTRIBUTING guidelines
- SECURITY policy
- CODE_OF_CONDUCT
- Architecture diagrams (Mermaid)

---

[//]: # (Add your release notes here following Keep a Changelog format)

[Unreleased]: https://github.com/gaganjainse/SheshAOS/compare/v0.1.0...HEAD
[v0.1.0]: https://github.com/gaganjainse/SheshAOS/releases/tag/v0.1.0
