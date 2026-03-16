# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 6 — Integration Polish
- **Deadline:** 2026-04-12
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-6-integration-polish` (to be created)
- **Previous sprint:** Sprint 5 — TUI Dashboard (complete, merged to `main` at `4e5f6cf`)

## Tasks

### Part 1: TUI Fixes

- [ ] I-027: Fix event gap recovery to not drop the triggering event (`main.rs:322-330`)
- [ ] I-028: Compute timezone offset at startup before tokio spawns threads, pass through `AppState`

### Part 2: Daemon Fixes

- [ ] I-025: Add `Some(Review) => Some(Review)` to `resume_target()` in `commands.rs` + test
- [ ] I-007: Immediate merge queue drain after `enqueue_merge()` in `slots.rs`

### Part 3: Integration Test

- [ ] Cross-crate integration test: daemon→TUI state flow via gRPC
- [ ] Test event gap recovery end-to-end

### Part 4: Cleanup

- [ ] Add `--version` to TUI CLI (`main.rs` Cli struct)
- [ ] Fix I-014: Update `docs/architecture/agent-harness.md` CLI flags
- [ ] Add `--version` to daemon CLI if not present

## Blocked

- None

## Done This Sprint

- (Sprint 6 not yet started)

## Done Previously (Sprint 5)

- New `nexode-tui` crate with three-panel dashboard
- Live gRPC streaming with event gap recovery
- Interactive controls: navigate, pause/resume/kill, command mode
- Terminal cleanup on exit/signal/panic
- 18 unit tests, status colors aligned to kanban spec (D-009)
- Total: 88 tests (66 daemon + 4 ctl + 18 TUI)

## Next Up

- After Sprint 6: VS Code Extension (M3b) or further TUI enhancements

## Notes

- Sprint 6 prompt: `.agents/prompts/sprint-6-codex.md`
- This is a **polish sprint** — small, focused fixes across existing crates
- The integration test is the most complex deliverable — it proves daemon→TUI works end-to-end
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`
- Proto modifications are allowed only if needed for integration test fixtures
