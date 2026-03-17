# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 7 — TUI Command Hardening
- **Deadline:** 2026-04-19
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-7-tui-command-hardening`
- **Previous sprint:** Sprint 6 — Integration Polish (complete, merged to `main` at `3ae2ffd`)

## Tasks

### Part 1: Reconnection

- [x] Add `ConnectionStatus` enum to `AppState`
- [x] Auto-reconnect on gRPC disconnect with exponential backoff (1s→30s)
- [x] Header bar connection status indicator
- [x] Block command dispatch when disconnected (show status message)
- [x] Event log entries for disconnect/reconnect
- [x] Tests: connection state transitions, command rejection when disconnected

### Part 2: Command UX

- [x] Command history (↑/↓ in command mode, 50 entry cap)
- [x] Status bar feedback with 5-second auto-clear
- [x] Tab-complete for slot IDs in `:move` and `:resume-slot`
- [x] Tests: history cycling, tab-complete matching

### Part 3: Help Overlay

- [x] `?` toggles keybinding reference overlay
- [x] Overlay renders on top of dashboard
- [x] Only `?` and quit keys active while help is visible
- [x] Test: help toggle state

### Part 4: Issue Fixes

- [x] I-019: `demo.sh` waits for DONE after MoveTask
- [x] I-024 (partial): Parse LoopDetected reason strings for specific labels
- [x] Test: reason string parsing for Loop/Stuck/Budget labels

## Blocked

- None

## Done This Sprint

- Added TUI reconnect state with exponential backoff, stale-data rendering, disconnect/reconnect event log entries, and command blocking while disconnected
- Added command history, slot-id tab completion, and a dedicated footer status bar with auto-clear feedback
- Added `?` help overlay modal and key filtering while help is visible
- Fixed `scripts/demo.sh` to wait for DONE after merge queue dispatch
- Improved LoopDetected event labels to distinguish loop, stuck, and budget-velocity reasons
- Verification passed: `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo build -p nexode-tui`, `cargo run -p nexode-tui -- --help`

## Done Previously (Sprint 6)

- Fixed TUI event-gap replay and startup timezone capture
- Fixed Review resume behavior and immediate merge queue draining
- Added cross-crate daemon→TUI gRPC integration coverage
- Added TUI `--version` and corrected harness CLI architecture docs
- 97 tests total (70 daemon + 4 ctl + 23 TUI)

## Next Up

- Sprint 7 review / merge
- After Sprint 7: VS Code Extension (M3b) — requires PC architecture docs first

## Notes

- Sprint 7 prompt: `.agents/prompts/sprint-7-codex.md`
- This is a **TUI-only sprint** — do NOT modify daemon, proto, or ctl crates
- Only non-TUI change: `scripts/demo.sh` (I-019)
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
