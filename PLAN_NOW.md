# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 1 — WAL-based crash recovery + Agent Harness abstraction + basic context compiler
- **Deadline:** 2026-03-29 (2 weeks from sprint start)
- **Active Agent:** gpt
- **Previous sprint:** Phase 0 Spike (complete, merged to main)

## Tasks

### Week 1: WAL Recovery

- [x] Read sprint docs: `.agents/CODEX-SPRINT-1.md`, `docs/architecture/wal-recovery.md`, `docs/architecture/agent-harness.md`
- [x] Create branch `agent/gpt/sprint-1-wal-harness`
- [x] Add dependencies: `bincode`, `sha2`, `crc32fast`, `uuid`, `async-trait`, `glob`
- [x] Implement WAL persistence layer (`crates/nexode-daemon/src/wal.rs`) — framed binary format, CRC integrity, fsync writes
- [x] Implement recovery logic (`crates/nexode-daemon/src/recovery.rs`) — checkpoint scan, WAL replay, PID checks, worktree verification
- [x] Integrate WAL into `DaemonEngine` — write entries on state changes, periodic checkpoint, recovery-or-bootstrap startup
- [x] WAL tests — checkpoint round-trip, WAL replay ordering, CRC corruption detection, config drift handling

### Week 2: Agent Harness + Context Compiler

- [x] Define `AgentHarness` trait in `crates/nexode-daemon/src/harness.rs`
- [x] Refactor `build_mock_agent_command` into `MockHarness` implementing the trait
- [x] Implement `ClaudeCodeHarness` — `claude --print` invocation, CLAUDE.md context injection, telemetry parsing
- [x] Implement `CodexCliHarness` — `codex exec --full-auto --json` invocation, `.codex/instructions.md` context injection
- [x] Implement basic context compiler (`crates/nexode-daemon/src/context.rs`) — task + globs + git diff + README
- [x] Wire harness selection into engine (`start_slot` resolves harness from model/config)
- [x] Add `harness` field to session.yaml slot config (optional, backward compatible)
- [x] Harness tests — MockHarness backward compat, context compiler, harness selection, command shape validation

## Blocked

- None

## Done This Sprint

- Added `wal.rs` with framed binary WAL storage, CRC validation, replay, and compaction.
- Added `recovery.rs` with checkpoint serialization, WAL replay, config-drift warnings, worktree verification, and Option A PID kill-and-respawn recovery.
- Added `context.rs` with task/include/exclude/git diff/README context compilation.
- Added `harness.rs` with `MockHarness`, `ClaudeCodeHarness`, and `CodexCliHarness`.
- Kept the harness API synchronous and removed async case handling from this layer; process supervision remains in `process.rs`.
- Extended `session.rs` with optional slot-level `harness` and `session_config_hash()`.
- Refactored `process.rs` so harnesses provide commands, env, setup files, telemetry parsing, and completion detection.
- Wired WAL, recovery, harness selection, context compilation, and checkpointing into `engine.rs`.
- Fixed recovery bootstrap so recovered `Review`/`Done` slots are preserved and only `Pending` or restart-required slots are relaunched.
- Added daemon-level recovery coverage for preserved review state across restart.
- Verification is green: `cargo test -p nexode-daemon` and `cargo check --workspace`.

## Next Up

- Push the branch and open Sprint 1 review.
- Optional follow-up: add an opt-in live CLI smoke test path for installed `claude`/`codex` binaries.
- Optional follow-up: add a daemon-level respawn integration test for a crash during `WORKING` if we introduce a controllable long-running test harness.

## Notes

- Sprint scope defined in `.agents/CODEX-SPRINT-1.md`
- Architecture docs: `docs/architecture/wal-recovery.md`, `docs/architecture/agent-harness.md`
- All Phase 0 decisions (D-002 through D-010) remain binding
- Open issues from Phase 0 review: see `ISSUES.md`
- Recovery strategy: Option A (kill + respawn, don't re-attach to surviving processes)
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
