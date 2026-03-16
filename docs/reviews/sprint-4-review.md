# Sprint 4 Code Review: Engine Hardening + Module Decomposition

**Branch:** `agent/gpt/sprint-4-engine-hardening`
**Reviewer:** pc (Claude)
**Date:** 2026-03-16
**Commits reviewed:**
- `3cd2355` — pure refactor: decompose daemon engine module
- `eb47ad2` — feat: harden engine transitions and daemon CLI

---

## Summary

Sprint 4 delivers four discrete work items: (1) decomposing the monolithic `engine.rs` (~2700 lines) into a focused `engine/` module directory, (2) hardening the Kanban task-transition state machine with context-aware pause/resume guards (I-016), (3) moving the observer tick's blocking `git-status` calls off the async runtime using `JoinSet::spawn_blocking` (I-022), and (4) replacing manual `std::env::args` parsing with `clap` derive macros (I-008).

The decomposition is a clean, content-preserving refactor with no behavioral changes. The transition hardening correctly implements `pre_pause_status` tracking so that `Paused -> Working` and `Paused -> MergeQueue` are only valid when the pre-pause state matches. The observer tick now runs git-status checks concurrently on the blocking thread pool. The CLI migration is straightforward and well-tested.

**Verdict: APPROVE** — all exit criteria are met, all tests pass, and no regressions were introduced. Two low-severity findings are noted for follow-up.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| `engine/` directory with sub-modules, no file > 800 LOC | PASS | 8 sub-modules; max file is `tests.rs` at 665 lines |
| `is_valid_task_transition` context-aware with `pre_pause_status` | PASS | Three-argument signature at `commands.rs:256`; 4 unit tests |
| `run_observer_tick` uses `spawn_blocking` for git-status | PASS | `JoinSet::spawn_blocking` at `mod.rs:367-377` |
| `main.rs` uses `clap` derive macros | PASS | `#[derive(Parser)]` with 3 CLI tests |
| `cargo test -p nexode-daemon` passes (63 lib + 3 bin) | PASS | 63 lib tests + 3 bin tests, 0 failures |
| `cargo test -p nexode-ctl` passes (4 tests) | PASS | 4 tests, 0 failures |
| `cargo fmt --all -- --check` clean | PASS | No formatting issues |
| `cargo check --workspace` clean | PASS | No errors |
| `cargo clippy --workspace -- -D warnings` clean | PASS | No warnings |
| First commit is a pure refactor (no behavioral changes) | PASS | Diff: 2811 insertions / 2722 deletions across 12 files; content-preserving split |
| `serial_test` applied only to server-backed tests | PASS | 4 tests with `#[serial_test::serial]` — all use `TcpListener::bind` + `run_daemon_with_listener` |
| WAL non-persistence of `pre_pause_status` documented | PASS | DECISION comment at `runtime.rs:273`; field absent from `to_persisted()` |

---

## Findings

### F-01 [Low] `Review -> Paused` creates un-resumable state via `ResumeAgent`/`ResumeSlot`

**Location:** `commands.rs:235-241`, `commands.rs:267`

The transition table allows `Review -> Paused` (line 267: `(Review, MergeQueue | Working | Paused)`), and `set_task_status` correctly records `pre_pause_status = Some(Review)` when this happens. However, `resume_target()` (lines 235-241) only handles `Some(Working)` and `Some(MergeQueue)`, returning `None` for any other pre-pause state:

```rust
fn resume_target(&self, slot_id: &str) -> Option<TaskStatus> {
    match self.pre_pause_status(slot_id) {
        Some(TaskStatus::Working) => Some(TaskStatus::Working),
        Some(TaskStatus::MergeQueue) => Some(TaskStatus::MergeQueue),
        _ => None,
    }
}
```

This means a slot paused from `Review` cannot be resumed via `ResumeAgent` or `ResumeSlot` commands — both return `InvalidTransition`. The operator must use `MoveTask` to move it back to `Review` or `Working` instead.

This is technically correct per the authoritative Kanban spec (`docs/architecture/kanban-state-machine.md`), which only defines `Paused -> Working` and `Paused -> MergeQueue` as valid resume paths. However, the UX asymmetry (can pause from Review, but must use a different command to un-pause) could surprise operators.

**Recommendation:** Either add `Some(TaskStatus::Review) => Some(TaskStatus::Review)` to `resume_target()`, or document this as intentional behavior. Low priority — MoveTask provides a workaround.

### F-02 [Info] `LoopAction::Kill` pauses rather than archives

**Location:** `slots.rs:444-448`

When `handle_observer_finding` processes a `LoopAction::Kill` finding, it calls `kill_slot(&finding.slot_id, TaskStatus::Paused)` — the slot ends up `Paused`, not `Archived`. This is actually the safer choice (operator can inspect and resume), but the naming is potentially misleading since "Kill" semantics usually imply termination. The `kill_slot` method does shut down the supervisor, so the process is actually killed — only the task state remains recoverable.

