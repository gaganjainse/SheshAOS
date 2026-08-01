# NexusAOS v2

**Governance-first, event-sourced AI operating environment for Ubuntu Linux.**

NexusAOS is a microkernel-like system that routes tasks to specialist local AI models (planner, coder, vision), enforces policy on every action, and keeps an append-only audit trail of every state change.

## Design Principles

- **Kernel owns truth.** Models propose actions; the kernel validates, constrains, and records.
- **Event sourcing.** Every state change is an append-only event. Current state is derived, never mutated directly.
- **Governance first.** All actions pass through policy checks. Destructive operations require explicit confirmation.
- **Local first.** Core operations work offline. No cloud dependencies.
- **Models are replaceable.** The kernel speaks to a provider interface, not to specific model runners.

## Architecture

```
┌─────────────────────────────────────────────┐
│                    CLI                       │
├─────────────────────────────────────────────┤
│                  Kernel                      │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ Scheduler│ │  Policy  │ │   Router    │ │
│  └──────────┘ └──────────┘ └─────────────┘ │
├─────────────────────────────────────────────┤
│          Model Providers                     │
│  ┌────────┐ ┌────────┐ ┌────────┐          │
│  │Planner │ │ Coder  │ │ Vision │          │
│  └────────┘ └────────┘ └────────┘          │
├─────────────────────────────────────────────┤
│           Tool Broker                        │
│  ┌──────┐ ┌─────┐ ┌──────────┐             │
│  │  FS  │ │ Git │ │ Terminal │             │
│  └──────┘ └─────┘ └──────────┘             │
├─────────────────────────────────────────────┤
│          Event Store                         │
│  ┌────────┐ ┌───────────┐ ┌────────────┐   │
│  │ Events │ │ Snapshots │ │Projections │   │
│  └────────┘ └───────────┘ └────────────┘   │
└─────────────────────────────────────────────┘
```

## Hardware Target

- Intel i7-14700HX
- NVIDIA RTX 4050 (6 GB VRAM)
- 16 GB RAM
- Ubuntu 26.04 LTS

## Model Stack

| Role | Model | Use Case |
|------|-------|----------|
| Planner | Gemma 4 12B Agentic Fable Q4_K_M | Architecture, planning, review |
| Coder | Qwen3-Coder 30B-A3B Q4_K_M | Implementation, debugging, tests |
| Vision | Qwen3.5 9B | Screenshots, diagrams, documents |

## Quick Start

```bash
cargo build --release
./target/release/nexusaos init
./target/release/nexusaos doctor
./target/release/nexusaos run "describe the project structure"
```

## License

MIT

## Functional Specification

- **Configuration Management**: Users can now modify their terminal, AI, and editor settings inside `~/.config/waveterm/settings.json`. The application watches this file and instantly reloads the configuration across all active terminal blocks and UI components without requiring a restart.
- **Dynamic Terminal UI**: The terminal interface now directly responds to real-time events. As users type, input is instantly routed to the underlying shell process, and layout splits are dynamically arranged based on the workspace's saved configuration state. Visuals update instantly when background events occur.
- **AI Chat Engine**: Users can now converse with AI assistants seamlessly. The built-in AI engine directly streams tokens down from OpenAI-compatible and Anthropic endpoints directly into the TUI, allowing for rapid real-time response generation without blocking UI updates.
- **Native SSH & Remote Management**: Users can connect securely to remote servers directly from the terminal without external SSH binaries. The environment automatically monitors connection health in the background and correctly tunnels PTY shell interactions over the multiplexed SSH channel.
- **IPC & RPC Control Layer**: The terminal daemon exposes a JSON-RPC 2.0 compatible socket (UDS/Named Pipe), allowing external tools, scripts, and alternative frontends to query internal state, manipulate UI layouts, or broadcast events into the active session programmatically.
- **Native GUI (Iced)**: In addition to the terminal UI (TUI), NexusAOS now provides a fully native desktop GUI built with `iced`. It leverages the identical underlying Rust architecture to deliver a high-performance, GPU-accelerated windowed interface featuring modern split-pane layouts.
