# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 4 — Engine Hardening + Module Decomposition
- **Deadline:** 2026-03-29
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-4-engine-hardening`
- **Previous sprint:** Sprint 3 — Observer Loops + Safety (complete, merged to `main` at `9371feb`)

## Tasks

### Part 1: Engine Module Decomposition

- [x] Read and map `engine.rs` structure (~2700 lines, 74 functions)
- [x] Create `crates/nexode-daemon/src/engine/` directory module
- [x] Extract `DaemonConfig` and config structs → `engine/config.rs`
- [x] Extract `RuntimeState`, `ProjectRuntime`, `SlotRuntime`, `SlotDescriptor`, `MergeDescriptor` → `engine/runtime.rs`
- [x] Extract command handlers → `engine/commands.rs`
- [x] Extract slot lifecycle management → `engine/slots.rs`
- [x] Extract merge queue logic → `engine/merge.rs`
- [x] Extract event emission helpers → `engine/events.rs`
- [x] Move integration tests → `engine/tests.rs`
- [x] Verify `cargo fmt --all`, `cargo test -p nexode-daemon`, `cargo check --workspace`, and `cargo doc --workspace`
- [x] Commit decomposition separately (`3cd2355`)

### Part 2: Fix I-016 — Task Transition Semantics

- [x] Add `pre_pause_status: Option<TaskStatus>` to slot runtime
- [x] Record pre-pause state on Paused transitions
- [x] Validate resume transitions against pre-pause state
- [x] Remove `MergeQueue → Paused` from allowed transitions
- [x] Add unit tests for pre-pause tracking
- [x] Add integration test for observer pause → resume flow

### Part 3: Fix I-022 — Async Observer Tick

- [x] Wrap `has_worktree_changes` in `spawn_blocking`
- [x] Concurrent await for all working slots
- [x] Verify existing observer tests pass

### Part 4: Fix I-008 — Daemon CLI with clap

- [x] Add `clap` to `nexode-daemon` dependencies
- [x] Define `#[derive(Parser)]` struct for daemon CLI
- [x] Add `--help` and `--version` support
- [x] Verify daemon starts correctly with existing flags

## Blocked

- None

## Done This Sprint

- Sprint 4 branch created from `origin/main`
- `engine.rs` decomposed into `engine/` submodules with runtime, command, slot, merge, event, and test files
- Part 1 committed separately at `3cd2355` (`[gpt] refactor: decompose daemon engine module`)
- I-016 resolved with pre-pause tracking, guarded resume semantics, and new unit/integration coverage
- I-022 resolved by moving observer git-status checks to concurrent `spawn_blocking` tasks
- I-008 resolved by replacing daemon manual arg parsing with `clap`, while preserving the positional session path and existing flags
- Server-backed daemon integration tests are serialized with `serial_test` to avoid false failures from parallel daemon/worktree interference
- Sprint 4 verification passed:
  - `cargo fmt --all`
  - `cargo test -p nexode-daemon`
  - `cargo test -p nexode-ctl`
  - `cargo check --workspace`
  - `cargo clippy --workspace -- -D warnings`

## Done Previously (Sprint 3)

- Added the observer layer in `crates/nexode-daemon/src/observer.rs` (554 lines)
- Loop detection, uncertainty routing, sandbox enforcement integrated into `engine.rs`
- Extended proto surface with `ObserverAlert`, `ResumeSlot`, `event_sequence`, `AgentStateChanged.slot_id`
- Updated gRPC transport and `nexode-ctl watch` for event-gap detection
- Resolved I-017 and R-005
- Sprint 3 merged to main at `9371feb` (squash merge, 18 files, +1798/-232)

## Next Up

- PC review of Sprint 4 on `agent/gpt/sprint-4-engine-hardening`
- After Sprint 4 merge: Phase 2 (M3) — TUI + VS Code Extension

## Notes

- Sprint 4 prompt: `.agents/prompts/sprint-4-codex.md`
- Part 1 is a pure refactor — commit separately for clean review
- I-016 fix requires reading `docs/architecture/kanban-state-machine.md` carefully
- I-022 fix follows the existing `spawn_blocking` pattern from merge operations in `git.rs`
- `pre_pause_status` is intentionally runtime-only; a naive WAL/checkpoint field addition is not backward-safe with the current bincode format
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
