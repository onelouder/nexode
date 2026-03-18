# PLAN_NOW — Current Sprint

> What's happening right now. Updated at each handoff.

## Current Sprint

- **Sprint:** 8
- **Goal:** Sprint 8 — Daemon Hardening + Issue Sweep
- **Deadline:** 2026-04-26
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-8-daemon-hardening` (to be created)
- **Previous sprint:** Sprint 7 — TUI Command Hardening (complete, merged to `main` via PR #19 at `a93e9af`)

## Tasks

### Part 1: Observer Hardening

- [ ] I-020: Guard `observe_output` against unknown/removed slots
- [ ] I-021: Configurable alert cooldown for repeated observer findings
- [ ] I-023: Filter URLs and source-location patterns from sandbox `candidate_paths`
- [ ] Tests: slot-exists guard, alert cooldown behavior, URL/source-loc filtering

### Part 2: Proto Cleanup

- [ ] I-024: Add `finding_kind` enum to `LoopDetected` proto message
- [ ] Update daemon `observer_tick.rs` to emit the new enum field
- [ ] Update TUI `events.rs` to use proto enum instead of string parsing
- [ ] Tests: proto field round-trip, TUI event formatting with new field

### Part 3: Harness & Telemetry Fixes

- [ ] I-013: Reject empty `ParsedTelemetry` from malformed `TOKENS` lines
- [ ] I-029: Update Claude harness doc with `--permission-mode` flags
- [ ] Tests: malformed TOKENS line rejection

### Part 4: Infrastructure

- [ ] R-006: Add `rust-version` MSRV to all Cargo.toml files
- [ ] R-006: Document MSRV in README.md
- [ ] Integration test: daemon restart → TUI reconnect verification

## Blocked

- (none)

## Done This Sprint

- (Sprint 8 not yet started)

## Done Previously (Sprint 7)

- Added TUI reconnect with exponential backoff, stale-data rendering, disconnect/reconnect event log entries, command blocking while disconnected
- Added command history, slot-id tab completion, status bar feedback with auto-clear
- Added `?` help overlay modal and key filtering
- Fixed `scripts/demo.sh` to wait for DONE after merge queue dispatch
- Improved LoopDetected event labels to distinguish loop, stuck, and budget-velocity reasons
- 108 tests pass

## Next Up

- Sprint 8 review / merge
- After Sprint 8: VS Code Extension (M3b) — requires PC architecture docs first

## Notes

- Sprint 8 is daemon-focused. TUI is production-ready and should only change for I-024 proto integration.
- The proto change (I-024) is the riskiest item — it modifies the wire format. Backward compatibility: new enum field defaults to 0 (unspecified), so older TUI versions still work.
- Alert cooldown (I-021) needs a new config field in `observer` settings. Use `alert_cooldown_seconds: u64` with a default of 300 (5 minutes).
