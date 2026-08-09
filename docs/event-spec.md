# SheshaAOS Event Specification

## Overview

Events are the source of truth in SheshaAOS. Every state change, model interaction,
tool call, and policy decision is recorded as an immutable event in the event store.

## Storage Format

Events are stored as JSON Lines (`.jsonl`) — one JSON object per line.

## Event Structure

```json
{
  "id": "01912345-6789-7abc-def0-123456789abc",
  "task_id": "01912345-0000-7abc-def0-123456789000",
  "sequence": 42,
  "kind": "TaskStateChanged",
  "payload": {
    "StateChanged": {
      "from": "Received",
      "to": "Classified"
    }
  },
  "metadata": {
    "source": "kernel",
    "correlation_id": null
  },
  "timestamp": "2026-07-31T12:00:00Z"
}
```

## Event Kinds

| Kind | Description |
|------|-------------|
| TaskCreated | A new task was submitted |
| TaskClassified | Router determined the task type |
| TaskStateChanged | Task transitioned to a new state |
| ModelRequested | Inference was requested from a provider |
| ModelResponded | Provider returned a completion |
| ModelFailed | Provider returned an error |
| ToolRequested | A tool call was initiated |
| ToolCompleted | A tool call succeeded |
| ToolFailed | A tool call failed |
| PolicyChecked | Policy was evaluated for an action |
| PolicyDenied | Policy denied an action |
| ConfirmationRequested | User confirmation was requested |
| ConfirmationGranted | User confirmed an action |
| ConfirmationDenied | User denied an action |
| CheckpointCreated | A checkpoint was saved |
| SnapshotCreated | A projection snapshot was saved |
| SystemStarted | SheshaAOS started |
| SystemShutdown | SheshaAOS shut down |
| Error | An error occurred |

## Guarantees

1. Events are append-only — never modified or deleted
2. Each event has a unique EventId (UUIDv7)
3. Sequence numbers are monotonically increasing
4. Duplicate EventIds are rejected
5. Events are fsynced to disk after each write batch
