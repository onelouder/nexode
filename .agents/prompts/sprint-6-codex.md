# Codex Sprint 6 Prompt — Integration Polish

## Task

Execute Sprint 6: Integration Polish. This is a focused cleanup sprint that fixes accumulated low-severity issues across the TUI and daemon, then adds the first cross-crate integration test proving the daemon→TUI pipeline works end-to-end.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — focus on I-007, I-025, I-027, I-028
   - `docs/reviews/sprint-5-review.md` — context for TUI findings

2. Read these for implementation context:
   - `crates/nexode-tui/src/main.rs` — gap recovery code (line ~322)
   - `crates/nexode-tui/src/events.rs` — timestamp formatting (line ~112)
   - `crates/nexode-daemon/src/engine/commands.rs` — `resume_target()` (line ~235)
   - `crates/nexode-daemon/src/engine/slots.rs` — `enqueue_merge()`
   - `crates/nexode-daemon/src/engine/mod.rs` — `drain_merge_queues()`

## Branch

Create and work on: `agent/gpt/sprint-6-integration-polish`

## What to Build

### Part 1: TUI Fixes

#### 1a. Fix I-027 — Event gap recovery should apply the triggering event

**Location:** `crates/nexode-tui/src/main.rs`, `run_grpc_receiver` function

Currently, when a sequence gap is detected, the code fetches a snapshot and `continue`s — dropping the event that triggered the gap. Fix:

```rust
// After sending the snapshot:
if event.event_sequence > snapshot.last_event_sequence {
    // The triggering event is newer than the snapshot — apply it too
    last_sequence = event.event_sequence;
    if tx.send(GrpcMessage::Event(event)).await.is_err() {
        break;
    }
} else {
    // Snapshot already covers this event
    last_sequence = snapshot.last_event_sequence;
}
continue;
```

Add a test in `state.rs` or a new test file that verifies this logic (you can test the state application side — apply a snapshot, then apply an event with sequence > snapshot).

#### 1b. Fix I-028 — Compute timezone offset at startup

**Location:** `crates/nexode-tui/src/events.rs` and `crates/nexode-tui/src/main.rs`

The `time` crate's `current_local_offset()` fails under multi-threaded tokio. Fix:

1. In `main()`, **before** `#[tokio::main]` spawns the runtime, compute the offset:
   ```rust
   fn main() {
       let local_offset = time::UtcOffset::current_local_offset()
           .unwrap_or(time::UtcOffset::UTC);
       tokio_main(local_offset);
   }

   #[tokio::main]
   async fn tokio_main(local_offset: time::UtcOffset) {
       // ... existing code, pass local_offset to AppState or events module
   }
   ```
   Note: This requires restructuring `main` so the offset is captured before `#[tokio::main]` creates threads. The `#[tokio::main]` macro expands to create a runtime — you need a plain `fn main()` that calls into the async entry point.

2. Add `local_offset: UtcOffset` to `AppState` (or pass it through to the event formatting functions).

3. Update `format_timestamp` in `events.rs` to accept the offset as a parameter instead of calling `current_local_offset()` each time.

4. If the offset is UTC, consider adding a `(UTC)` label to the event log header or timestamps so operators know they're seeing UTC.

### Part 2: Daemon Fixes

#### 2a. Fix I-025 — Add Review to `resume_target()`

**Location:** `crates/nexode-daemon/src/engine/commands.rs`, `resume_target()` function

Add `Some(TaskStatus::Review) => Some(TaskStatus::Review)` to the match:

```rust
fn resume_target(&self, slot_id: &str) -> Option<TaskStatus> {
    match self.pre_pause_status(slot_id) {
        Some(TaskStatus::Working) => Some(TaskStatus::Working),
        Some(TaskStatus::MergeQueue) => Some(TaskStatus::MergeQueue),
        Some(TaskStatus::Review) => Some(TaskStatus::Review),
        _ => None,
    }
}
```

Add a test that verifies:
1. Slot in Review → pause → resume → back in Review
2. This should mirror the existing `observer_pause_can_resume_back_to_working` test pattern

#### 2b. Fix I-007 — Immediate merge queue drain

