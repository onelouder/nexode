---
agent: gpt
status: review
from: gpt
timestamp: 2026-03-15T08:11:49-07:00
task: "Sprint 1 — WAL Recovery + Agent Harness"
branch: "agent/gpt/sprint-1-wal-harness"
---

# Handoff: Sprint 1 Progress

## What Landed

- `crates/nexode-daemon/src/wal.rs`
  - Framed append-only WAL at `.nexode/wal.binlog`
  - Entry CRC validation, ordered replay, checkpoint compaction
- `crates/nexode-daemon/src/recovery.rs`
  - Checkpoint serialization/deserialization
  - WAL replay into persisted runtime state
  - Config drift warning
  - Missing worktree cleanup
  - Option A recovery: kill surviving PID and restart `WORKING` slot
- `crates/nexode-daemon/src/context.rs`
  - Task/include/exclude/git diff/README context compiler
- `crates/nexode-daemon/src/harness.rs`
  - `AgentHarness` trait
  - `MockHarness`, `ClaudeCodeHarness`, `CodexCliHarness`
  - Harness inference and explicit override handling
  - Async harness cases intentionally eliminated; harness construction/parsing stays synchronous
- `crates/nexode-daemon/src/session.rs`
  - Optional `slot.harness`
  - `session_config_hash()` for WAL session drift detection
- `crates/nexode-daemon/src/process.rs`
  - Harness-driven command/env/setup-file execution
  - Harness telemetry parsing + completion detection
- `crates/nexode-daemon/src/engine.rs`
  - Recovery-or-bootstrap startup path
  - WAL writes for slot state, telemetry, merge outcomes
  - Periodic checkpoints
  - Harness/context-based `start_slot()`
  - Recovery-aware bootstrap that preserves recovered `Review`/`Done` state

## Verification

- `cargo test -p nexode-daemon`
  - 35 tests passing
- `cargo check --workspace`
  - passing

Key coverage now includes:

- WAL framing / CRC / compaction
- Recovery replay / config drift / PID restart planning
- Context compiler
- Harness selection and command shape
- Mock harness backward compatibility through engine integration
- Daemon restart preserving recovered `Review` state without relaunching the slot

## Important Notes

- Real harness command shapes were aligned to local CLI help:
  - `claude -p --permission-mode bypassPermissions --model ...`
  - `codex exec --full-auto --json --model ...`
- The harness layer is intentionally synchronous.
  - No `async-trait` or async case handling remains in the harness API.
  - Process lifecycle and streaming stay in `process.rs`; harnesses only build commands and parse lines.
- Engine tests that previously relied on the old mock-only launcher now specify `harness: "mock"` explicitly so Sprint 1 inference does not invoke real CLIs during test runs.
- The new recovery bootstrap logic only relaunches:
  - slots explicitly marked for restart by recovery
  - slots that are still `Pending` after replay
  - recovered `Review`, `Done`, `Paused`, `Resolving`, `Archived`, and `MergeQueue` states are preserved

## Remaining Follow-Ups

- Branch should be pushed and reviewed from `agent/gpt/sprint-1-wal-harness`.
- Optional: add opt-in live smoke tests for installed `claude` / `codex`.
- Optional: add a daemon-level crash-recovery integration test for a slot that is still `WORKING` at crash time using a controllable long-running harness fixture.
