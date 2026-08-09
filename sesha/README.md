# NexusAOS v2

NexusAOS v2 is a governance-first, event-sourced AI operating environment for Ubuntu Linux.

## Core Principles

- **Kernel-Centric**: The kernel owns truth, governance, execution, and audit.
- **Model Agnostic**: Models propose actions; they do not execute them.
- **Local-First**: Designed for execution on local hardware with constrained resources.
- **Event-Sourced**: Every action is auditable, reversible, and permissioned.

## Tech Stack

- **Language**: Rust
- **OS**: Ubuntu 26.04 (Wayland/GNOME)
- **Specialist Models**:
  - Gemma 4 12B (Planner)
  - Qwen3-Coder 30B (Implementation)
  - Qwen3.5 9B (Vision)

## Getting Started

Refer to `docs/architecture.md` for the system design and `docs/manifest-spec.md` for agent definitions.
