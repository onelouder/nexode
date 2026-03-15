# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 3 — Observer Loops + Safety
- **Deadline:** 2026-03-29
- **Active Agent:** pc (review)
- **Current Branch:** `agent/gpt/sprint-3-observer-safety`
- **Previous sprint:** Sprint 2 real-agent integration + Codex verification (complete, merged to `main`)

## Tasks

### Part 1: Loop Detection

- [x] Add `observer.rs` module with `LoopDetector`
- [x] Detect repeated identical output patterns per slot
- [x] Detect stuck slots after configurable no-diff timeout
- [x] Detect budget-velocity stalls when an optional token budget is configured
- [x] Emit `ObserverAlert` loop events with configurable action (`alert | kill | pause`)
- [x] Wire observer checks into the daemon tick loop and slot lifecycle
- [x] Add unit and engine-level tests for loop alerts and kill/pause intervention

### Part 2: Sandbox Enforcement

- [x] Add `SandboxGuard` to `observer.rs`
- [x] Canonicalize worktree roots before spawn and register them with the guard
- [x] Parse output lines for attempted writes outside the assigned worktree
- [x] Validate post-run changed paths before review/merge
- [x] Add `sandbox_enforcement: bool` to `DaemonConfig`
- [x] Add unit and engine-level sandbox tests

### Part 3: Event Sequence Numbers (R-005)

- [x] Add monotonic `event_sequence` to `HypervisorEvent`
- [x] Add `last_event_sequence` to `FullStateSnapshot`
- [x] Increment sequence numbers on every emitted daemon event
- [x] Surface lagged broadcast consumers as gRPC `DATA_LOSS`
- [x] Update `nexode-ctl watch` to warn and refresh on sequence gaps
- [x] Add transport and engine tests for event sequencing / lag detection

### Part 4: Uncertainty Routing

- [x] Add `UncertaintySignal` under `ObserverAlert`
- [x] Parse uncertainty markers from agent output (`I'm not sure`, `I need clarification`, `DECISION:`)
- [x] Pause the slot on uncertainty and emit an operator-facing alert
- [x] Add `nexode-ctl dispatch resume-slot <slot-id> [instruction...]`
- [x] Add unit and engine-level uncertainty tests

## Blocked

- None

## Done This Sprint

- Added the new observer layer in `crates/nexode-daemon/src/observer.rs`
- Integrated loop detection, uncertainty routing, and sandbox enforcement into `engine.rs`
- Added worktree dirtiness helpers in `git.rs`
- Extended the proto surface with `ObserverAlert`, `ResumeSlot`, `event_sequence`, `last_event_sequence`, and `AgentStateChanged.slot_id`
- Updated gRPC transport and `nexode-ctl watch` for event-gap detection and snapshot refresh
- Added Sprint 3 unit/integration coverage and cleaned up existing style issues so `cargo clippy --workspace -- -D warnings` passes

## Next Up

- pc reviews `agent/gpt/sprint-3-observer-safety`
- After review: open PR, merge, and decide whether any follow-up is needed for unresolved non-sprint issues (`I-016`, `I-018`, `I-019`)

## Notes

- Sprint 3 prompt: `.agents/prompts/sprint-3-codex.md`
- `LoopDetector` uses `provider_config.max_context_tokens` as the optional token-budget baseline when present; the budget-velocity check is otherwise inactive
- `nexode-ctl` now has both `resume-agent` (legacy) and `resume-slot` (observer/operator flow)
- `AgentStateChanged.slot_id` was added while touching the proto/event stream, resolving the low-priority consumer ambiguity tracked as `I-017`
- Live CLI verification was not rerun in Sprint 3; all new coverage uses mock harness flows
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