**Recommendation:** No code change needed. Consider a doc comment on the `Kill` variant clarifying that "kill" means "terminate process and pause slot" rather than "terminate and archive."

---

## Positive Notes

1. **Clean two-commit split.** The first commit is a pure structural refactor with zero behavioral changes. This makes both commits independently reviewable and bisect-safe. The diff stats confirm: commit 1 is -2722/+2811 (structural move), commit 2 is -134/+505 (feature work).

2. **Module boundaries are well-chosen.** Each sub-module has a clear responsibility:
   - `config.rs` (40 lines): daemon configuration and defaults
   - `runtime.rs` (438 lines): state structs, accessor methods, persistence
   - `commands.rs` (315 lines): operator command dispatch, transition validation
   - `slots.rs` (515 lines): slot lifecycle, process event handling, telemetry
   - `merge.rs` (179 lines): merge queue, merge execution, checkpointing
   - `events.rs` (171 lines): event publishing, WAL helpers, utilities
   - `mod.rs` (405 lines): engine struct, bootstrap, run loop, observer tick
   - `test_support.rs` (242 lines): test fixtures and helpers
   - `tests.rs` (665 lines): integration tests

   No file exceeds the 800-line cap. Total: ~2970 lines across 9 files (compared to the original ~2706 monolith plus test additions).

3. **Pre-pause tracking is correctly implemented.** The `set_task_status` method (slots.rs:485-489) handles all edge cases:
   - First pause: records previous status
   - Re-pause while already paused: preserves existing pre-pause status
   - Non-pause transition: clears pre-pause status

   The `is_valid_task_transition` function uses a clean three-argument pattern match that is easy to reason about.

4. **New test covers the full pause/resume cycle.** `observer_pause_can_resume_back_to_working` (tests.rs:498-557) exercises: start -> loop detection -> pause -> resume -> successful completion to review. This directly validates I-016.

5. **`JoinSet::spawn_blocking` for concurrent observer checks.** The observer tick (mod.rs:367-397) collects all working slots, spawns blocking git-status checks concurrently on the Tokio blocking thread pool, then processes results. This is the correct pattern — it avoids both blocking the async runtime and serializing independent checks.

6. **`serial_test` scoping is correct.** Only the 4 tests that bind TCP listeners and run the full gRPC server are marked `#[serial_test::serial]`. The 6 engine-only tests (which use `DaemonFixture::engine()` without TCP) run in parallel. This maximizes test throughput without port conflicts.

7. **CLI migration is minimal and well-tested.** The `clap` integration (main.rs:8-25) preserves all existing flags, adds `--help`/`--version` for free, and handles the positional/flag conflict for `SESSION` correctly with `conflicts_with`. Three tests cover flag parsing, port-only mode, and help/version output.

8. **WAL non-persistence decision is justified.** The DECISION comment at `runtime.rs:273` explains that `pre_pause_status` is intentionally runtime-only due to bincode backward-safety concerns. Since the WAL checkpoint format uses `bincode` serialization, adding new fields would break existing WAL files. The cost is minor: a daemon crash while a slot is paused loses the pre-pause state, making `ResumeAgent`/`ResumeSlot` unavailable (operator must use `MoveTask`). This is an acceptable trade-off for Sprint 4.

---

## Issues

### Resolved

| Issue | Resolution |
|---|---|
| I-016: `is_valid_task_transition` diverges from Kanban spec | Fixed: transition function is now context-aware with `pre_pause_status` tracking. `MergeQueue -> Paused` removed. `Paused -> Working`/`MergeQueue` gated on pre-pause state. |
| I-022: `run_observer_tick` runs blocking git-status in async context | Fixed: uses `JoinSet::spawn_blocking` for concurrent blocking checks |
| I-008: daemon uses manual arg parsing | Fixed: replaced with `clap` derive macros |

### Open

| Issue | Notes |
|---|---|
| F-01: `Review -> Paused` un-resumable via `ResumeAgent`/`ResumeSlot` | Low priority. Matches spec but creates UX asymmetry. `MoveTask` is a workaround. |

### New

*None.*

---

## Verification Suite

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (no output) |
| `cargo test -p nexode-daemon` | PASS — 63 lib tests + 3 bin tests, 0 failures |
| `cargo test -p nexode-ctl` | PASS — 4 tests, 0 failures |
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS (no warnings) |

---

## Verdict

**APPROVE.** All four deliverables are complete and correct. The decomposition is a clean structural split, the transition hardening implements the spec faithfully with proper test coverage, the observer tick no longer blocks the async runtime, and the CLI migration is minimal and well-tested. Two low-severity findings are noted for future consideration but do not block this sprint.
