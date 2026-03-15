# Sprint 1 Codex Prompt (Retroactive)

> **Note:** This prompt was reconstructed after the fact. Sprint 1 was kicked off
> directly from `.agents/CODEX-SPRINT-1.md` without a separate handoff prompt.
> Recorded here for completeness.

## Task

Execute Sprint 1: WAL Recovery + Agent Harness.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `.agents/CODEX-SPRINT-1.md` — full sprint instructions
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `DECISIONS.md` — all accepted decisions (D-002 through D-010)
   - `ISSUES.md` — open issues from Phase 0 review

2. Read architecture docs:
   - `docs/architecture/wal-recovery.md` — WAL format and recovery protocol
   - `docs/architecture/agent-harness.md` — harness trait design
   - `docs/architecture/kanban-state-machine.md` — state transitions
   - `docs/spec/master-spec.md` (Sections 2, 3, 6, 9)

## Branch

Create and work on: `agent/gpt/sprint-1-wal-harness`

## What to Build

### Week 1: WAL Recovery
- WAL persistence layer (`wal.rs`) — framed binary format `[u32 len][u32 crc][payload]`, append-only at `.nexode/wal.binlog`
- Recovery logic (`recovery.rs`) — checkpoint scan, WAL replay, PID check, worktree verification, config drift warning
- Engine integration — WAL writes on state changes, periodic checkpoint, recovery-or-bootstrap startup

### Week 2: Agent Harness + Context Compiler
- `AgentHarness` trait (`harness.rs`) — `build_command`, `parse_telemetry`, `detect_completion`
- `MockHarness` — refactor existing mock into trait implementation
- `ClaudeCodeHarness` — `claude -p --permission-mode bypassPermissions`, CLAUDE.md injection
- `CodexCliHarness` — `codex exec --full-auto --json`, .codex instructions injection
- Basic context compiler (`context.rs`) — task + include/exclude globs + git diff + README
- Harness selection in engine — model inference + explicit `harness` override in session.yaml

## Exit Criteria

1. WAL persistence — daemon writes entries, recovers after kill + restart
2. CRC integrity — corrupted entries detected and skipped
3. Agent harness trait — Mock, ClaudeCode, CodexCli all implement it, existing tests pass
4. Context compiler — task + globs + diff assembled into ContextPayload
5. Harness selection — different model/harness values select correct implementation
6. Config migration — `harness` field is optional, existing session.yaml files parse correctly

## Verification

```bash
cargo test -p nexode-daemon
cargo check --workspace
```

## Rules

- Commit messages: `[gpt] type: description`
- Branch: `agent/gpt/sprint-1-wal-harness`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session

## Outcome

Sprint 1 was completed and merged as PR #8 (commit `388a224`). Code review in `docs/reviews/sprint-1-review.md`. 35 tests passing, all 6 exit criteria met, recommendation: `ready with follow-ups`. 10 findings tracked in ISSUES.md (I-009 through I-015).
