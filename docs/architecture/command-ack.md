# Command Acknowledgment Architecture

> **Status:** PROPOSED
> **Date:** 2026-03-15
> **Author:** pc
> **Addresses:** R-007 (CommandResponse is fire-and-forget)
> **Spec refs:** `sec-03-04-events-commands`, `sec-05-universal-command-chat`

---

## 1. Problem Statement

The current `DispatchCommand` gRPC endpoint always returns `CommandResponse { success: true }` as long as the channel to the engine is open. It does not wait for the engine to process the command, validate it, or report the result. This means:

- A `PauseAgent` sent to a nonexistent slot returns `success: true`
- A `ResumeAgent` sent to a slot that isn't paused returns `success: true`
- A `MoveTask` that fails due to an invalid state transition returns `success: true`
- The client has no way to distinguish "command accepted" from "command processed successfully"

This was acceptable in Phase 0 where the only client was `nexode-ctl` printing status. It becomes a correctness issue when:
- The TUI/extension needs to update UI state based on command results
- The demo script needs to verify merge success
- Any automated workflow depends on knowing whether a command actually executed

## 2. Design

### 2.1 Request/Response Pattern

Replace the unidirectional channel with a oneshot-based request/response:

```
┌──────────┐        ┌───────────────┐        ┌────────────┐
│ gRPC     │  (cmd, │  Transport    │  (cmd,  │   Engine   │
│ Client   │──tx)──▶│  Layer        │──tx)───▶│   Loop     │
│          │        │               │         │            │
│          │◀──rx───│  await rx     │◀──send──│  result    │
│          │        │  (5s timeout) │         │            │
└──────────┘        └───────────────┘        └────────────┘
```

**Transport layer:**
1. `dispatch_command` receives an `OperatorCommand` from the client.
2. Creates a `tokio::sync::oneshot::channel()` for the response.
3. Sends `(OperatorCommand, oneshot::Sender<CommandResponse>)` into the engine's command channel.
4. Awaits the `oneshot::Receiver` with a 5-second timeout.
5. Returns the `CommandResponse` to the client, or a timeout error if the engine didn't respond.

**Engine loop:**
1. Receives `(OperatorCommand, oneshot::Sender<CommandResponse>)` from the channel.
2. Validates the command (slot exists, transition is valid).
3. Executes the command.
4. Sends `CommandResponse` through the oneshot sender with the actual result.

### 2.2 Proto Schema Changes

```protobuf
// Existing — no changes
message OperatorCommand {
  string command_id = 1;
  oneof action { ... }
}

// Updated
message CommandResponse {
  bool success = 1;
  string error_message = 2;
  string command_id = 3;        // NEW: Echo back the command_id
  CommandOutcome outcome = 4;   // NEW: What happened
}

// NEW
enum CommandOutcome {
  COMMAND_OUTCOME_UNSPECIFIED = 0;
  COMMAND_OUTCOME_EXECUTED = 1;
  COMMAND_OUTCOME_REJECTED = 2;
  COMMAND_OUTCOME_SLOT_NOT_FOUND = 3;
  COMMAND_OUTCOME_INVALID_TRANSITION = 4;
}
```

### 2.3 Outcome Semantics

| Outcome | `success` | When |
|---|---|---|
| `EXECUTED` | `true` | Command was processed and the requested action was performed |
| `REJECTED` | `false` | Command was syntactically valid but could not be executed (e.g., budget exceeded) |
| `SLOT_NOT_FOUND` | `false` | The `slot_id` or `agent_id` in the command doesn't match any active slot |
| `INVALID_TRANSITION` | `false` | The requested state transition is not valid from the slot's current state |
| `UNSPECIFIED` | `false` | Timeout or internal error |

### 2.4 Command Validation Rules

| Command | Requires | Valid States | Error Outcome |
|---|---|---|---|
| `PauseAgent` | Valid `agent_id` in an active slot | `WORKING` | `INVALID_TRANSITION` if not WORKING |
| `ResumeAgent` | Valid `agent_id` in a paused slot | `PAUSED` | `INVALID_TRANSITION` if not PAUSED |
| `MoveTask` | Valid `slot_id` | Per kanban state machine (D-009) | `INVALID_TRANSITION` |
| `ChatDispatch` | None (broadcast) | Any | Always `EXECUTED` (fire-and-forget to orchestrator) |
| `SlotDispatch` | Valid `slot_id` | Any | `SLOT_NOT_FOUND` |

### 2.5 Timeout Handling

The transport layer enforces a 5-second timeout on the oneshot response. If the engine loop is blocked or slow:

```rust
match tokio::time::timeout(Duration::from_secs(5), response_rx).await {
    Ok(Ok(resp)) => Ok(Response::new(resp)),
    Ok(Err(_)) => Ok(Response::new(CommandResponse {
        success: false,
        error_message: "Engine dropped command response channel".into(),
        command_id: command.command_id.clone(),
        outcome: CommandOutcome::Unspecified.into(),
    })),
    Err(_) => Ok(Response::new(CommandResponse {
        success: false,
        error_message: "Engine did not respond within 5 seconds".into(),
        command_id: command.command_id.clone(),
        outcome: CommandOutcome::Unspecified.into(),
    })),
}
```

### 2.6 Backward Compatibility

- The `command_id` and `outcome` fields are new proto fields with default values. Older clients that don't set `command_id` will receive an empty string echo. Older clients that don't read `outcome` will still see `success: bool` with correct semantics.
- The channel type change from `mpsc::UnboundedSender<OperatorCommand>` to `mpsc::UnboundedSender<(OperatorCommand, oneshot::Sender<CommandResponse>)>` is internal — the gRPC API surface doesn't change.

## 3. What This Does NOT Address

- **Event sequence numbers (R-005):** The event broadcast stream still drops lagged events. That's Sprint 3 scope (observer loops).
- **Long-running command tracking:** Commands like `MoveTask` may trigger a merge that takes seconds. This design acknowledges the command and starts execution but does NOT wait for merge completion. Merge results are reported via the event stream (`MergeCompleted` event). If we need command-level completion tracking in the future, we'd add a `CommandCompleted` event type.
- **Command queuing / ordering guarantees:** Commands are processed in FIFO order from the mpsc channel. No priority queue or reordering.

## 4. Test Plan

1. **Happy path:** Send `PauseAgent` for a running slot → `EXECUTED`, slot transitions to PAUSED.
2. **Slot not found:** Send `PauseAgent` for `"nonexistent-slot"` → `SLOT_NOT_FOUND`.
3. **Invalid transition:** Send `ResumeAgent` for a slot in WORKING state → `INVALID_TRANSITION`.
4. **Command ID echo:** Send command with `command_id: "cmd-42"` → response has `command_id: "cmd-42"`.
5. **Timeout:** Drop the engine's command receiver, send command → response with `UNSPECIFIED` and timeout error.
6. **MoveTask lifecycle:** Send `MoveTask(REVIEW → MERGE_QUEUE)` → `EXECUTED`. Send `MoveTask(WORKING → DONE)` → `INVALID_TRANSITION`.
