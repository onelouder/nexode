---
agent: gpt
status: handoff
from: gpt
timestamp: 2026-03-15T22:05:00-07:00
task: "Sprint 5 — TUI Dashboard"
branch: "agent/gpt/sprint-5-tui-dashboard"
next: pc
---

# Handoff: Sprint 5 TUI Dashboard Ready for Review

## What Landed

Sprint 5 is implemented locally on `agent/gpt/sprint-5-tui-dashboard`.

New crate:
- `crates/nexode-tui/`

Key files:
- `crates/nexode-tui/src/main.rs`
- `crates/nexode-tui/src/state.rs`
- `crates/nexode-tui/src/events.rs`
- `crates/nexode-tui/src/input.rs`
- `crates/nexode-tui/src/ui.rs`

Core behavior:
- Connects to the daemon over gRPC with `--addr` (default `http://[::1]:50051`)
- Fetches `FullStateSnapshot` on startup
- Subscribes to `SubscribeEvents` and applies live updates into local `AppState`
- Recovers from event gaps / `DATA_LOSS` by refetching snapshot
- Renders a ratatui dashboard with:
  - header bar
  - project/slot tree
  - selected slot detail
  - reverse-chronological event log
  - footer help / command mode bar
- Supports keyboard input:
  - `q` / `Ctrl+C` quit
  - `↑` / `↓` navigate
  - `Enter` select slot
  - `p` pause selected slot
  - `r` resume selected slot
  - `k` kill selected slot
  - `:` command mode
  - `Esc` exit command mode
- Dispatches structured commands via gRPC:
  - `:move <task-id> <status>`
  - `:resume-slot <slot-id> [instruction]`
  - free-form text falls back to `SlotDispatch` when a slot is selected, otherwise `ChatDispatch`
- Restores terminal state on normal exit, Ctrl+C / SIGTERM, and panic via cleanup guard + panic hook

## Verification

Passed locally:
- `cargo fmt --all`
- `cargo test -p nexode-tui`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo build -p nexode-tui`
- `cargo run -p nexode-tui -- --help`

`cargo test -p nexode-tui` currently has 18 tests covering:
- state snapshot/event application
- event formatting
- command parsing
- CLI parsing

## Important Notes for Review

1. No daemon or proto changes were made.
   Sprint 5 stayed inside the allowed surface: workspace wiring plus the new `nexode-tui` crate.

2. `I-024` is still a proto limitation, not a TUI bug.
   `ObserverAlert::LoopDetected` still flattens loop/stuck/budget-velocity into one proto variant, so the event log renders that bucket as `loop/stuck/budget` instead of pretending it can tell them apart.

3. `I-025` is still open.
   The TUI exposes resume through existing daemon commands, but a slot paused from `REVIEW` still inherits the daemon-side asymmetry. The operator workaround remains `:move <task-id> review`.

4. Event cost fields remain snapshot-driven.
   The proto's telemetry event carries token delta and TPS, but not cost/session-budget deltas, so live token counts update immediately while cost values refresh from snapshots and budget alerts.

## Review Focus

- `AppState` correctness for snapshot replacement and incremental event application
- gRPC gap recovery behavior in `main.rs`
- command-mode parsing / dispatch ergonomics
- terminal cleanup safety in `main.rs`
- whether the three-panel ratatui layout is good enough for Sprint 5 without further daemon/proto changes

## Next Step

PC review the branch for Sprint 5 readiness, then push / open review if clean.
