---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-15T22:30:00-07:00
task: "Sprint 6 — Integration Polish"
branch: "main"
next: gpt
---

# Handoff: Sprint 6 Ready for Codex

## What Just Happened

Sprint 5 (TUI Dashboard) was reviewed and merged to `main` at `4e5f6cf`. All exit criteria met, 18 TUI tests pass, 66 daemon + 4 ctl tests pass, no regressions. Status colors fixed post-review to align with kanban spec (D-009). Three new issues filed: I-026 (resolved pre-merge), I-027, I-028.

Sprint 5 delivered:
- New `nexode-tui` crate with `ratatui` + `crossterm`
- Three-panel dashboard: project tree, slot detail, event log
- Live gRPC streaming with event gap recovery
- Interactive controls: navigate, pause/resume/kill, command mode
- Terminal cleanup on exit/signal/panic
- 18 unit tests

Sprint 5 review: `docs/reviews/sprint-5-review.md`

## Sprint 6 Scope

Sprint 6 is a **polish and integration sprint** before the VS Code extension. It addresses accumulated low-severity issues, improves the TUI's event handling, and adds a cross-crate integration test that exercises the full daemon→TUI pipeline.

### Part 1: TUI Fixes (I-027, I-028)

1. **I-027 — Event gap recovery should not drop the triggering event.** After fetching a snapshot on gap detection, also apply the triggering event if its sequence is beyond the snapshot's `last_event_sequence`.
2. **I-028 — Compute local timezone offset at startup.** Capture `UtcOffset::current_local_offset()` in `main()` before the tokio runtime spawns threads, pass it through `AppState`, use it in `format_timestamp`. Label timestamps as UTC if offset detection fails.

### Part 2: Daemon Fixes (I-025, I-007)

3. **I-025 — Add `Review` to `resume_target()`.** In `engine/commands.rs`, add `Some(TaskStatus::Review) => Some(TaskStatus::Review)` so slots paused from Review can be resumed without `MoveTask`. Add a test.
4. **I-007 — Immediate merge queue drain on enqueue.** After `enqueue_merge()` in `engine/slots.rs`, call `drain_merge_queues()` immediately instead of waiting for the next tick. This reduces merge latency from up to 2s to near-zero.

### Part 3: Integration Test

5. **Cross-crate integration test.** Create `tests/integration/` (workspace-level) or add to `nexode-tui/tests/`:
   - Start daemon in-process with mock config
   - Connect TUI's `AppState` (no terminal rendering) via gRPC
   - Dispatch commands, verify state updates flow through
   - Verify event gap recovery works end-to-end
   - This is the first test that proves daemon→TUI works together

### Part 4: Cleanup

6. **Add `--version` to TUI CLI** (F-07 from Sprint 5 review)
7. **Update `I-014`** — fix agent-harness architecture doc CLI flags
8. **Add `--version` to daemon CLI** if missing

## Sprint 6 Prompt

`.agents/prompts/sprint-6-codex.md`

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `.agents/prompts/sprint-6-codex.md` — full sprint instructions
- `ISSUES.md` — focus on I-007, I-025, I-027, I-028
- `docs/reviews/sprint-5-review.md` — context for TUI fixes

## Context for Codex

### TUI Source

The TUI crate at `crates/nexode-tui/` has 5 source files:
- `main.rs` — gRPC bootstrap, event loop, command dispatch, terminal cleanup
- `state.rs` — `AppState`, `apply_event`, `apply_snapshot`
- `events.rs` — event formatting with `format_timestamp`
- `input.rs` — key bindings, command parsing
- `ui.rs` — dashboard rendering

### Daemon Engine

The engine is decomposed into `crates/nexode-daemon/src/engine/`:
- `commands.rs` — command dispatch, `resume_target()`, `is_valid_task_transition()`
- `slots.rs` — slot lifecycle, `enqueue_merge()`
- `merge.rs` — merge execution
- `mod.rs` — engine loop, `drain_merge_queues()`

### Test Baseline

- Daemon: 63 lib + 3 bin = 66 tests
- Ctl: 4 tests
- TUI: 18 tests
- Total: 88 tests
