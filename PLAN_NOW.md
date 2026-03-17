# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 7 — TUI Command Hardening
- **Deadline:** 2026-04-19
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-7-tui-command-hardening` (to be created)
- **Previous sprint:** Sprint 6 — Integration Polish (complete, merged to `main` at `3ae2ffd`)

## Tasks

### Part 1: Reconnection

- [ ] Add `ConnectionStatus` enum to `AppState`
- [ ] Auto-reconnect on gRPC disconnect with exponential backoff (1s→30s)
- [ ] Header bar connection status indicator
- [ ] Block command dispatch when disconnected (show status message)
- [ ] Event log entries for disconnect/reconnect
- [ ] Tests: connection state transitions, command rejection when disconnected

### Part 2: Command UX

- [ ] Command history (↑/↓ in command mode, 50 entry cap)
- [ ] Status bar feedback with 5-second auto-clear
- [ ] Tab-complete for slot IDs in `:move` and `:resume-slot`
- [ ] Tests: history cycling, tab-complete matching

### Part 3: Help Overlay

- [ ] `?` toggles keybinding reference overlay
- [ ] Overlay renders on top of dashboard
- [ ] Only `?` and quit keys active while help is visible
- [ ] Test: help toggle state

### Part 4: Issue Fixes

- [ ] I-019: `demo.sh` waits for DONE after MoveTask
- [ ] I-024 (partial): Parse LoopDetected reason strings for specific labels
- [ ] Test: reason string parsing for Loop/Stuck/Budget labels

## Blocked

- None

## Done This Sprint

- (Sprint 7 not yet started)

## Done Previously (Sprint 6)

- Fixed TUI event-gap replay and startup timezone capture
- Fixed Review resume behavior and immediate merge queue draining
- Added cross-crate daemon→TUI gRPC integration coverage
- Added TUI `--version` and corrected harness CLI architecture docs
- 97 tests total (70 daemon + 4 ctl + 23 TUI)

## Next Up

- After Sprint 7: VS Code Extension (M3b) — requires PC architecture docs first

## Notes

- Sprint 7 prompt: `.agents/prompts/sprint-7-codex.md`
- This is a **TUI-only sprint** — do NOT modify daemon, proto, or ctl crates
- Only non-TUI change: `scripts/demo.sh` (I-019)
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
