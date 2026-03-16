---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-15T19:30:00-07:00
task: "Sprint 5 — TUI Dashboard"
branch: "main"
next: gpt
---

# Handoff: Sprint 5 Ready for Codex

## What Just Happened

Sprint 4 (Engine Hardening + Module Decomposition) was reviewed and merged to `main` at `ee82552`. All exit criteria met, all 70 tests pass, two low-severity findings noted (I-025 added).

Sprint 4 resolved:
- **I-016:** Task transitions now context-aware with `pre_pause_status` tracking
- **I-022:** Observer tick uses `JoinSet::spawn_blocking` for git-status
- **I-008:** Daemon CLI migrated to `clap`

New finding: **I-025** (Low) — `Review → Paused` is un-resumable via `ResumeAgent`/`ResumeSlot`; `MoveTask` is the workaround.

Sprint 4 review: `docs/reviews/sprint-4-review.md`

## Sprint 5 Scope

Sprint 5 begins Phase 2 (M3) — the first real user-facing interface. Build a terminal dashboard (`nexode-tui`) that connects to the daemon via gRPC and provides:

1. **Live session overview** — project tree with slot status, budget tracking
2. **Slot detail view** — selected slot's task, agent, status, tokens, cost
3. **Event log** — scrolling feed of all daemon events (state changes, telemetry, observer alerts)
4. **Interactive controls** — navigate tree, pause/resume/kill slots, command input mode

New crate: `crates/nexode-tui/` using `ratatui` + `crossterm`.

## Sprint 5 Prompt

`.agents/prompts/sprint-5-codex.md`

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `.agents/prompts/sprint-5-codex.md` — full sprint instructions
- `crates/nexode-proto/proto/hypervisor.proto` — the TUI renders these entities
- `crates/nexode-ctl/src/main.rs` — existing gRPC client patterns
- `docs/architecture/kanban-state-machine.md` — Kanban columns define TUI status colors

## Context for Codex

### Proto Surface

The TUI is a pure client of the existing gRPC service. Three RPCs:
- `GetFullState` → `FullStateSnapshot` (initial load and reconnect)
- `SubscribeEvents` → stream of `HypervisorEvent` (live updates)
- `DispatchCommand` → `CommandResponse` (user actions)

The proto is at `crates/nexode-proto/proto/hypervisor.proto`. Do NOT modify it.

### Existing Client Patterns

`nexode-ctl` (`crates/nexode-ctl/src/main.rs`, 532 lines) already demonstrates:
- Connecting to the daemon: `HypervisorClient::connect(addr)`
- Fetching state: `client.get_full_state()`
- Subscribing: `client.subscribe_events()`
- Dispatching commands: `client.dispatch_command()`

Use the same patterns in the TUI.

### Architecture

The TUI runs three concurrent tasks:
1. gRPC event receiver (async, reads from event stream)
2. Input handler (blocking, reads terminal key events via crossterm)
3. Render loop (~15 FPS, draws to terminal via ratatui)

These communicate through tokio channels. See the sprint prompt for the `tokio::select!` pattern.
