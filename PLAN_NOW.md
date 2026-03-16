# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 5 — TUI Dashboard
- **Deadline:** 2026-04-05
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-5-tui-dashboard`
- **Status:** Implemented locally, awaiting review/push
- **Previous sprint:** Sprint 4 — Engine Hardening + Module Decomposition (complete, merged to `main` at `ee82552`)

## Tasks

### Part 1: Crate Setup and gRPC Client

- [x] Create `crates/nexode-tui/` workspace member
- [x] Add dependencies: `ratatui`, `crossterm`, `nexode-proto`, `tonic`, `tokio`, `clap`
- [x] CLI args: `--addr <host:port>` (default `http://[::1]:50051`)
- [x] Connect to daemon, fetch `FullStateSnapshot`, subscribe to events
- [x] Create `src/state.rs` with `AppState`, `apply_event()`, `apply_snapshot()`

### Part 2: Dashboard Layout

- [x] Create `src/ui.rs` with three-panel layout (header, main split, event log)
- [x] Render project/slot tree (left panel) with status-colored indicators
- [x] Render slot detail view (right panel)
- [x] Render scrolling event log (bottom panel)
- [x] Create `src/events.rs` with event-to-string formatting for all event types

### Part 3: Keyboard Input and Command Dispatch

- [x] Create `src/input.rs` with key bindings (quit, navigate, pause, resume, kill)
- [x] Command mode (`:`) for structured and free-form commands
- [x] Dispatch commands via gRPC `DispatchCommand`
- [x] Show `CommandResponse` result in header status text

### Part 4: Async Architecture

- [x] Three-task `tokio::select!` loop (gRPC receiver, input handler, render tick)
- [x] Input reader in `spawn_blocking` with channel forwarding
- [x] ~15 FPS render tick

### Part 5: Graceful Terminal Handling

- [x] Raw mode + alternate screen on startup
- [x] Cleanup on quit, Ctrl+C, and panic
- [x] `Drop` guard + panic hook for terminal restoration

## Blocked

- None

## Done This Sprint

- Added `nexode-tui` as a new workspace member and compiled it cleanly
- Implemented a live gRPC client with snapshot bootstrap, event subscription, and gap recovery
- Added ratatui dashboard rendering for project tree, slot detail, and event log
- Added interactive key handling and command dispatch
- Added state/event/command/CLI tests for the new crate

## Verification

- `cargo fmt --all`
- `cargo test -p nexode-tui`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo build -p nexode-tui`
- `cargo run -p nexode-tui -- --help`

## Notes

- Sprint 5 prompt: `.agents/prompts/sprint-5-codex.md`
- The TUI stayed within the approved surface: no daemon or proto changes
- `I-024` still limits observer-event specificity in the UI because the proto flattens loop/stuck/budget findings
- `I-025` remains daemon-side; paused-from-review slots still need `:move <task-id> review` as the operator workaround
