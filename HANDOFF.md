---
agent: gpt
status: handoff
from: gpt
timestamp: 2026-03-15T14:30:16-07:00
task: "Sprint 3 — Observer Loops + Safety"
branch: "agent/gpt/sprint-3-observer-safety"
next: pc
---

# Handoff: Sprint 3 Ready For PC Review

## What Landed

Sprint 3 adds the daemon safety layer required for unattended operation:

1. **Loop detection**
   - New `crates/nexode-daemon/src/observer.rs` with `LoopDetector`
   - Detects repeated identical output, no-diff stalls, and optional budget-velocity stalls
   - Supports configurable `on_loop` intervention: `alert`, `kill`, or `pause`

2. **Sandbox enforcement**
   - New `SandboxGuard` in `observer.rs`
   - Canonical worktree root registered before spawn
   - Output parser flags obvious escape attempts like `../../../etc/shadow`
   - Post-run changed-path validation runs before a slot can move into review / merge

3. **Reliable event sequencing**
   - `HypervisorEvent.event_sequence`
   - `FullStateSnapshot.last_event_sequence`
   - `transport.rs` now surfaces lagged consumers as gRPC `DATA_LOSS`
   - `nexode-ctl watch` warns on sequence gaps and refreshes state

4. **Uncertainty routing**
   - New `ObserverAlert` proto event with `LoopDetected`, `SandboxViolation`, and `UncertaintySignal`
   - Uncertainty markers in agent output now pause the slot and emit an alert
   - Added `resume-slot` command path in `nexode-ctl`

## Primary Files

| Area | Files |
|---|---|
| Observer core | `crates/nexode-daemon/src/observer.rs` |
| Engine integration | `crates/nexode-daemon/src/engine.rs` |
| Transport / gap detection | `crates/nexode-daemon/src/transport.rs`, `crates/nexode-ctl/src/main.rs` |
| Proto surface | `crates/nexode-proto/proto/hypervisor.proto` |
| Worktree diff helpers | `crates/nexode-daemon/src/git.rs` |
| Mock test behaviors | `crates/nexode-daemon/src/harness.rs` |

## Verification

- `cargo fmt --all`
- `cargo test -p nexode-daemon`
- `cargo test -p nexode-ctl`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- Branch pushed to `origin/agent/gpt/sprint-3-observer-safety`
- Review URL: `https://github.com/onelouder/nexode/pull/new/agent/gpt/sprint-3-observer-safety`

Current test counts:
- `nexode-daemon`: 58 passing tests
- `nexode-ctl`: 4 passing tests

## Reviewer Focus

1. **Observer action semantics**
   - `LoopAction::Kill` currently force-stops the running slot process and leaves the task in `PAUSED`
   - This is intentional for operator recovery, but it is the main behavior choice to scrutinize

2. **Budget-velocity inference**
   - There is still no first-class token-budget field in session config
   - Sprint 3 uses `provider_config.max_context_tokens` as the optional baseline when present
   - If absent, the budget-velocity check is inactive

3. **Proto/event surface**
   - `AgentStateChanged.slot_id` was added while touching sequencing, resolving `I-017`
   - `resume-slot` was added without removing the older agent-id command path

## Open / Not Changed

- `I-016` task-transition semantics are still open
- `I-018` telemetry double-count risk is still open
- `I-019` demo polling still exits before guaranteed `DONE`
- No live Claude/Codex verification was rerun in Sprint 3; all new safety coverage is mock-driven

## Suggested Next Step

- pc reviews `agent/gpt/sprint-3-observer-safety` against `.agents/prompts/sprint-3-codex.md`
- If review is clean, open PR and merge

## PC Review Brief

Read first:
- `AGENTS.md`
- `PLAN_NOW.md`
- `HANDOFF.md`
- `.agents/prompts/sprint-3-codex.md`
- `ISSUES.md`

Review focus:
- `crates/nexode-daemon/src/observer.rs`
- `crates/nexode-daemon/src/engine.rs`
- `crates/nexode-daemon/src/transport.rs`
- `crates/nexode-ctl/src/main.rs`
- `crates/nexode-proto/proto/hypervisor.proto`

Please verify:
- loop intervention semantics, especially `LoopAction::Kill -> PAUSED`
- sandbox output/path checks and whether any escape cases remain
- event gap behavior for slow clients and `nexode-ctl watch`
- uncertainty pause/resume flow
- whether the `provider_config.max_context_tokens` inference for budget velocity is acceptable for Sprint 3