**Location:** `crates/nexode-daemon/src/engine/slots.rs` and `crates/nexode-daemon/src/engine/mod.rs`

Currently, `drain_merge_queues()` only runs on the tick interval (~2s). When a task is enqueued for merge, there's an unnecessary delay.

Option A (preferred): After `enqueue_merge()` is called in the slot lifecycle (when a task transitions to `MergeQueue`), immediately trigger `drain_merge_queues()` in the same tick. This can be done by having `enqueue_merge()` set a flag (e.g., `self.merge_pending = true`) that the engine loop checks before the next sleep.

Option B: Call `drain_merge_queues()` directly after each `enqueue_merge()` call site. This is simpler but may need careful ordering.

Either approach is acceptable. The key invariant: a test that moves a task to `MergeQueue` should see the merge happen without waiting for a tick interval.

Add or update a test that verifies merge starts immediately (or within 1 tick) after enqueue.

### Part 3: Integration Test

**Goal:** Prove the daemon→TUI pipeline works end-to-end.

Create a test (in `crates/nexode-tui/tests/` or a workspace-level `tests/` directory) that:

1. Starts a daemon in-process using the existing `DaemonFixture` test helper pattern (see `crates/nexode-daemon/src/engine/test_support.rs`)
2. The daemon binds to a random port (use `TcpListener::bind("[::1]:0")`)
3. Connects a TUI `AppState` (no terminal rendering needed) via gRPC:
   - Calls `GetFullState` to get initial snapshot
   - Calls `apply_snapshot()` on `AppState`
4. Subscribes to `SubscribeEvents` stream
5. Dispatches a command (e.g., `PauseAgent` on a running slot)
6. Verifies that:
   - The command response comes back with `Executed`
   - An event arrives on the subscription stream
   - `apply_event()` updates the `AppState` correctly
   - The slot's status in `AppState` matches what was commanded

This test exercises: gRPC transport, state serialization/deserialization, event streaming, command dispatch, and the TUI's state application logic — all in one integration.

**Dependencies:** The test will need both `nexode-daemon` and `nexode-tui` as dev-dependencies. If creating a workspace-level test, add `[dev-dependencies]` in the workspace or create a new test crate.

**Note:** If wiring up the full daemon fixture is too complex for this sprint, a simpler alternative is acceptable: test that `AppState::apply_snapshot()` correctly handles a `FullStateSnapshot` built from real proto types, and that `apply_event()` for each event variant produces the expected state changes. This is less end-to-end but still validates the TUI↔proto contract.

### Part 4: Cleanup

1. **Add `--version` to TUI CLI:** Add `version` to the `#[command(...)]` attribute on the `Cli` struct in `crates/nexode-tui/src/main.rs`. One-line change.

2. **Fix I-014:** Update `docs/architecture/agent-harness.md` to use the correct CLI flags:
   - Codex: `codex exec --full-auto --json` (not `codex --approval-mode full-auto`)
   - Claude: `claude --verbose --output-format stream-json`
   - Check against actual harness code in `crates/nexode-daemon/src/harness.rs`

3. **Add `--version` to daemon CLI** if not already present (check `crates/nexode-daemon/src/main.rs`).

## Exit Criteria

All must pass:

1. I-027 fixed: event gap recovery applies the triggering event when its sequence exceeds the snapshot
2. I-028 fixed: timezone offset computed at startup, timestamps use local time (or labeled UTC)
3. I-025 fixed: `resume_target()` returns `Some(Review)` for slots paused from Review, with test
4. I-007 fixed: merge queue drains immediately on enqueue, verified by test
5. Integration test passes: at least one test proves TUI state updates from daemon events
6. CLI cleanup: `--version` works on both `nexode-tui` and `nexode-daemon`
7. No regressions: all existing tests pass

## Verification

Before marking complete:
```bash
cargo fmt --all
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build -p nexode-tui
cargo build -p nexode-daemon
cargo run -p nexode-tui -- --version
cargo run -p nexode-daemon -- --version
```

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`
- Proto modifications allowed ONLY if needed for integration test fixtures, and must be backward-compatible
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes
- Mark resolved issues in `ISSUES.md`
