# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 5 — TUI Dashboard
- **Deadline:** 2026-04-05
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-5-tui-dashboard` (to be created)
- **Previous sprint:** Sprint 4 — Engine Hardening + Module Decomposition (complete, merged to `main` at `ee82552`)

## Tasks

### Part 1: Crate Setup and gRPC Client

- [ ] Create `crates/nexode-tui/` workspace member
- [ ] Add dependencies: `ratatui`, `crossterm`, `nexode-proto`, `tonic`, `tokio`, `clap`
- [ ] CLI args: `--addr <host:port>` (default `http://[::1]:50051`)
- [ ] Connect to daemon, fetch `FullStateSnapshot`, subscribe to events
- [ ] Create `src/state.rs` with `AppState`, `apply_event()`, `apply_snapshot()`

### Part 2: Dashboard Layout

- [ ] Create `src/ui.rs` with three-panel layout (header, main split, event log)
- [ ] Render project/slot tree (left panel) with status-colored indicators
- [ ] Render slot detail view (right panel)
- [ ] Render scrolling event log (bottom panel)
- [ ] Create `src/events.rs` with event-to-string formatting for all event types

### Part 3: Keyboard Input and Command Dispatch

- [ ] Create `src/input.rs` with key bindings (quit, navigate, pause, resume, kill)
- [ ] Command mode (`:`) for structured and free-form commands
- [ ] Dispatch commands via gRPC `DispatchCommand`
- [ ] Show `CommandResponse` result in status bar

### Part 4: Async Architecture

- [ ] Three-task `tokio::select!` loop (gRPC receiver, input handler, render tick)
- [ ] Input reader in `spawn_blocking` with channel forwarding
- [ ] ~15 FPS render tick

### Part 5: Graceful Terminal Handling

- [ ] Raw mode + alternate screen on startup
- [ ] Cleanup on quit, Ctrl+C, and panic
- [ ] `Drop` guard or panic hook for terminal restoration

## Blocked

- None

## Done This Sprint

- (Sprint 5 not yet started)

## Done Previously (Sprint 4)

- Decomposed `engine.rs` (~2700 lines) into `engine/` directory with 8 sub-modules
- I-016 resolved: `pre_pause_status` tracking for Kanban-compliant transitions
- I-022 resolved: Observer tick uses `JoinSet::spawn_blocking`
- I-008 resolved: Daemon CLI migrated to `clap`
- 70 tests total (63 daemon lib + 3 daemon bin + 4 ctl), all passing

## Next Up

- After Sprint 5: VS Code extension (M3 continuation), or Phase 2 polish sprint

## Notes

- Sprint 5 prompt: `.agents/prompts/sprint-5-codex.md`
- The TUI is a **new crate** — do NOT modify daemon, proto, or ctl crates
- Proto surface: `GetFullState`, `SubscribeEvents`, `DispatchCommand`
- Use `ratatui` built-in widgets, keep it functional over pretty
- Minimum terminal: 80x24
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
