---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-15T16:42:00-07:00
task: "Sprint 4 — Engine Hardening + Module Decomposition"
branch: "main"
next: gpt
---

# Handoff: Sprint 4 Ready for Codex

## What Just Happened

Sprint 3 (Observer Loops + Safety) was reviewed, rebased onto latest `origin/main`, and merged via squash merge. Merge commit: `9371feb`.

Post-merge state:
- 62 tests passing (58 daemon + 4 ctl)
- `cargo fmt`, `cargo check`, `cargo clippy` all clean
- All 5 Sprint 3 exit criteria met
- Issues I-020 through I-024 added to ISSUES.md (all Low, non-blocking)
- I-017 and R-005 resolved

## Sprint 4 Scope

Sprint 4 is a hardening sprint before Phase 2 (TUI + VS Code extension). Four deliverables:

1. **Engine module decomposition** — split `engine.rs` (~2700 lines) into an `engine/` directory with focused sub-modules (config, runtime, commands, slots, merge, events, tests). Pure refactor, zero behavior changes.

2. **Fix I-016** — `is_valid_task_transition` diverges from the Kanban state machine spec. Add `pre_pause_status` tracking so resume transitions are validated against the slot's pre-pause state.

3. **Fix I-022** — Observer tick runs blocking `git status` synchronously in the async engine loop. Wrap in `spawn_blocking` following the existing pattern from merge operations.

4. **Fix I-008** — Daemon `main.rs` uses manual arg parsing. Replace with `clap` derive macros (matching `nexode-ctl` patterns). Adds `--help` and `--version`.

## Sprint 4 Prompt

`.agents/prompts/sprint-4-codex.md`

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `ISSUES.md` — focus on I-016, I-022, I-008
- `DECISIONS.md`
- `docs/architecture/kanban-state-machine.md` — needed for I-016 fix
- `.agents/prompts/sprint-4-codex.md` — full sprint instructions

## Context for Codex

### engine.rs Structure

The file contains everything: `DaemonConfig`, `DaemonEngine`, `RuntimeState`, `ProjectRuntime`, `SlotRuntime`, `SlotDescriptor`, `MergeDescriptor`, all command handlers, event handlers, tick loop, observer integration, merge queue, slot lifecycle, helpers, and integration tests. See the sprint prompt for a suggested module decomposition.

### I-016 Details

Two divergences from `docs/architecture/kanban-state-machine.md`:
1. `MergeQueue → Paused` is allowed in code but not in the spec
2. `Paused → Working` and `Paused → MergeQueue` are allowed unconditionally, but the spec requires knowing the pre-pause state

The fix needs a `pre_pause_status` field on the slot runtime. The observer's `LoopAction::Pause` only pauses WORKING slots, so it should work correctly with pre-pause tracking.

### I-022 Details

`run_observer_tick` in `engine.rs` calls `has_worktree_changes()` (which shells out to `git status --porcelain`) synchronously for every working slot per tick. Apply the `spawn_blocking` pattern already used in merge operations.

### Commit Strategy

Commit Part 1 (decomposition) separately before Parts 2-4. This lets the pure refactor be reviewed independently from behavioral changes.
