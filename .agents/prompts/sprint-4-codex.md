# Codex Sprint 4 Prompt — Engine Hardening + Module Decomposition

## Task

Execute Sprint 4: Engine Hardening + Module Decomposition. Sprints 1-3 delivered the core daemon, real agent integration, and the observer safety layer. The daemon works, but `engine.rs` has grown to ~2700 lines and carries technical debt from three rapid sprints. This sprint stabilizes the codebase before Phase 2 (TUI + VS Code extension) by decomposing the engine, fixing the last medium-severity issue, and closing the highest-value low-severity items.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — open issues (I-016 is your primary target; I-008, I-022 are secondary)
   - `DECISIONS.md` — all accepted decisions (D-001 through D-011)
   - `ROADMAP.md` — milestone M3 defines Phase 2 scope; this sprint clears the path

2. Read these for implementation context:
   - `docs/reviews/sprint-3-review.md` — Sprint 3 findings (I-020 through I-024 are low-priority context)
   - `docs/reviews/sprint-2-review.md` — Sprint 2 findings (I-016 origin)
   - `docs/architecture/kanban-state-machine.md` — valid state transitions (critical for I-016 fix)
   - `docs/architecture/observer-design.md` — observer module design

3. Read source files you'll work with:
   - `crates/nexode-daemon/src/engine.rs` — primary decomposition target (~2700 lines)
   - `crates/nexode-daemon/src/observer.rs` — observer module (I-022 target)
   - `crates/nexode-daemon/src/main.rs` — daemon CLI (I-008 target)
   - `crates/nexode-daemon/src/transport.rs` — gRPC transport
   - `crates/nexode-daemon/src/process.rs` — agent process lifecycle
   - `crates/nexode-ctl/src/main.rs` — CLI client (reference for clap patterns)

## Branch

Create and work on: `agent/gpt/sprint-4-engine-hardening`

## What to Build

Four deliverables. Complete them in order — Part 1 is the largest and most important.

### Part 1: Engine Module Decomposition (do first)

**Goal:** Split `engine.rs` (~2700 lines) into focused modules without changing any behavior.

`engine.rs` currently contains: `DaemonConfig`, `DaemonEngine`, `RuntimeState`, `ProjectRuntime`, `SlotRuntime`, `SlotDescriptor`, `MergeDescriptor`, all command handlers, all event handlers, the tick loop, observer integration, merge queue logic, slot lifecycle management, helper functions, and integration test infrastructure.

**Target structure:**

```
crates/nexode-daemon/src/
├── engine/
│   ├── mod.rs           — DaemonEngine struct, run_daemon* entry points, top-level tick loop
│   ├── config.rs        — DaemonConfig, LoopDetectionConfig, and all config structs
│   ├── runtime.rs       — RuntimeState, ProjectRuntime, SlotRuntime, SlotDescriptor, MergeDescriptor
│   ├── commands.rs      — All OperatorCommand handlers (dispatch_command, handle_*)
│   ├── slots.rs         — Slot lifecycle: start_slot, stop_slot, swap_agent, slot state transitions
│   ├── merge.rs         — Merge queue logic: enqueue_merge, drain_merge_queues, merge_and_verify
│   ├── events.rs        — Event emission helpers: publish_event, observer_payload, format_task_status
│   └── tests.rs         — All integration tests (cfg(test) module)
├── engine.rs            — DELETE (replaced by engine/ directory module)
```

**Rules for decomposition:**
- This is a **pure refactor**. Zero behavior changes. Zero new features. Zero deleted features.
- Every public API that exists today must still exist after decomposition (re-exported from `engine/mod.rs` if needed).
- All 62 tests must pass unchanged after decomposition (58 daemon + 4 ctl).
- Move functions and structs to the module where they logically belong. Use `pub(crate)` for items that need cross-module visibility within the engine but aren't part of the public API.
- The `DaemonEngine` struct in `mod.rs` may hold references or own the sub-module structs — choose the cleanest ownership pattern.
- Helper functions (`resolve_accounting_path`, `default_worktree_root`, `now_ms`, `next_barrier_id`, `is_valid_task_transition`, `format_task_status`, `observer_payload`, `loop_action_to_proto`) should move to whichever module uses them. If shared, put them in `events.rs` or a `helpers.rs`.
- You have design freedom on exact module boundaries. The structure above is a guideline, not a prescription. If a different split makes more sense after reading the code, do that — but explain the reasoning in HANDOFF.md.
- Do NOT change `lib.rs` exports in a way that breaks `nexode-ctl` or any other crate's imports.

