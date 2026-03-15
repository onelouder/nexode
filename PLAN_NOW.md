# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 2 — Real Agent Integration + Critical Fixes
- **Deadline:** 2026-03-29 (2 weeks from sprint start)
- **Active Agent:** gpt
- **Previous sprint:** Sprint 1 — WAL Recovery + Agent Harness (complete, merged to main)

## Tasks

### Week 1: Bug Fixes + Command Acknowledgment

- [ ] Read sprint docs: `.agents/CODEX-SPRINT-2.md`, `docs/architecture/command-ack.md`, `docs/reviews/sprint-1-review.md`
- [ ] Create branch `agent/gpt/sprint-2-real-agents`
- [ ] **Fix I-009:** Change `completion_detected` semantics — non-zero exit always means failure. Add `requires_completion_signal()` to `AgentHarness` trait. Update `process.rs` logic.
- [ ] **Fix I-010:** Emit `AgentStateChanged(Executing)` after `SlotAgentSwapped` in `engine.rs`.
- [ ] **Fix I-015:** Replace JSON substring matching with `serde_json` parsing in `ClaudeCodeHarness` and `CodexCliHarness` completion detection. Add `serde_json` to deps.
- [ ] **R-007:** Update proto — add `command_id` echo, `CommandOutcome` enum to `CommandResponse`.
- [ ] **R-007:** Replace fire-and-forget channel with oneshot request/response in `transport.rs`.
- [ ] **R-007:** Add command validation and result reporting in engine command handler.
- [ ] **R-007:** Update `nexode-ctl` to display command results.
- [ ] Unit tests for all fixes.

### Week 2: Live Integration + Demo

- [ ] Add `live-test` feature flag to `nexode-daemon/Cargo.toml`.
- [ ] Create `tests/live_harness.rs` with gated integration tests.
- [ ] Live smoke test: ClaudeCode harness runs trivial task on temp repo.
- [ ] Live smoke test: CodexCli harness runs trivial task on temp repo.
- [ ] Live full lifecycle: agent completes → MoveTask → merge → DONE.
- [ ] Create `scripts/demo.sh` end-to-end demo script.
- [ ] Verify all existing tests still pass (`cargo test -p nexode-daemon`).

## Blocked

- None

## Done This Sprint

(Updated as work progresses)

## Next Up

- Sprint 3: Observer Loops + Safety (loop detection, uncertainty routing, sandbox enforcement, event sequence numbers)

## Notes

- Sprint scope defined in `.agents/CODEX-SPRINT-2.md`
- Architecture doc: `docs/architecture/command-ack.md`
- All Phase 0 + Sprint 1 decisions remain binding
- Open issues: see `ISSUES.md` — I-009, I-010, I-015 are Sprint 2 targets
- Live tests gated behind `--features live-test` — require `claude` or `codex` CLI installed
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
