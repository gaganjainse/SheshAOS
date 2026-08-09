# NexusAOS v2 — Architecture Brief

## 1. Executive Summary

NexusAOS v2 should be designed as a governance-first, event-sourced AI operating environment for Ubuntu Linux. The system is not a chatbot wrapper and not a monolithic agent. It is a small kernel plus a set of well-bounded specialist services that route tasks to local models, tools, and persistent state safely.

The practical constraints matter: 16 GB RAM, 6 GB VRAM, Ubuntu 26.04, local-first execution, and only three chosen models. That means the architecture must assume model switching, not simultaneous residency of everything. The system should keep the kernel lightweight, make models replaceable, and treat every action as auditable, reversible, and permissioned.

The core design choice is: **the kernel owns truth, governance, execution, and audit; models only propose actions**. The kernel never trusts model output blindly. It validates, constrains, serializes, checkpoints, and records every change.

## 2. Assumptions

- Host is Ubuntu 26.04 LTS on GNOME/Wayland.
- Hardware is an i7-14700HX, RTX 4050 6 GB, 16 GB RAM, and SSD storage.
- Only one or a small number of models can be active at once.
- The local model stack is fixed for now:
  - Gemma 4 12B Agentic Fable Q4\_K\_M: planner, architect, reviewer.
  - Qwen3-Coder 30B-A3B Q4\_K\_M: implementation, debugging, refactoring.
  - Qwen3.5 9B vision-capable: screenshots, diagrams, documents, general vision assistant.
- The system must work offline for core operations.
- The user prefers a Rust-centered implementation and low-maintenance design.
- Backend/model-provider abstraction must allow later migration away from the initial runner without redesigning the kernel.

## 3. System Goals and Non-Goals

### Goals

- Route tasks to the right specialist model automatically.
- Maintain governance, permissions, and auditability.
- Support planning, coding, vision, and review workflows.
- Support rollback, replay, checkpoints, and deterministic history.
- Operate safely on constrained hardware.
- Be local-first and usable offline.
- Keep the implementation maintainable and replaceable.
- Preserve a clean separation between kernel, model providers, tools, and memory.

### Non-Goals

- Do not build a general-purpose desktop replacement.
- Do not require all models to remain loaded simultaneously.
- Do not assume cloud connectivity.
- Do not embed business logic inside models.
- Do not make the kernel dependent on a single runner, model vendor, or UI.
- Do not optimize for maximum feature count over reliability.

## 4. Architecture Options Considered

### Option A: One monolithic agent app

Pros: simpler start, less surface area.

Cons: poor separation of concerns, hard to secure, hard to test, hard to evolve, brittle under model failure.

Decision: reject.

### Option B: Thin orchestrator around a single model runner

Pros: easy to ship.

Cons: model lock-in, poor specialization, hard to support vision and coding separately, weak governance.

Decision: reject.

### Option C: Kernel + specialist models + tool layer + event bus

Pros: fits the actual use case, supports governance, supports multiple models, enables replacement of backends, and scales in complexity gradually.

Cons: more up-front design work, more internal interfaces to define.

Decision: choose this.

## 5. Chosen Architecture

The chosen design is a **microkernel-like local AI operating environment**.

### High-level layers

1. **Kernel**

   - Owns task intake, governance, permissions, scheduling, state transitions, checkpointing, and audit.

2. **Router / Policy Layer**

   - Decides which model or tool should handle a given task.
   - Enforces allowed actions and model selection policy.

3. **Specialist Model Runners**

   - Planner/reviewer model.
   - Coder model.
   - Vision model.
   - These are treated as replaceable providers.

4. **Tool Layer**

   - Filesystem.
   - Git.
   - Terminal.
   - Search/fetch.
   - Docker.
   - Future MCP-compatible services.

- MCP and ACP integration points.

5. **Memory and Audit Layer**

   - Event store.
   - Conversation summaries.
   - Project state.
   - Checkpoints.
   - Replay data.

6. **UI / Client Layer**

   - May be a desktop app, terminal, web UI, or IDE integration.
   - UI is not the system of record. IDE integrations should use ACP as a client protocol, not as an authority boundary.

## 6. Component-by-Component Design

### 6.1 Kernel

Responsibilities:

- Accept tasks.
- Assign task IDs.
- Track lifecycle state.
- Enforce policy.
- Request model inference.
- Decide when tool execution is permitted.
- Persist all state transitions.
- Support rollback and replay.

Rules:

- Kernel never acts on raw model output without validation.
- Kernel remains small and deterministic where possible.
- Kernel must survive model failure, tool failure, and UI disconnection.

### 6.2 Task Router

Responsibilities:

- Classify intent.
- Select planner, coder, or vision.
- Route ambiguous tasks to planner first.
- Escalate to review when confidence is low.

Routing policy examples:

- Architecture, planning, trade-off analysis → planner.
- Code creation, refactor, bugfix, tests → coder.
- Screenshots, PDFs, diagrams, UI interpretation → vision.
- Mixed tasks → planner first, then coder, then review.

### 6.3 Policy Engine

