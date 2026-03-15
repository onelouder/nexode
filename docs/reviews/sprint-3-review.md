# Sprint 3 Review — Observer Loops + Safety

**Reviewer:** pc  
**Branch:** `agent/gpt/sprint-3-observer-safety`  
**Base:** `main`  
**Commits reviewed:** `6f97e6c`, `df8ee53`  
**Date:** 2026-03-15  

## Summary

Sprint 3 delivers all four planned features: loop detection, sandbox enforcement, event sequencing with gap recovery, and uncertainty routing. The implementation is clean, well-tested, and integrates into the existing engine without distorting the architecture. 58 daemon tests + 4 ctl tests all pass. No blockers.

## Exit Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Loop detection catches repeated identical output patterns and stuck agents | ✅ Met |
| 2 | Sandbox enforcement prevents file writes outside worktree root | ✅ Met |
| 3 | Events carry sequence numbers; clients can detect and recover from gaps | ✅ Met |
| 4 | Uncertainty routing pauses agents that signal they're stuck | ✅ Met |
| 5 | All existing tests still pass | ✅ Met |

## Findings

### F-001 (Low): `observe_output` creates slot state for unknown slots

**File:** `observer.rs:118`  
**Details:** `observe_output()` calls `self.slots.entry(slot_id).or_default()` unconditionally, creating a `SlotLoopState` even if `observe_status()` was never called for that slot. In the current engine integration this is fine because output only arrives for slots that are already in `Working` state, but the detector itself doesn't enforce that invariant. If output arrives for a slot that was already removed (e.g., due to a race between process event delivery and a status transition), the detector silently re-creates state for a dead slot.

**Recommendation:** Consider guarding `observe_output` with a check that the slot already exists in the map, or document that callers must not send output for non-working slots.

### F-002 (Low): One-shot alert emission means resumed slots don't re-alert

**File:** `observer.rs:88-91` (emitted_*_alert flags), `engine.rs` (ResumeSlot handler)  
**Details:** Each `SlotLoopState` has `emitted_loop_alert`, `emitted_stuck_alert`, `emitted_budget_alert`, and `emitted_uncertainty_alert` flags. Once an alert fires, it won't fire again for that slot. When `ResumeSlot` re-dispatches a slot via `start_slot()`, `start_slot()` calls `self.loop_detector.reset_slot(slot_id)` which removes the slot entirely — so on resume, the flags are indeed cleared.

