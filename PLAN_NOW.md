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

- [ ] Read sprint docs: `.agents/CODEX-SPRINT-1.md`, `docs/architecture/wal-recovery.md`, `docs/architecture/agent-harness.md`
- [ ] Create branch `agent/gpt/sprint-1-wal-harness`
- [ ] Add dependencies: `bincode`, `sha2`, `crc32fast`, `uuid`, `async-trait`, `glob`
- [ ] Implement WAL persistence layer (`crates/nexode-daemon/src/wal.rs`) — framed binary format, CRC integrity, fsync writes
- [ ] Implement recovery logic (`crates/nexode-daemon/src/recovery.rs`) — checkpoint scan, WAL replay, PID checks, worktree verification
- [ ] Integrate WAL into `DaemonEngine` — write entries on state changes, periodic checkpoint, recovery-or-bootstrap startup
- [ ] WAL tests — checkpoint round-trip, WAL replay ordering, CRC corruption detection, config drift handling

### Week 2: Agent Harness + Context Compiler

- [ ] Define `AgentHarness` trait in `crates/nexode-daemon/src/harness.rs`
- [ ] Refactor `build_mock_agent_command` into `MockHarness` implementing the trait
- [ ] Implement `ClaudeCodeHarness` — `claude --print` invocation, CLAUDE.md context injection, telemetry parsing
- [ ] Implement `CodexCliHarness` — `codex --approval-mode full-auto` invocation, .codex context injection
- [ ] Implement basic context compiler (`crates/nexode-daemon/src/context.rs`) — task + globs + git diff + README
- [ ] Wire harness selection into engine (`start_slot` resolves harness from model/config)
- [ ] Add `harness` field to session.yaml slot config (optional, backward compatible)
- [ ] Harness tests — MockHarness backward compat, context compiler, harness selection, command shape validation

## Blocked

- None

## Done This Sprint

(Items move here as they're completed)

## Notes

- Sprint scope defined in `.agents/CODEX-SPRINT-1.md`
- Architecture docs: `docs/architecture/wal-recovery.md`, `docs/architecture/agent-harness.md`
- All Phase 0 decisions (D-002 through D-010) remain binding
- Open issues from Phase 0 review: see `ISSUES.md`
- Recovery strategy: Option A (kill + respawn, don't re-attach to surviving processes)
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
