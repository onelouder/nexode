# PLAN_NOW — Current Sprint

> What's happening right now. Updated at each handoff.

## Current Sprint

- **Sprint:** 8
- **Goal:** Sprint 8 — Daemon Hardening + Issue Sweep
- **Deadline:** 2026-04-26
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-8-daemon-hardening`
- **Previous sprint:** Sprint 7 — TUI Command Hardening (complete, merged to `main` via PR #19 at `a93e9af`)

## Tasks

### Part 1: Observer Hardening

- [x] I-020: Guard `observe_output` against unknown/removed slots
- [x] I-021: Configurable alert cooldown for repeated observer findings
- [x] I-023: Filter URLs and source-location patterns from sandbox `candidate_paths`
- [x] Tests: slot-exists guard, alert cooldown behavior, URL/source-loc filtering

### Part 2: Proto Cleanup

- [x] I-024: Add `finding_kind` enum to `LoopDetected` proto message
- [x] Update daemon observer event mapping to emit the new enum field
- [x] Update TUI `events.rs` to use proto enum instead of string parsing
- [x] Tests: proto mapping and TUI event formatting with the new field

### Part 3: Harness & Telemetry Fixes

- [x] I-013: Reject empty `ParsedTelemetry` from malformed `TOKENS` lines
- [x] I-029: Update Claude harness doc with `--permission-mode` flags
- [x] Tests: malformed TOKENS line rejection

### Part 4: Infrastructure

- [x] R-006: Add `rust-version` MSRV to all Cargo.toml files
- [x] R-006: Document MSRV in README.md
- [x] Integration test: daemon restart → TUI reconnect verification

## Blocked

- (none)

## Done This Sprint

- Guarded observer output against unknown/removed slots, added cooldown-based re-alerting, and filtered URLs, source locations, MIME types, and module-like tokens from sandbox candidate path extraction
- Added proto `FindingKind` enum and `LoopDetected.finding_kind`; daemon observer events now populate it and the TUI prefers it with a fallback to Sprint 7 reason parsing
- Rejected empty malformed `TOKENS` telemetry payloads before they reach accounting/WAL updates
- Updated the Claude harness architecture doc to match the real CLI invocation, including `-p` and `--permission-mode bypassPermissions`
- Added `rust-version = "1.85"` to all crate manifests and documented Rust 1.85+ in `README.md`
- Added daemon restart/reconnect integration coverage and expanded observer/proto regression tests
- Verification passed: `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo build -p nexode-tui`, `cargo build -p nexode-daemon`

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
