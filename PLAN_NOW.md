# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Sprint 3 — Observer Loops + Safety
- **Deadline:** 2026-03-29
- **Active Agent:** gpt (next turn)
- **Previous sprint:** Codex CLI Verification (complete, merged to main)

## Tasks

### Part 1: Loop Detection (do first)

- [ ] Add `observer.rs` module with `LoopDetector` struct
- [ ] Implement repeated-output detection (3+ identical output patterns per slot)
- [ ] Implement stuck-timeout detection (configurable, no git diff progress after N minutes)
- [ ] Implement budget-velocity check (>50% tokens consumed with zero worktree changes)
- [ ] Add `ObserverAlert` event emission on loop detection
- [ ] Add configurable action: `on_loop: alert | kill | pause`
- [ ] Add `loop_detection` config block to `DaemonConfig`
- [ ] Unit tests: identical outputs → alert, varied outputs → no alert, kill action → agent killed
- [ ] Integration test: mock looping agent → daemon detects and alerts

### Part 2: Sandbox Enforcement

- [ ] Add `SandboxGuard` to `observer.rs`
- [ ] Resolve worktree canonical path before agent spawn
- [ ] Monitor agent output for file write patterns outside worktree root
- [ ] Post-completion `git diff --name-only` check for path escapes
- [ ] Add `sandbox_enforcement: bool` to `DaemonConfig` (default `true`)
- [ ] Unit tests: path inside → allowed, path outside → flagged
- [ ] Integration test: mock agent writes outside worktree → merge blocked

### Part 3: Event Sequence Numbers (R-005)

- [ ] Add monotonic `event_sequence: u64` to `DaemonEvent` proto (or envelope wrapper)
- [ ] Engine increments counter on every event emit
- [ ] Add `last_event_sequence` field to `FullStateSnapshot`
- [ ] Update `nexode-ctl watch` to print warning on sequence gap
- [ ] Unit tests: sequence is monotonically increasing, snapshot includes latest sequence
- [ ] Integration test: slow consumer detects gap and requests state refresh

### Part 4: Uncertainty Routing

- [ ] Add `UncertaintySignal` variant to `ObserverAlert`
- [ ] Parse agent output for uncertainty markers ("I'm not sure", "DECISION:", etc.)
- [ ] On uncertainty signal: transition slot to PAUSED, emit alert event
- [ ] Support `nexode-ctl dispatch resume-slot <slot-id>` with optional instruction
- [ ] Unit tests: "DECISION:" in output → uncertainty signal → slot paused
- [ ] Integration test: mock agent writes "DECISION: need guidance" → slot transitions to PAUSED

## Blocked

- None

## Done This Sprint

- (Sprint 3 not yet started by Codex)

## Next Up

- pc reviews Sprint 3 code after Codex completes
- Sprint 4 planning

## Notes

- Sprint 3 Codex prompt: `.agents/prompts/sprint-3-codex.md`
- All Phase 0 + Sprint 1 + Sprint 2 decisions remain binding
- Open issues: see `ISSUES.md` — I-004, I-005, I-007, I-008, I-011–I-014, I-016–I-019
- Open risks: R-001–R-003, R-005, R-006, R-008–R-010
- R-005 (broadcast stream drops) is directly addressed by Part 3 of this sprint
- R-008, R-009, R-010 are newly documented risks from Gemini architectural analysis
- Live tests gated behind `--features live-test` — require `claude` or `codex` CLI installed
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
