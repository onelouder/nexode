---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-17T01:40:00-07:00
task: "Sprint 7 — TUI Command Hardening"
branch: "main"
next: gpt
---

# Handoff: Sprint 7 Ready for Codex

## What Just Happened

Sprint 6 (Integration Polish) was reviewed and merged to `main` at `3ae2ffd`. All exit criteria met, 97 tests pass, no regressions. Five issues closed (I-007, I-014, I-025, I-027, I-028). One new low-severity finding: I-029 (Claude harness doc omits `--permission-mode` flags).

Sprint 6 delivered:
- I-027: Gap recovery replays triggering event when snapshot is behind
- I-028: Timezone offset captured before tokio, threaded through AppState
- I-025: `resume_target()` handles Review state
- I-007: Merge queue drains immediately on enqueue
- Cross-crate daemon→TUI gRPC integration test
- CLI: `--version` on both binaries
- I-014: Agent harness doc updated

Sprint 6 review: `docs/reviews/sprint-6-review.md`

## Sprint 7 Scope

Sprint 7 makes the TUI production-ready with reconnection resilience, better command UX, and a help overlay. It also closes two open issues.

### Part 1: Reconnection

- Auto-reconnect on gRPC disconnect with exponential backoff (1s→30s cap)
- `ConnectionStatus` enum in `AppState` (Connected / Disconnected / Reconnecting)
- Header bar indicator when disconnected
- Command dispatch blocked with status message when not connected
- Event log entries for disconnect/reconnect events

### Part 2: Command UX

- Command history (↑/↓ in command mode, capped at 50)
- Status bar feedback with 5-second auto-clear
- Tab-complete for slot IDs in `:move` and `:resume-slot`

### Part 3: Help Overlay

- `?` toggles a keybinding reference overlay
- Rendered on top of the dashboard via `Clear` + centered `Paragraph`

### Part 4: Issue Fixes

- I-019: `demo.sh` waits for DONE after MoveTask
- I-024 (partial): Parse LoopDetected reason strings for specific labels

## Sprint 7 Prompt

`.agents/prompts/sprint-7-codex.md`

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `.agents/prompts/sprint-7-codex.md` — full sprint instructions
- `ISSUES.md` — focus on I-019, I-024
- `docs/reviews/sprint-6-review.md`
- `crates/nexode-tui/src/` — all TUI source files

## Context for Codex

### TUI Source

The TUI crate at `crates/nexode-tui/` has a lib.rs (exposing modules) and these source files:
- `main.rs` — gRPC bootstrap, async event loop, command dispatch, terminal cleanup, reconnection logic goes here
- `state.rs` — `AppState`, `apply_event`, `apply_snapshot`, `ConnectionStatus` and command history go here
- `events.rs` — event formatting with pre-computed timezone offset, LoopDetected parsing goes here
- `input.rs` — key bindings, command parsing, help toggle and history navigation go here
- `ui.rs` — dashboard rendering, help overlay and status bar go here

### Test Baseline

- Daemon: 67 lib + 3 bin = 70 tests
- Ctl: 4 tests
- TUI: 17 lib + 6 bin = 23 tests
- Total: 97 tests

### Key Constraint

This sprint is TUI-only. Do NOT modify daemon, proto, or ctl crates. The only non-TUI change is `scripts/demo.sh` (I-019).