However, if the operator resumes the slot after a `LoopAction::Alert` (which doesn't pause or kill), the slot continues running and the loop detector will never alert again for the same loop pattern because the slot wasn't reset. This is technically correct per the handoff's "alert-only" semantics, but an operator who sees an alert and doesn't act gets no second warning.

**Recommendation:** Document this behavior. Consider adding a configurable `alert_cooldown` duration that re-arms the alert flag after N seconds, or log at debug level when a repeated condition is suppressed.

### F-003 (Low): `run_observer_tick` calls `has_worktree_changes` synchronously in async context

**File:** `engine.rs:1042-1097` (`run_observer_tick`)  
**Details:** The observer tick iterates all working slots and calls `orchestrator.has_worktree_changes(&worktree_path)` which runs `git status --porcelain` via `std::process::Command`. This is a blocking call in the async engine loop. For Phase 0 with a small number of slots, this is acceptable. At scale (10-15 concurrent agents), running N git-status commands synchronously per tick (every 2s) could block the engine's tokio runtime.

**Recommendation:** For now, acceptable. Track for Phase 2+. The existing pattern of `spawn_blocking` used in merge operations should be applied here when slot counts grow.

### F-004 (Low): `candidate_paths` false positive on common CLI patterns

**File:** `observer.rs:341-353`  
**Details:** The `candidate_paths` function matches any whitespace-delimited token containing `/` or `\\`. This will match common CLI output patterns like `https://github.com/...`, `application/json`, `src/lib.rs:42: warning:`, or URLs in agent output. These strings often contain `/` but aren't file write attempts.

In practice, the false-positive rate is mitigated by `resolve_candidate_path` — most of these strings won't escape the worktree root when resolved. But a URL like `/etc/passwd` appearing in agent output (e.g., in a log message or error trace) would trigger a sandbox violation.

**Recommendation:** Consider filtering out tokens that look like URLs (`http://`, `https://`), Rust source locations (`*.rs:N:`), or other common non-path patterns. Low urgency since the current behavior is conservative (false pause > false pass).

### F-005 (Low): `ResumeSlot` appends operator guidance without separator clarity

**File:** `engine.rs:480-490`  
**Details:** When `ResumeSlot` includes an instruction, the slot's task is updated to `"{original_task}\n\nOperator guidance:\n{instruction}"`. If the slot is paused and resumed multiple times, guidance accumulates:
```
Original task

Operator guidance:
First instruction

Operator guidance:
Second instruction
```
This isn't harmful but could bloat the context payload. Also, the `Operator guidance:` header isn't a recognized marker in `detect_uncertainty`, so it won't trigger re-pausing — which is correct.

**Recommendation:** Minor. Consider limiting accumulated guidance (e.g., keep only the latest), or use a structured field rather than string concatenation.

### F-006 (Low): `LoopDetected` proto flattens three distinct finding kinds

**File:** `engine.rs:1825-1833` (`observer_payload`), `hypervisor.proto`  
**Details:** `ObserverFindingKind::LoopDetected`, `Stuck`, and `BudgetVelocity` all map to the same proto variant `observer_alert::Detail::LoopDetected`. The `reason` string differentiates them, but a UI client can't switch on the finding kind without string parsing.

**Recommendation:** Either add a `finding_kind` enum field to the `LoopDetected` proto message, or split into three proto variants. Low urgency — the `reason` string is descriptive enough for the TUI.

### F-007 (Info): `event_sequence` field number 11 creates a gap in the proto

**File:** `hypervisor.proto` — `HypervisorEvent.event_sequence = 11`  
**Details:** Field numbers 4-10 are the oneof payload variants, and `event_sequence` is assigned field 11. The `ObserverAlert` variant is assigned field 12. This is correct and intentional — oneof members share the field number space. No issue here, just noting the numbering for future reference.

### F-008 (Info): Clippy/style cleanups are mixed into the sprint commits

**Details:** Several files received `if let` chain cleanups (accounting.rs, context.rs, recovery.rs, session.rs) replacing nested `if let Some(...) { if ... { } }` with `if let Some(...) && ... { }` using the `let_chains` feature. `process.rs` gained a `Default` impl and `&Path` improvement. These are style-only changes that came from running `cargo clippy -- -D warnings`.

This is fine — the commit message notes it — but future sprints could isolate clippy fixes into a separate commit for cleaner review.

## Positive Notes

- **`observer.rs` is well-isolated.** Zero coupling to async runtime, zero coupling to proto types. The engine translates between the observer's domain types (`ObserverFinding`) and proto types (`ObserverAlert`). This makes the observer independently testable and avoids the common pattern of proto types leaking into business logic.

- **`SandboxGuard.validate_paths` provides defense in depth.** Output-line scanning catches obvious escape attempts in real-time, and post-completion `git diff --name-only` validation provides a second check before merge. The two-layer approach means an agent that uses indirect file writes (e.g., via a subprocess) still gets caught.

- **Event sequencing is end-to-end.** The engine's `publish_event` increments, the transport surfaces `DATA_LOSS` on lag, `nexode-ctl watch` detects gaps and refreshes, and the snapshot carries `last_event_sequence` for client reconciliation. R-005 is resolved.

- **Test coverage is strong.** 6 new engine integration tests (loop kill, uncertainty, sandbox, event sequencing, and supporting helpers) plus 7 observer unit tests and 2 git tests. The `drive_engine_until` helper is a good pattern for deterministic async engine tests.

- **Mock harness behaviors (`[[mock-loop]]`, `[[mock-uncertain]]`, `[[mock-outside-write]]`)** are a clean extension point. Task-string-driven mock behaviors avoid the need for separate mock harness implementations per test scenario.

## Issues Resolved

| Issue | Resolution |
|-------|-----------|
| I-017 | `AgentStateChanged.slot_id` added to proto and populated in all engine event emissions |
| R-005 | Event sequence numbers + lag detection + client gap recovery fully implemented |

## Issues Still Open

| Issue | Notes |
|-------|-------|
| I-016 | Task-transition semantics divergence — not in Sprint 3 scope |
| I-018 | Telemetry double-count risk — not in Sprint 3 scope |
| I-019 | `demo.sh` polling — not in Sprint 3 scope |

## New Issues

| ID | Sev | Summary |
|----|-----|---------|
| I-020 | Low | `observe_output` creates slot state for unknown/removed slots (F-001) |
| I-021 | Low | Alert-only loop findings suppress re-alerting permanently (F-002) |
| I-022 | Low | `run_observer_tick` runs blocking git-status in async context (F-003) |
| I-023 | Low | `candidate_paths` may false-positive on URLs and source locations (F-004) |
| I-024 | Low | `LoopDetected` proto flattens three distinct observer finding kinds (F-006) |

## Verdict

**Ready to merge.**

All 5 exit criteria are met. 7 findings, all low severity, none blocking. The observer layer is well-designed, well-tested, and cleanly integrated. I-017 and R-005 are resolved. The new issues (I-020 through I-024) are all "nice-to-have" improvements that can be addressed in future sprints.

Recommendation: merge, update ISSUES.md with I-020 through I-024, and plan Sprint 4.
