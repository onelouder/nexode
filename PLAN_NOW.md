# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 4 — Engine Hardening + Module Decomposition
- **Deadline:** 2026-03-29
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-4-engine-hardening` (to be created)
- **Previous sprint:** Sprint 3 — Observer Loops + Safety (complete, merged to `main` at `9371feb`)

## Tasks

### Part 1: Engine Module Decomposition

- [ ] Read and map `engine.rs` structure (~2700 lines, 74 functions)
- [ ] Create `crates/nexode-daemon/src/engine/` directory module
- [ ] Extract `DaemonConfig` and config structs → `engine/config.rs`
- [ ] Extract `RuntimeState`, `ProjectRuntime`, `SlotRuntime`, `SlotDescriptor`, `MergeDescriptor` → `engine/runtime.rs`
- [ ] Extract command handlers → `engine/commands.rs`
- [ ] Extract slot lifecycle management → `engine/slots.rs`
- [ ] Extract merge queue logic → `engine/merge.rs`
- [ ] Extract event emission helpers → `engine/events.rs`
- [ ] Move integration tests → `engine/tests.rs`
- [ ] Verify all 62 tests pass, cargo fmt/check/clippy clean
- [ ] Commit decomposition separately

### Part 2: Fix I-016 — Task Transition Semantics

- [ ] Add `pre_pause_status: Option<TaskStatus>` to slot runtime
- [ ] Record pre-pause state on Paused transitions
- [ ] Validate resume transitions against pre-pause state
- [ ] Remove `MergeQueue → Paused` from allowed transitions
- [ ] Add unit tests for pre-pause tracking
- [ ] Add integration test for observer pause → resume flow

### Part 3: Fix I-022 — Async Observer Tick

- [ ] Wrap `has_worktree_changes` in `spawn_blocking`
- [ ] Concurrent await for all working slots
- [ ] Verify existing observer tests pass

### Part 4: Fix I-008 — Daemon CLI with clap

- [ ] Add `clap` to `nexode-daemon` dependencies
- [ ] Define `#[derive(Parser)]` struct for daemon CLI
- [ ] Add `--help` and `--version` support
- [ ] Verify daemon starts correctly with existing flags

## Blocked

- None

## Done This Sprint

- (Sprint 4 not yet started)

## Done Previously (Sprint 3)

- Added the observer layer in `crates/nexode-daemon/src/observer.rs` (554 lines)
- Loop detection, uncertainty routing, sandbox enforcement integrated into `engine.rs`
- Extended proto surface with `ObserverAlert`, `ResumeSlot`, `event_sequence`, `AgentStateChanged.slot_id`
- Updated gRPC transport and `nexode-ctl watch` for event-gap detection
- Resolved I-017 and R-005
- Sprint 3 merged to main at `9371feb` (squash merge, 18 files, +1798/-232)

## Next Up

- After Sprint 4: Phase 2 (M3) — TUI + VS Code Extension

## Notes

- Sprint 4 prompt: `.agents/prompts/sprint-4-codex.md`
- Part 1 is a pure refactor — commit separately for clean review
- I-016 fix requires reading `docs/architecture/kanban-state-machine.md` carefully
- I-022 fix follows the existing `spawn_blocking` pattern from merge operations in `git.rs`
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
