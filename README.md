---
title: SheshAOS
description: Governance-first, event-sourced AI operating environment in Rust
lang: en
---

## 🦀 SheshAOS

![Rust](https://img.shields.io/badge/Rust-2024-important?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)
![Version](https://img.shields.io/badge/Version-0.1.0-green?style=for-the-badge&logo=git-semver)
![Status](https://img.shields.io/badge/Status-Production%20Ready-success?style=for-the-badge)
![CI/CD](https://img.shields.io/badge/CI%2FCD-GitHub%20Actions-blue?style=for-the-badge&logo=github)
![Tests](https://img.shields.io/badge/Tests-877-success?style=for-the-badge)

**🏛️ Governance-first, event-sourced AI operating environment in Rust —
local-first and Linux-native.**

[📖 Docs](docs/architecture.md)  
[🤝 Contributing](CONTRIBUTING.md)  
[🔒 Security](SECURITY.md)  
[📝 Changelog](CHANGELOG.md)  
[💬 Discussions](https://github.com/gaganjainse/SheshAOS/discussions)

---

## 📋 About

**SheshAOS** is a production-ready, open-source AI operating
environment that combines local LLM inference, terminal
emulation, SSH multiplexing, and governance-first task
execution into a unified Rust system.

### 🎯 Mission

To provide a **governance-first AI platform** where:

- Models propose actions; the kernel validates and records
- Every state change is append-only and auditable
- Destructive operations require explicit policy approval
- Core operations work offline without cloud dependencies
- AI providers are replaceable via a common interface

### 📊 Project Stats

| Metric | Value |
| ------- | ----- |
| 🦀 **Language** | Rust 2024 |
| 📦 **Crates** | 9 workspace crates + `shesh` CLI |
| 🧪 **Tests** | 877 passing |
| 🔍 **Lints** | 0 warnings |
| 🚀 **CI/CD** | GitHub Actions |
| 📄 **License** | MIT |
| 🏷️ **Status** | Production Ready |

### 🌟 What Makes SheshAOS Different

| Traditional AI Tools | SheshAOS |
| ------------------- | -------- |
| Cloud-dependent | 💻 **Local-first** — works offline |
| No oversight | 🛡️ **Governance-first** — kernel validates everything |
| Mutable state | 📝 **Event-sourced** — append-only audit trail |
| Single model lock-in | 🔌 **Provider interface** — replaceable models |
| No terminal integration | 🌊 **Wave-native** — stock Wave Terminal frontend (ADR-0016) |

### 🏗️ System Architecture

```mermaid
graph TB
    subgraph "Interface Layer"
        CLI["🖥️ CLI<br/>shesh-cli"]
        WAVE["🌊 Wave Terminal<br/>(stock, ADR-0016)"]
        RPC["🔌 RPC<br/>shesh-rpc"]
    end

    subgraph "Kernel Core"
        K["🏛️ Kernel<br/>shesh-kernel"]
        P["🛡️ Policy Engine"]
        R["🔀 Task Router"]
        S["⏰ Scheduler"]
    end

    subgraph "Model Layer"
        PL["📋 Planner"]
        CO["💻 Coder"]
        VI["👁️ Vision"]
    end

    subgraph "Execution Layer"
        T["🔧 Tool Broker"]
        B["🧱 Block Controller"]
        RM["🌐 Remote Shell"]
    end

    subgraph "Storage Layer"
        WO["📦 WaveObj Store"]
        WP["📡 Pub/Sub Broker"]
        ES["📝 Event Store"]
    end

    CLI --> K
    WAVE --> RPC
    RPC --> K

    K --> P
    K --> R
    K --> S

    R --> PL
    PL --> CO
    CO --> VI

    K --> T
    K --> B
    K --> RM

    K --> WO
    K --> WP
    K --> ES
```

### 🚀 Quick Start

```bash
# Clone
git clone https://github.com/gaganjainse/SheshAOS.git
cd SheshAOS

# Build
cargo build --release

# Initialize
./target/release/shesh init

# Run
./target/release/shesh run "describe the project structure"
```

### 🧪 Quality Metrics

| Check | Status |
| ----- | ------ |
| ✅ Compilation | 0 errors, 0 warnings |
| ✅ Lints | 0 clippy warnings |
| ✅ Tests | 877 passing |
| ✅ Benchmarks | 6 criterion benches |
| ✅ CI/CD | Full pipeline configured |
| ✅ Security | Policy + audit + scanning |

### 📚 Documentation

| Document | Purpose |
| --------- | -------- |
| [📖 Architecture](docs/architecture.md) | System diagrams and data flows |
| [🤝 Contributing](CONTRIBUTING.md) | Development workflow |
| [🔒 Security](SECURITY.md) | Vulnerability reporting |
| [📝 Changelog](CHANGELOG.md) | Version history |
| [📋 Handover](HANDOVER.md) | Developer transition guide |
| [🤗 Code of Conduct](CODE_OF_CONDUCT.md) | Community standards |

### 🌐 Topics

`rust` `ai` `governance` `event-sourcing` `microkernel` `ssh`
`pty` `sqlite` `local-first` `privacy` `open-source`

---

## 🎯 Overview

SheshAOS is a **microkernel-like system** that routes tasks to
specialist local AI models (planner, coder, vision), enforces
policy on every action, and keeps an **append-only audit trail**
of every state change.

### Why SheshAOS?

| Problem | Solution |
| -------- | -------- |
| 🤖 AI lacks oversight | ✅ **Governance-first**: Kernel validates actions |
| 📝 State is mutable | ✅ **Event sourcing**: Append-only log |
| 🔌 Cloud-dependent AI tools | ✅ **Local-first**: Works offline, no cloud |
| 🔒 Destructive ops need approval | ✅ **Policy engine**: Actions pass checks |
| 🧩 Locked to one model | ✅ **Provider interface**: Models are swappable |

### Design Principles

```mermaid
graph LR
    A["🏛️ Kernel owns truth"] --> B["📝 Event sourcing"]
    B --> C["🛡️ Governance first"]
    C --> D["💻 Local first"]
    D --> E["🔌 Models are replaceable"]
```

---

## ✨ Key Features

### 🧠 AI Chat Engine

- **Streaming responses** from OpenAI-compatible and Anthropic endpoints
- **Real-time token streaming** surfaced through Wave blocks / MCP clients
- **Multi-modal support** with vision capabilities
- **Session management** with full conversation history

### 🧱 Block & Shell Control

- **PTY block controller** with backpressure-aware reading — the layer Wave blocks ride on
- **Remote PTY shell tunneling** via russh

### 🔐 Security & Governance

- **Policy engine** with trust tiers and capability-based security
- **Approval modals** for destructive operations
- **Append-only event store** with cryptographic integrity
- **SSH multiplexing** with connection monitoring

### 🌐 Remote Management

- **Native SSH client** via `russh`
- **Connection health monitoring**
- **Remote PTY shell tunneling**
- **Config watcher** with live reload

### 🎨 User Interfaces

- **Wave Terminal (stock)** — mission-control surface; bespoke ratatui/iced frontends were removed 2026-08-12 (ADR-0016, ADR-0018)
- **CLI**: full-featured `shesh` command-line interface
- **IPC**: JSON-RPC 2.0 over Unix sockets

---

## 🏗️ Architecture

### High-Level Architecture

```mermaid
graph TB
    subgraph "Interface Layer"
        CLI["🖥️ CLI<br/>shesh-cli"]
        WAVE["🌊 Wave Terminal<br/>(stock, ADR-0016)"]
        RPC["🔌 RPC<br/>shesh-rpc"]
    end

    subgraph "Kernel Core"
        KERNEL["🏛️ Kernel<br/>shesh-kernel"]
        POLICY["🛡️ Policy Engine"]
        ROUTER["🔀 Task Router"]
        SCHEDULER["⏰ Scheduler"]
    end

    subgraph "Model Layer"
        PLANNER["📋 Planner"]
        CODER["💻 Coder"]
        VISION["👁️ Vision"]
        AI["🤖 AI Engine"]
    end

    subgraph "Execution Layer"
        TOOLS["🔧 Tool Broker"]
        BLOCKCTL["🧱 Block Controller"]
        REMOTE["🌐 Remote Shell"]
    end

    subgraph "Storage Layer"
        WAVEOBJ["📦 WaveObj Store"]
        WPS["📡 Pub/Sub Broker"]
        EVENTSTORE["📝 Event Store"]
        SNAPSHOT["📸 Snapshots"]
    end

    CLI --> KERNEL
    WAVE --> RPC
    RPC --> KERNEL

    KERNEL --> POLICY
    KERNEL --> ROUTER
    KERNEL --> SCHEDULER

    ROUTER --> PLANNER
    PLANNER --> CODER
    CODER --> VISION

    KERNEL --> TOOLS
    KERNEL --> BLOCKCTL
    KERNEL --> REMOTE

    KERNEL --> WAVEOBJ
    KERNEL --> WPS
    KERNEL --> EVENTSTORE
    EVENTSTORE --> SNAPSHOT

    AI --> PLANNER
    AI --> CODER
    AI --> VISION
```

### Runtime Data Flow

```mermaid
graph LR
    A["📥 Submit Task"] --> B["🔍 Dedup Check"]
    B --> C["🛡️ Policy Check"]
    C --> D["🔀 Route Task"]
    D --> E["📋 Plan"]
    E --> F["💻 Code"]
    F --> G["👁️ Review"]
    G --> H["🔧 Execute Tools"]
    H --> I["📝 Record Events"]
    I --> J["💾 Update State"]
    J --> K["📸 Snapshot"]
```

### Wave Object Model

```mermaid
graph TD
    A["WaveObj trait"] --> B["Block"]
    A --> C["Job"]
    A --> D["Window"]
    A --> E["Workspace"]
    A --> F["Tab"]
    A --> G["LayoutState"]

    B -->|parent| F
    B -->|children| B

    H["ORef"] -->|references| A
    I["MetaMap"] -->|metadata| A

    J["WaveStore"] -->|persists| A
```

---

## 🛠️ Tech Stack

### Core Technologies

| Category | Technology | Purpose |
| -------- | ---------- | ------- |
| **Language** | Rust 2024 | Core implementation |
| **Async Runtime** | Tokio | Async execution |
| **Serialization** | Serde / JSON | Data interchange |
| **PTY** | portable-pty | Block shell process management |
| **AI/ML** | reqwest + SSE | Streaming providers |
| **SSH** | russh | Remote connections |
| **Persistence** | SQLite (rusqlite) | Object storage |
| **Policy** | Custom engine | Governance |
| **Observability** | tracing | Logging/metrics |

### External Integrations

| Integration | Type | Purpose |
| ---------- | ---- | ------- |
| OpenAI-compatible APIs | HTTP/SSE | LLM streaming |
| Anthropic API | HTTP/SSE | Claude models |
| SSH servers | Network | Remote execution |
| Unix sockets | IPC | External control |
| File watcher | OS | Config hot-reload |

---

## 🖥️ Hardware Target

| Component | Specification |
| ---------- | ------------- |
| **CPU** | Intel i7-14700HX |
| **GPU** | NVIDIA RTX 4050 (6 GB VRAM) |
| **Memory** | 16 GB RAM |
| **OS** | Linux (primary target: CachyOS/Arch + Hyprland) |
| **Storage** | NVMe SSD recommended |

---

## 🤖 Model Stack

| Role | Model | Quantization | Use Case |
| ----- | ------ | ------------ | -------- |
| 📋 **Planner** | Gemma 4 12B | Q4_K_M | Architecture, planning, review |
| 💻 **Coder** | Qwen3-Coder 30B | Q4_K_M | Implementation, debugging, tests |
| 👁️ **Vision** | Qwen3.5 9B | Q4_K_M | Screenshots, diagrams, documents |

---

## 🚀 Quick Start Guide

### Prerequisites

- Rust 1.75+ (edition 2024)
- Linux (CachyOS/Arch primary; any Linux with a Rust toolchain)
- 16 GB RAM minimum
- NVIDIA GPU optional (local model inference)

### Installation

```bash
# Clone the repository
git clone https://github.com/gaganjainse/SheshAOS.git
cd SheshAOS

# Build the project
cargo build --release

# Run initialization
./target/release/shesh init

# Check system health
./target/release/shesh doctor

# Show kernel state
./target/release/shesh status

# Run a task
./target/release/shesh run "describe the project structure"
```

### Development Setup

```bash
# Install dependencies
cargo build

# Run all tests
cargo test --workspace

# Run lints
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Run benchmarks
cargo bench --workspace
```

---

## 📁 Project Structure

```text
SheshAOS/
├── .github/                    # GitHub Actions, templates, dependabot
│   ├── workflows/             # CI/CD pipelines
│   ├── ISSUE_TEMPLATE/        # Issue templates
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── CODEOWNERS
│   └── BRANCH_PROTECTION.md
├── bin/shesh-cli/          # CLI binary entrypoint
├── crates/
│   ├── shesh-kernel/       # 🏛️ Core governance microkernel
│   ├── shesh-waveobj/      # 📦 Object store & ORef graph
│   ├── shesh-wps/          # 📡 Pub/Sub event broker
│   ├── shesh-blockctl/     # 🧱 PTY shell controller
│   ├── shesh-ai/           # 🤖 OpenAI/Anthropic streaming
│   ├── shesh-remote/       # 🌐 SSH remote shell
│   ├── shesh-rpc/          # 🔌 Unix socket JSON-RPC
│   ├── shesh-vault/        # 🔐 Command snippets & inspector
│   └── shesh-wconfig/      # ⚙️ Config watcher & settings
├── configs/                   # Configuration files
├── scripts/                   # Dev/test helper scripts
├── docs/                      # Additional documentation
├── docs/architecture.md      # 🏗️ System architecture
├── Cargo.toml                 # Workspace definition
├── Makefile                   # Build shortcuts
├── .clippy.toml               # Lint configuration
├── rustfmt.toml               # Format configuration
├── CONTRIBUTING.md            # Contribution guidelines
├── CODE_OF_CONDUCT.md         # Community standards
├── SECURITY.md                # Security policy
├── CHANGELOG.md               # Version history
└── README.md                  # This file
```

---

## 🧪 Testing

### Test Coverage

| Crate | Tests |
| ----- | ----- |
| shesh-kernel | 406 |
| shesh-waveobj | 204 |
| shesh-wps | 71 |
| shesh-blockctl | 48 |
| shesh-ai | 18 |
| shesh-rpc | 29 |
| shesh-remote | 16 |
| shesh-vault | 54 |
| shesh-wconfig | 31 |
| **Total** | **877** |

### Running Tests

```bash
# Unit tests
cargo test --lib --workspace

# Integration tests
cargo test --workspace --tests

# Doc tests
cargo test --workspace --doc

# All tests
cargo test --workspace

# With coverage
cargo test --workspace -- --nocapture
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench --workspace

# Specific benchmark
cargo bench -p shesh-kernel bench_kernel_task_submission
```

| Benchmark | Where | Description |
| --------- | ----- | ----------- |
| `bench_event_store` | crates/shesh-kernel/benches | Event append/read throughput |
| `bench_wavestore` | crates/shesh-waveobj/benches | WaveObj put/get latency |
| `bench_broker_throughput` | crates/shesh-wps/benches | Pub/sub routing throughput |

---

## 📚 Docs

- **📖 Architecture**: `docs/architecture.md` — Complete system diagrams
- **🤝 Contributing**: `CONTRIBUTING.md` — Development workflow
- **🔒 Security**: `SECURITY.md` — Vulnerability reporting
- **📝 Changelog**: `CHANGELOG.md` — Version history
- **📋 Handover**: `HANDOVER.md` — Developer transition guide

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md)
for detailed guidelines.

### Quick Contribution Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] No `unwrap()` or `expect()` in production code
- [ ] All new public functions have tests
- [ ] PR title follows conventional commits

### Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md).
By participating, you agree to uphold a welcoming and inclusive
environment.

### License

This project is licensed under the [MIT License](LICENSE).

---

## 🙏 Acknowledgments

- **Alacritty** — VTE parser integration patterns
- **WezTerm** — GPU-accelerated rendering architecture
- **Warp** — AI streaming UI patterns
- **Kitty** — PTY backpressure handling
- **Ghostty** — Modern terminal rendering
- **Tabby** — Remote shell architecture

---

### Built with ❤️ by the SheshAOS Team

[GitHub](https://github.com/gaganjainse/SheshAOS) • [Issues](https://github.com/gaganjainse/SheshAOS/issues)
• [Discussions](https://github.com/gaganjainse/SheshAOS/discussions)