**Verification after Part 1:**
```bash
cargo fmt --all
cargo test -p nexode-daemon
cargo test -p nexode-ctl
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

Commit Part 1 separately before starting Part 2.

### Part 2: Fix I-016 — Task Transition Semantics

**Goal:** Align `is_valid_task_transition` with `docs/architecture/kanban-state-machine.md`.

**Background (from I-016):** Two divergences exist:
1. `MergeQueue → Paused` is allowed in code but not in the spec's exhaustive transition table.
2. `Paused → Working` and `Paused → MergeQueue` are allowed unconditionally, but the spec requires tracking the pre-pause state ("if was WORKING" / "if was queued"). No pre-pause state is stored.

**Implementation:**
1. Add a `pre_pause_status: Option<TaskStatus>` field to `SlotRuntime` (or wherever slot state lives after Part 1 decomposition).
2. When a slot transitions to `Paused`, record its current status in `pre_pause_status`.
3. When resuming from `Paused`:
   - If `pre_pause_status == Some(Working)` → allow `Paused → Working`
   - If `pre_pause_status == Some(MergeQueue)` → allow `Paused → MergeQueue`
   - Otherwise → reject the transition
4. Remove `MergeQueue → Paused` from the allowed transitions (it's not in the spec).
5. Update `is_valid_task_transition` to accept the `pre_pause_status` context (it currently takes only `current` and `target` — it may need a third parameter, or the validation logic may move into a method on the runtime struct that has access to slot state).

**Edge case:** The observer's `LoopAction::Pause` intervention currently pauses WORKING slots. Verify that this still works correctly with the pre-pause tracking. The observer should only pause slots that are in WORKING state, which is already the case.

**Tests:**
- Unit test: Pause from Working → resume to Working succeeds
- Unit test: Pause from MergeQueue → resume to MergeQueue succeeds
- Unit test: Pause from Working → resume to MergeQueue fails (wrong pre-pause state)
- Unit test: MergeQueue → Paused direct transition is rejected
- Integration test: Observer pauses a working slot → operator resumes → slot returns to Working

### Part 3: Fix I-022 — Async Observer Tick

**Goal:** Move blocking `git status` calls in the observer tick to `spawn_blocking`.

**Background (from I-022):** `run_observer_tick` calls `orchestrator.has_worktree_changes(&worktree_path)` synchronously for every working slot. This runs `git status --porcelain` via `std::process::Command`, blocking the tokio runtime. The existing codebase already uses `spawn_blocking` for merge operations in `git.rs` — apply the same pattern here.

**Implementation:**
1. Wrap the `has_worktree_changes` call in `tokio::task::spawn_blocking()`.
2. Collect the futures for all working slots and await them concurrently (e.g., `futures::future::join_all` or similar).
3. Feed the results back into the observer's `observe_status` calls.

**Tests:**
- Existing observer/engine tests must still pass (the behavior is unchanged, only the execution model changes).
- If practical, add a test that verifies the observer tick doesn't block the engine's event processing (e.g., fire an observer tick while a command is pending and verify the command completes promptly). This may be difficult to test deterministically — use your judgment on whether it's worth the complexity.

### Part 4: Fix I-008 — Daemon CLI with clap

**Goal:** Replace manual `std::env::args()` parsing in the daemon's `main.rs` with `clap` derive macros, matching the pattern already used in `nexode-ctl`.

**Implementation:**
1. Add `clap` to `nexode-daemon`'s `Cargo.toml` dependencies (it's already in the workspace via `nexode-ctl`).
2. Define a `#[derive(Parser)]` struct for the daemon's CLI arguments.
3. Current supported flags (from reading `main.rs`):
   - `--session <path>` — path to session.yaml
   - `--port <number>` — gRPC listen port
   - Any other flags currently parsed manually
4. Add `--help` support (free with clap).
5. Add `--version` support using `#[command(version)]`.

**Tests:**
- The daemon should still start correctly with existing flags.
- `--help` and `--version` should produce correct output.

## Exit Criteria

All five must pass:

1. `engine.rs` is decomposed into an `engine/` module directory with ≥4 sub-modules, no file exceeds 800 lines, and all 62 tests pass unchanged
2. `is_valid_task_transition` tracks pre-pause state and rejects invalid resume transitions (I-016 resolved)
3. Observer tick runs `git status` via `spawn_blocking`, not synchronously on the async runtime (I-022 resolved)
4. Daemon binary uses `clap` for argument parsing with `--help` and `--version` (I-008 resolved)
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

All existing tests must continue to pass. New tests must be added for Part 2 (I-016) at minimum.

## Rules

- Commit messages: `[gpt] type: description`
- Commit Part 1 (decomposition) separately before Parts 2-4 so the refactor can be reviewed independently
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- If you need a design decision, add a `// DECISION:` comment and note it in HANDOFF.md for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes
- After decomposition (Part 1), verify that `cargo doc --workspace` still generates clean documentation