Responsibilities:

- Define which actions are allowed.
- Restrict filesystem scope.
- Prevent unsafe shell execution.
- Require confirmations for destructive operations.
- Enforce least privilege.

Policy concepts:

- capability grant
- scope
- trust tier
- confirmation requirement
- rollback requirement

### 6.4 Model Provider Interface

Responsibilities:

- Abstract over LM Studio, Unsloth Studio, Ollama, or future backends.
- Normalize chat/inference APIs.
- Report model capabilities.
- Handle load/unload and context sizing.

Required interface shape:

- list available models
- get model metadata
- generate completion
- generate with vision
- stream tokens
- cancel request
- health check
- warmup / unload hints

Important design choice:

- The kernel speaks only to the provider interface, not to any specific runner.

### 6.5 Memory Store

Responsibilities:

- Store projects, tasks, decisions, summaries, and artifacts.
- Support search and retrieval.
- Keep event history append-only.
- Store derived summaries separately from raw events.

Data types:

- event log
- checkpoint
- task summary
- project summary
- tool result
- execution artifact
- policy decision

### 6.6 Tool Layer

Tool categories:

- Filesystem read/write/move/delete.
- Git status/commit/branch/diff.
- Terminal command execution.
- Docker container actions.
- Search/fetch for external docs when permitted.

Tool principles:

- Every tool call is explicit.
- Every tool call is logged.
- Tools have scopes.
- Destructive actions require confirmation or pre-approved policy.
- Tool return values are captured as events.

### 6.7 Replay and Checkpoint System

Responsibilities:

- Capture every state transition.
- Capture every tool result.
- Persist snapshots before risky actions.
- Reconstruct a prior system state for debugging or rollback.

Checkpoint triggers:

- before file writes
- before git commits
- before package installs
- before destructive actions
- before long multi-step tasks

### 6.8 Vision Specialist

Responsibilities:

- Interpret screenshots.
- Read diagrams and PDFs.
- Extract UI state.
- Assist with document understanding.

Constraints:

- Vision model should not directly control system actions.
- Vision outputs must be converted into structured observations before use.

### 6.9 Planner / Architect Specialist

Responsibilities:

- Break goals into steps.
- Define work packages.
- Estimate risk.
- Produce implementation plans.
- Review code and architecture.

Constraint:

- Planner should not write files directly unless explicitly delegated.

### 6.10 Coder Specialist

Responsibilities:

- Write code.
- Refactor code.
- Fix bugs.
- Generate tests.
- Apply changes through tools.

Constraint:

- Coder should not decide product scope or architecture direction without planner input for large tasks.

## 6.11 MCP and ACP Integration

NexusAOS should support both **MCP** and **ACP**, but in different roles.

### MCP

MCP is a good fit for the Tool Layer because it is an open protocol for connecting AI applications to external tools, data sources, and workflows. In NexusAOS, MCP should be used to expose capabilities such as filesystem access, Git, terminal helpers, search/fetch, and future adapters. All MCP requests must still pass through the Policy Engine, Capability Manager, and Kernel. MCP is a transport/interface standard, not a source of authority.

### ACP

ACP is a good fit for IDE and agent-client integration. JetBrains' ACP documentation describes it as a protocol for connecting external agents to the IDE, and JetBrains Air/AI Assistant already supports ACP-compatible agents. In NexusAOS, ACP should be used at the session/client boundary for editor integrations and agent-facing workflows. ACP should never bypass the Kernel, and ACP-connected agents must still receive explicit capabilities before they can act.

### Design rule

- MCP extends **tool access**.
- ACP extends **agent/client interaction**.
- Neither protocol bypasses governance.
- Every request still flows through the Kernel.
- Every side effect is logged as an event.

## 7. Data Flow and Control Flow

### Normal flow

1. User submits a request.
2. Kernel records the request event.
3. Router classifies the task.
4. Policy engine checks allowed actions.
5. Appropriate model generates a proposal.
6. Kernel validates proposal.
7. If tool actions are required, kernel authorizes them.
8. Tools execute and return results.
9. Kernel stores results and updates task state.
10. Planner or reviewer may inspect results.
11. Final output is returned to the user.

### Control flow

- Control starts in the kernel.
- Models propose, never execute.
- Tools execute, never decide.
- Policy gates risky actions.
- Audit records every transition.

## 8. State Model / Lifecycle

Recommended task states:

- received
- classified
- planned
- awaiting\_confirmation
- executing
- blocked
- failed
- rolled\_back
- completed
- archived

State properties:

- state transitions are explicit.
- transitions are append-only events.
- current state is derived from event history.

## 9. Failure Modes and Edge Cases

### Model failures

- model timeout
- model returns malformed output
- model hallucinates unsupported tool usage
- model refuses a task
- model loads too slowly
- model exceeds context window

Mitigation:

- retry with smaller context
- route to fallback model
- ask planner for re-scope
- degrade to partial result

### Tool failures

- command exits nonzero
- filesystem permission denied
- path missing
- git conflict
- network unavailable
- disk full

Mitigation:

