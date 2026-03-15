# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Land the Phase 0 runtime slice: workspace, proto, parser, accountant, git orchestration, mock agent lifecycle, and gRPC skeleton
- **Deadline:** 2026-03-28
- **Active Agent:** gpt

## Tasks

- [x] Read AGENTS.md, HANDOFF.md, sprint brief, decisions, and referenced spec sections
- [x] Create branch `agent/gpt/phase-0-spike` and claim the handoff
- [x] Initialize Cargo workspace with `nexode-daemon`, `nexode-proto`, and `nexode-ctl`
- [x] Add `hypervisor.proto` v2 with D-003, D-006, and D-009 amendments
- [x] Verify the workspace builds far enough for the next implementation pass
- [x] Implement the Session Config Manager with include resolution, v1 fallback, `.nexode.yaml` merge, and D-004 cascade behavior
- [x] Implement the SQLite token accountant with `token_log`, `project_costs`, and budget alert evaluation
- [x] Implement the Git Worktree Orchestrator with worktree lifecycle and merge-plus-verify flow
- [x] Implement the mock agent process manager with stdout/stderr streaming, crash respawn, and watchdog timeout handling
- [x] Implement the daemon-side gRPC skeleton with event streaming, command dispatch, and full-state stubs
- [ ] Start the merge queue or `nexode-ctl` client pass after review

## Blocked

- None

## Done This Sprint

- [x] Completed items move here
- [x] `cargo test -p nexode-daemon`
- [x] `cargo check --workspace`

## Notes

- Phase 0 scope is defined in `.agents/CODEX-SPRINT-0.md`.
- Do not modify `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, or `docs/architecture/*`.
- Session parser tests cover include cycles, unknown fields, repo-local overrides, explicit array clears, and v1 wrapping.
- Token accountant tests cover project/session aggregation plus soft/hard budget alerts.
- Worktree tests cover create/list/remove, verification-gated merge success, git conflict detection, and verify-failure refusal to update the target branch.
- Process-manager tests cover telemetry parsing, crash recovery with `SlotAgentSwapped`, and watchdog-triggered respawn.
- gRPC tests cover over-the-wire `GetFullState`, `DispatchCommand`, and `SubscribeEvents`.
- Current branch state is being pushed for review.
