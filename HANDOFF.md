---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-17T19:30:00-07:00
task: "Sprint 8 — Daemon Hardening + Issue Sweep"
branch: "main"
next: gpt
---

# Handoff: Sprint 8 Ready for Codex

## What Just Happened

Sprint 7 (TUI Command Hardening) was reviewed and merged to `main` via PR #19 (squash merge at `a93e9af`). All exit criteria met, 108 tests pass, no regressions. Two issues closed (I-019, I-024 partial). No new issues above Info severity.

Sprint 7 delivered:
- Auto-reconnect on gRPC disconnect with exponential backoff (1s→30s)
- ConnectionStatus tracking, header bar indicator, command blocking while disconnected
- Command history (↑/↓, capped at 50)
- Status bar feedback with 5-second auto-clear
- Slot ID tab-completion for `:move` and `:resume-slot`
- Help overlay (`?` toggle)
- I-019: demo.sh waits for DONE after MoveTask
- I-024 (partial): LoopDetected reason string parsing

Sprint 7 review: `docs/reviews/sprint-7-review.md`

## Sprint 8 Scope

Sprint 8 clears the accumulated low-severity issue debt in the daemon and observer crates before moving to the VS Code Extension (M3b). This is a daemon-focused sprint — the TUI is production-ready and should not be modified.

### Part 1: Observer Hardening

- I-020: Guard `observe_output` against unknown/removed slots
- I-021: Configurable alert cooldown for repeated observer findings
- I-023: Filter URLs, source-location patterns, and MIME types from sandbox `candidate_paths`

### Part 2: Proto Cleanup

- I-024: Add `finding_kind` enum to `LoopDetected` proto message (eliminate string parsing in clients)

### Part 3: Harness & Telemetry Fixes

- I-013: Reject empty `ParsedTelemetry` from malformed `TOKENS` lines
- I-029: Update `docs/architecture/agent-harness.md` with Claude `--permission-mode` flags

### Part 4: Infrastructure

- R-006: Add `rust-version` (MSRV) field to all Cargo.toml files and document in README
- Add daemon integration test: start daemon, start TUI client, kill daemon, verify TUI reconnects after daemon restart

## Sprint 8 Prompt

`.agents/prompts/sprint-8-codex.md`

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `.agents/prompts/sprint-8-codex.md` — full sprint instructions
- `ISSUES.md` — focus on I-013, I-020, I-021, I-023, I-024, I-029

## Context for Codex

### Daemon Source

The daemon crate at `crates/nexode-daemon/src/` has these key files:
- `engine/` — Engine module (decomposed in Sprint 4): `mod.rs`, `commands.rs`, `merge.rs`, `observer_tick.rs`, `state.rs`, `snapshot.rs`, `events.rs`, `tick.rs`, `test_support.rs`, `tests.rs`
- `observer.rs` — `LoopDetector`, `SandboxGuard`, observer findings (I-020, I-021, I-023 changes go here)
- `process.rs` — Agent process manager, telemetry parsing (I-013 changes go here)
- `harness.rs` — `AgentHarness` trait, `ClaudeCodeHarness`, `CodexCliHarness`

### Proto Source

- `crates/nexode-proto/proto/hypervisor.proto` — I-024 proto changes go here

### Test Baseline

- Daemon: 67 lib + 3 bin = 70 tests
- Ctl: 4 tests
- TUI: 28 lib + 6 bin = 34 tests
- Total: 108 tests

### Key Constraint

This sprint focuses on daemon + proto. TUI changes should be limited to consuming the new `finding_kind` proto field (I-024). Do NOT modify TUI reconnect, command UX, or help overlay code.
