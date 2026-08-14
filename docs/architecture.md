# SheshAOS — Architecture

See the architecture overview in the project root `README.md` and the full handover brief in `HANDOVER.md`.

## Layers

```mermaid
---
title: SheshAOS architecture layers
---
graph TB
    subgraph interface["Interface Layer"]
        CLI["🖥️ CLI<br/>shesh-cli"]
        WAVE["🌊 Wave Terminal<br/>(stock, ADR-0016)"]
        RPC["🔌 RPC<br/>shesh-rpc"]
    end
    subgraph kernel["Kernel Core"]
        K["🏛️ Kernel<br/>shesh-kernel"]
        P["🛡️ Policy Engine"]
        R["🔀 Task Router"]
        S["⏰ Scheduler"]
    end
    subgraph model["Model Layer"]
        PL["📋 Planner"]
        CO["💻 Coder"]
        VI["👁️ Vision"]
    end
    subgraph exec["Execution Layer"]
        T["🔧 Tool Broker"]
        B["🧱 Block Controller<br/>shesh-blockctl"]
        RM["🌐 Remote Shell<br/>shesh-remote"]
    end
    subgraph storage["Storage Layer"]
        WO["📦 WaveObj Store<br/>shesh-waveobj"]
        WP["📡 Pub/Sub Broker<br/>shesh-wps"]
        ES["📝 Event Store"]
        SN["📸 Snapshots"]
    end
    CLI --> K
    WAVE --> RPC
    RPC --> K
    K -->|validates via| P
    K -->|routes via| R
    K -->|schedules via| S
    R -->|plans with| PL
    PL -->|delegates to| CO
    PL -->|requests vision from| VI
    K -->|dispatches to| T
    T -->|controls| B
    T -->|proxies| RM
    K -->|persists to| WO
    WO -->|publishes via| WP
    K -->|appends to| ES
    K -->|checkpoints to| SN
```

### Quick Reference

1. **Kernel** — Task intake, governance, scheduling, state transitions, audit
2. **Router** — Intent classification, model selection
3. **Policy Engine** — Deny-by-default action gating
4. **Model Providers** — Swappable specialist inference (planner, coder, vision)
5. **Tool Broker** — Filesystem, Git, Terminal with capability checks
6. **Event Store** — Append-only JSONL event log
7. **CLI** — Terminal interface for all kernel operations

## Control Flow

```mermaid
---
title: SheshAOS task control flow
---
sequenceDiagram
    participant U as User
    participant C as CLI
    participant K as Kernel
    participant R as Router
    participant P as Policy Engine
    participant M as Provider
    participant T as Tool Broker
    participant E as Event Store
    U->>C: command
    C->>K: submit task
    K->>E: append TASK_RECEIVED
    K->>R: classify intent
    R-->>K: intent + model selection
    K->>P: check action
    P-->>K: allow / deny
    K->>M: request proposal
    M-->>K: proposal (untrusted)
    K->>T: execute approved step
    T-->>K: result
    K->>E: append TASK_COMPLETE
    K-->>C: result
    C-->>U: output
```

```text
User → CLI → Kernel → Router (classify) → Policy (check) → Provider (infer)
                ↓                                               ↓
          Event Store ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← Tool Broker (execute)
```

## Trust Boundaries

- User input: **untrusted**
- Model output: **untrusted** (proposals only)
- Tool results: **partially trusted** (logged and validated)
- Event store: **trusted** (append-only, checksummed)

## Key Design Rules

- Models propose, never execute
- Tools execute, never decide
- Kernel validates everything
- Every state change is an event
- Every event is durable
