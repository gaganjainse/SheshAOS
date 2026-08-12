# 📝 Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added ✨

- Initial project structure with 12 workspace crates
- CI/CD pipelines (lint, test, build, security, benchmarks)
- Architecture documentation with Mermaid diagrams
- 1000+ test suite
- Benchmark suite with Criterion

### Changed 🔄

- Improved test organization across all crates
- Enhanced architecture validation in CI

### Fixed 🐛

- Removed orphaned `src/` directory
- Fixed hanging test in approval modal
- Fixed clippy warnings across all crates

### Security 🔒

- Added security policy and vulnerability reporting process

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

[Unreleased]: https://github.com/shesh/SheshAOS/compare/v0.1.0...HEAD
[v0.1.0]: https://github.com/shesh/SheshAOS/releases/tag/v0.1.0