- preflight checks
- staged execution
- rollback checkpoints
- error classification

### System failures

- app crash
- power loss
- reboot during execution
- corrupted cache
- backend unavailable

Mitigation:

- event sourcing
- startup recovery scan
- replay from event log
- checkpoint restoration

### User-caused edge cases

- ambiguous request
- conflicting instructions
- destructive request without confirmation
- rapid repeated prompts
- model switch mid-task

Mitigation:

- clarification prompts
- explicit confirmation gates
- task deduplication
- cancellation support

## 10. Security and Privacy

### Trust boundaries

- User input is untrusted.
- Model output is untrusted.
- Tool results are partially trusted but still logged and validated.
- External data is untrusted.

### Security rules

- Capabilities are explicit.
- Shell access is sandboxed or heavily constrained.
- Filesystem access is scoped.
- Destructive operations require confirmation.
- Secrets are never passed to models unless absolutely necessary and redacted when possible.
- Audit logs must not leak sensitive data unnecessarily.

### Privacy

- Local-first by default.
- No cloud telemetry in the core system.
- External model providers are optional adapters, not core dependencies.

## 11. Performance and Resource Analysis

### Constraints

- 16 GB RAM.
- 6 GB VRAM.
- Model switching is cheaper than concurrent residency.
- 8K context is practical for default workloads.

### Strategy

- Load one specialist at a time when possible.
- Keep kernel memory footprint small.
- Cache summaries, not huge raw histories.
- Use event sourcing plus derived views.
- Avoid permanent background residency for all models.

### Bottlenecks

- model loading time
- context growth
- file diff size
- large logs
- tool invocation latency
- repeated review loops

### Trade-off

- Better quality models can be slower.
- Faster models may be used for lightweight tasks or prefiltering.
- The kernel should route based on task importance, not just speed.

## 12. Testing and Validation Plan

### Unit tests

- task classification
- policy decisions
- state transitions
- event serialization
- rollback logic
- provider abstraction

### Integration tests

- model call to tool call loop
- filesystem write and rollback
- git commit and revert
- vision-to-text-to-action flow

### Failure tests

- backend unavailable
- tool returns error
- write permission denied
- disk full
- cancellation mid-task
- task replay after crash

### Acceptance criteria

- tasks are traceable
- actions are reversible where promised
- no silent destructive actions
- the system recovers from restarts
- router chooses specialists consistently

## 13. Rollout / Migration Plan

### Phase 1

- define kernel interfaces
- define task/event schemas
- define provider contract
- define tool contract

### Phase 2

- implement event store and replay
- implement router
- implement policy engine

### Phase 3

- integrate planner, coder, vision providers
- integrate filesystem and git tools
- add rollback and checkpointing

### Phase 4

- add IDE integration
- add UI improvements
- add memory summarization

### Migration rule

- Do not lock the system to one backend.
- Keep model provider swap as a configuration change, not a redesign.

## 14. Open Questions

- What is the minimal task schema that still supports future expansion?
- Which tool set is mandatory for v1?
- How strict should confirmation gates be?
- What should be cached permanently vs derived on demand?
- Which tasks should be asynchronous by default?
- How much autonomy is acceptable before user approval?

## 15. Final Recommendation

Build NexusAOS as a small kernel plus specialist model providers plus an explicit tool layer, all tied together by event sourcing and policy enforcement.

Do not try to make the models do the system design. Do not try to keep all models resident. Do not try to make the UI the architecture. The architecture should be the durable part; model runners and UI should be swappable.

The best near-term outcome is a system that can:

- plan with Gemma
- code with Qwen
- inspect with vision
- recover from failure
- and keep an audit trail for every important action

That is the correct foundation for NexusAOS v2.

## 16. Refinement notes after review

The first-pass design is structurally correct, but the following clarifications make it more implementable and internally consistent:

- **Local-only core vs fallback providers:** keep the kernel local-first and offline-capable. If a remote model is ever allowed later, treat it as an explicit non-core escape hatch behind the same provider interface, not as part of the default design.
- **Task lifecycle:** add a formal task state machine: `received → classified → planned → authorized → executing → blocked/failed/completed → archived`, with `rolled_back` as a compensating terminal state where applicable.
- **Manifest lifecycle:** define `draft → validated → signed → active → superseded → retired`. Manifests must be immutable once active.
- **Replay semantics:** replay should mean control-plane replay from the event log and stored outputs, not exact token regeneration. Model text is archival evidence, not the replay oracle.
- **Heavy model policy:** the 30B coder should be treated as a cold-loaded specialist with hard queueing and refusal when memory pressure would cause swap-thrash.
- **Tool sandboxing:** tools should run as same-machine isolated workers with explicit capability leases, not as in-process plugins.
- **Resource budgets:** codify hard ceilings for RAM, VRAM, context length, queue depth, and disk watermarks so the kernel can refuse work before the system becomes unstable.
- **Worker isolation:** clarify that all workers are local processes on the same host in v2; this is not a networked microservices deployment.

These refinements do not change the architecture; they make the existing direction concrete enough to implement safely.
