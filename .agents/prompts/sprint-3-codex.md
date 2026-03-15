# Codex Sprint 3 Prompt — Observer Loops + Safety

## Task

Execute Sprint 3: Observer Loops + Safety. The daemon can now orchestrate real agents (Claude and Codex verified end-to-end). This sprint adds the safety layer that makes unattended operation viable — detecting stuck agents, enforcing sandbox boundaries, and ensuring event stream reliability.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — open issues (I-016 through I-019, R-008 through R-010)
   - `DECISIONS.md` — all accepted decisions (D-002 through D-010)
   - `ROADMAP.md` — milestone M2b defines this sprint's scope

2. Read these for implementation context:
   - `docs/reviews/sprint-1-review.md` — Sprint 1 findings
   - `docs/reviews/sprint-2-review.md` — Sprint 2 findings
   - `docs/architecture/kanban-state-machine.md` — valid state transitions

3. Read source files you'll work with:
   - `crates/nexode-daemon/src/engine.rs` — engine loop, event dispatch, command handling
   - `crates/nexode-daemon/src/process.rs` — agent process lifecycle, output streaming
   - `crates/nexode-daemon/src/transport.rs` — gRPC transport, event broadcast (R-005 target)
   - `crates/nexode-daemon/src/harness.rs` — harness trait and implementations
   - `crates/nexode-daemon/src/git.rs` — worktree orchestration, merge-and-verify

## Branch

Create and work on: `agent/gpt/sprint-3-observer-safety`

## What to Build

Four deliverables. Complete them in order.

### Part 1: Loop Detection (do first)

**Goal:** Detect when an agent is spinning without making progress and intervene.

**Detection signals:**
1. **Repeated identical tool calls** — if the agent process emits the same output pattern (same file write, same command) 3+ times in a row, it's likely looping.
2. **No git diff progress** — if the agent has been in WORKING state for >N minutes (configurable via `DaemonConfig`) and `git diff` in its worktree shows zero changes, the agent is stuck.
3. **Token budget velocity** — if the agent has consumed >50% of its token budget with zero worktree changes, flag it.

**Implementation:**
- Add a `LoopDetector` struct in a new `crates/nexode-daemon/src/observer.rs` module.
- `LoopDetector` receives `AgentProcessEvent::Output` lines and tracks patterns per-slot.
- The engine loop calls `loop_detector.check(slot_id)` on each tick. If a loop is detected, emit an `ObserverAlert` event and optionally kill the agent (configurable: `on_loop: alert | kill | pause`).
- Add `loop_detection` config to `DaemonConfig`: `enabled: bool`, `max_identical_outputs: u32` (default 3), `stuck_timeout_seconds: u64` (default 300), `budget_velocity_threshold: f64` (default 0.5).

**Tests:**
- Unit test: feed `LoopDetector` a sequence of identical output lines → alert fires.
- Unit test: feed `LoopDetector` varied output → no alert.
- Unit test: configure `on_loop: kill` → verify engine kills the agent process.
- Integration test: mock agent that loops → verify daemon detects and alerts.

### Part 2: Sandbox Enforcement

**Goal:** Ensure agents cannot write outside their assigned worktree.

**Implementation:**
- Before spawning an agent process, resolve the worktree's canonical path.
- Add a `SandboxGuard` to `observer.rs` that monitors `AgentProcessEvent::Output` for file write patterns outside the worktree root.
- After agent completion (before merge), run a `git diff --name-only` check — if any file paths are outside the worktree (symlink escapes, `../` traversal), reject the merge and emit `ObserverAlert`.
- Add `sandbox_enforcement: bool` to `DaemonConfig` (default `true`).

**Tests:**
- Unit test: `SandboxGuard` flags a path outside worktree root.
- Unit test: `SandboxGuard` allows paths inside worktree root.
- Integration test: mock agent that writes `../../../etc/shadow` → verify sandbox blocks merge.

### Part 3: Event Sequence Numbers (R-005)

**Goal:** Fix the broadcast stream drop issue — add sequence numbers so clients can detect missed events.

**Context:** `transport.rs` uses `tokio::sync::broadcast` with `BroadcastStream`. `RecvError::Lagged` events are silently filtered. Under burst conditions (e.g., 10 agents finishing simultaneously), slow clients lose events with no indication. R-005 in ISSUES.md tracks this.

**Implementation:**
- Add a monotonic `event_sequence: u64` field to every `DaemonEvent` proto message (or wrap events in an `Envelope { sequence: u64, event: DaemonEvent }`).
- The engine increments a counter on every event emit.
- Clients track the last-seen sequence number. On reconnect or `Lagged` detection, the client requests a `FullStateSnapshot` to catch up.
- Add a `last_event_sequence` field to `FullStateSnapshot` so clients know where they are.
- Update `nexode-ctl watch` to print a warning if a sequence gap is detected.

**Tests:**
- Unit test: event sequence is monotonically increasing.
- Unit test: `FullStateSnapshot` includes the latest sequence number.
- Integration test: slow consumer detects sequence gap and requests state refresh.

### Part 4: Uncertainty Routing

**Goal:** When an agent signals it's stuck or uncertain, route the situation to the operator.

**Implementation:**
- Add an `UncertaintySignal` variant to `ObserverAlert`: `{ slot_id, agent_id, reason: String }`.
- The `LoopDetector` can emit this when it detects stalling (separate from hard loop detection).
- Parse agent output for uncertainty markers: lines containing `"I'm not sure"`, `"I need clarification"`, `"DECISION:"` comments (as defined in AGENTS.md).
- When an `UncertaintySignal` fires, the engine transitions the slot to `PAUSED` state and emits the alert event for the UI/TUI to display.
- The operator can resume via `nexode-ctl dispatch resume-slot <slot-id>` with an optional instruction.

**Tests:**
- Unit test: agent output containing "DECISION:" triggers uncertainty signal.
- Unit test: uncertainty signal pauses the slot.
- Integration test: mock agent that writes "DECISION: need guidance" → slot transitions to PAUSED.

## Exit Criteria

All five must pass:

1. Loop detection catches repeated identical output patterns and stuck agents (configurable thresholds)
2. Sandbox enforcement prevents file writes outside worktree root
3. Events carry sequence numbers; clients can detect and recover from gaps
4. Uncertainty routing pauses agents that signal they're stuck
5. All existing tests still pass (`cargo test -p nexode-daemon && cargo test -p nexode-ctl`)

## Verification

Before marking complete:
```bash
cargo fmt --all
cargo test -p nexode-daemon
cargo test -p nexode-ctl
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

All existing tests must continue to pass. New tests must be added for each feature.

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- If you need a design decision, add a `// DECISION:` comment and note it in HANDOFF.md for pc review
- The `observer.rs` module is new — you have full design freedom within the trait/struct boundaries described above
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes
