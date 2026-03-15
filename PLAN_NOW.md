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

- [x] Read sprint docs: `.agents/CODEX-SPRINT-2.md`, `docs/architecture/command-ack.md`, `docs/reviews/sprint-1-review.md`
- [x] Create branch `agent/gpt/sprint-2-real-agents`
- [x] **Fix I-009:** Change `completion_detected` semantics — non-zero exit always means failure. Add `requires_completion_signal()` to `AgentHarness` trait. Update `process.rs` logic.
- [x] **Fix I-010:** Emit `AgentStateChanged(Executing)` after `SlotAgentSwapped` in `engine.rs`.
- [x] **Fix I-015:** Replace JSON substring matching with `serde_json` parsing in `ClaudeCodeHarness` and `CodexCliHarness` completion detection. Add `serde_json` to deps.
- [x] **R-007:** Update proto — add `command_id` echo, `CommandOutcome` enum to `CommandResponse`.
- [x] **R-007:** Replace fire-and-forget channel with oneshot request/response in `transport.rs`.
- [x] **R-007:** Add command validation and result reporting in engine command handler.
- [x] **R-007:** Update `nexode-ctl` to display command results.
- [x] Unit tests for all fixes.

### Week 2: Live Integration + Demo

- [x] Add `live-test` feature flag to `nexode-daemon/Cargo.toml`.
- [x] Create `tests/live_harness.rs` with gated integration tests.
- [x] Add gated ClaudeCode smoke test on a temp repo.
- [x] Add gated CodexCli smoke test on a temp repo.
- [x] Add gated full lifecycle test: agent completes → MoveTask → merge → DONE.
- [x] Create `scripts/demo.sh` end-to-end demo script.
- [x] Verify all existing tests still pass (`cargo test -p nexode-daemon`, `cargo test -p nexode-ctl`, `cargo check --workspace`).

## Blocked

- None

## Done This Sprint

- Fixed I-009 in `process.rs`: success now requires zero exit code, and real harnesses can require a completion signal.
- Fixed I-010 in `engine.rs`: `SlotAgentSwapped` now also emits `AgentStateChanged(Executing)` for the replacement agent.
- Fixed I-015 in `harness.rs`: Claude/Codex completion detection now parses JSON instead of substring matching.
- Kept the harness layer synchronous: command building and completion parsing stay in `harness.rs`, while async process lifecycle and streaming remain in `process.rs`.
- Implemented R-007 across proto, transport, engine, and CLI:
  - `CommandResponse` now echoes `command_id`
  - `CommandOutcome` is returned to the client
  - transport uses oneshot request/response with timeout
  - engine validates slot existence and task-state transitions
  - `nexode-ctl` prints real command outcomes
- Added process, transport, engine, harness, and CLI tests for Sprint 2 behavior.
- Added gated live harness integration tests and `scripts/demo.sh`.
- Verified local test suite:
  - `cargo test -p nexode-daemon`
  - `cargo test -p nexode-ctl`
  - `cargo check --workspace`
  - `cargo test -p nexode-daemon --features live-test --test live_harness` with blanked keys to confirm compile/self-skip path

## Next Up

- Review `agent/gpt/sprint-2-real-agents`.
- Run at least one real live harness smoke test with valid CLI credentials before merge if CLI access is available.
- Sprint 3: Observer Loops + Safety (loop detection, uncertainty routing, sandbox enforcement, event sequence numbers)

## Notes

- Sprint scope defined in `.agents/CODEX-SPRINT-2.md`
- Architecture doc: `docs/architecture/command-ack.md`
- All Phase 0 + Sprint 1 decisions remain binding
- Open issues: see `ISSUES.md` — I-009, I-010, I-015 are Sprint 2 targets
- Live tests gated behind `--features live-test` — require `claude` or `codex` CLI installed
- Live harness tests are implemented and compile; real credential-backed execution is still pending in this environment
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
