# Sprint 2 Code Review — Real Agent Integration + Critical Fixes

> **Date:** 2026-03-15
> **Reviewer:** pc
> **Branch:** `agent/gpt/sprint-2-real-agents` (commit `9fd513e`)
> **Base:** `main` (commit `e080028`)
> **Diff:** 2,735 lines across 19 files (+1,700 / -188)
> **Agent:** gpt (Codex)

---

## Scope Summary

| Deliverable | Status |
|---|---|
| I-009: `completion_detected` overrides non-zero exit | ✅ Fixed |
| I-010: `AgentStateChanged(Executing)` dropped after swap | ✅ Fixed |
| I-015: JSON substring matching in completion detection | ✅ Fixed |
| R-007: Command acknowledgment (oneshot request/response) | ✅ Implemented |
| Live integration tests (gated `--features live-test`) | ✅ Added |
| End-to-end demo script (`scripts/demo.sh`) | ✅ Added |
| Claude CLI live verification (real credentials) | ✅ Passed |
| Codex CLI live verification | ⚠️ Not yet run |

---

## Findings

### F-001: `is_valid_task_transition` diverges from Kanban State Machine spec — Severity: Medium

**File:** `engine.rs`, `is_valid_task_transition()` function

The implementation:
```rust
(Review, MergeQueue | Working | Paused) => true,
(MergeQueue, Done | Resolving | Paused) => true,
(Resolving, Done | Archived) => true,
(Paused, Working | MergeQueue) => true,
```

Compared to the exhaustive valid transitions in `docs/architecture/kanban-state-machine.md`:

| Transition | Spec | Implementation | Issue |
|---|---|---|---|
| `Review → Archived` | Not listed | Not allowed | OK — spec doesn't list it either |
| `MergeQueue → Paused` | Not listed | **Allowed** | Spec has no `MergeQueue → Paused` transition |
| `Paused → Working` | Listed (if was WORKING) | Allowed unconditionally | Missing state history check — a slot paused from REVIEW shouldn't resume to WORKING |
| `Paused → MergeQueue` | Listed (if was queued) | Allowed unconditionally | Same — missing state history check |

The `MergeQueue → Paused` transition is not in the spec's exhaustive table. This is minor for now since the daemon doesn't enforce pause during merge, but it's a divergence from the state machine.

The `Paused` transitions are more concerning: the spec says `Paused → Working` is only valid "if was WORKING" and `Paused → MergeQueue` only "if was queued." The implementation doesn't track what state the slot was in before being paused, so it allows any resume target. This won't cause immediate breakage but could allow semantically wrong transitions (e.g., pausing a REVIEW slot and resuming to WORKING, bypassing re-review).

**Action:** Track in ISSUES.md. Non-blocking for merge since the daemon currently only pauses WORKING slots.

---

### F-002: `AgentStateChanged` missing `slot_id` field — Severity: Low

**File:** `engine.rs`, I-010 fix (~line 804)

The emitted event:
```rust
AgentStateChanged {
    agent_id: swapped.new_agent_id,
    new_state: AgentState::Executing as i32,
}
```

The proto definition for `AgentStateChanged` only has `agent_id` and `new_state` — no `slot_id`. This means TUI/extension subscribers who want to update a slot's visual state after a swap event need to correlate the `agent_id` from `AgentStateChanged` with the `new_agent_id` from the preceding `SlotAgentSwapped` event. This works but is fragile — if events arrive out of order or the swap event is missed, the UI can't determine which slot changed.

This is a pre-existing proto design limitation, not introduced by this sprint. The fix itself is correct within the current proto contract.

**Action:** Track as follow-up — consider adding `slot_id` to `AgentStateChanged` proto message.

---

### F-003: `parse_json_summary_telemetry` double-counts if both completion and telemetry lines hit — Severity: Low

**File:** `harness.rs`, `parse_json_summary_telemetry()`

The function fires on lines where `type == "result"`, `event == "done"`, or `status == "completed"`. For Claude, the result line includes `total_cost_usd` and `usage` fields. The harness's `parse_telemetry` is called on every line:

```rust
fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry> {
    parse_json_summary_telemetry(line).or_else(|| parse_keyed_telemetry(line))
}
```

If Claude emits multiple result-type lines (e.g., partial results then final result), each one that matches the gate and has usage fields would produce a `ParsedTelemetry`. The engine's `apply_telemetry` increments cumulative totals, so this would double-count.

However, the test `real_harnesses_parse_json_summary_telemetry_without_counting_partial_messages` explicitly validates that `type: "assistant"` lines are filtered out (they don't match the `type == "result"` gate). In practice, Claude emits exactly one `type: "result"` line at the end. So this is low risk but worth documenting — the telemetry will be wrong if the CLI format ever emits multiple result lines.

**Action:** Track as follow-up. Consider a `telemetry_finalized` flag in the process loop.

---

### F-004: `locate_generated_file` only checks two paths — Severity: Low

**File:** `live_harness.rs`, `locate_generated_file()`

```rust
fn locate_generated_file(root: &Path) -> Option<PathBuf> {
    [root.join("hello.rs"), root.join("src/hello.rs")]
        .into_iter()
        .find(|path| path.exists())
}
```

A real Claude/Codex agent might place the file elsewhere (e.g., `src/lib.rs`, `src/main.rs`, or include `hello()` in an existing file). The task prompt says "Add a hello() function to hello.rs" which is fairly directive, but agents are non-deterministic.

This is acceptable for a smoke test — the test prompt is specific enough. If it becomes flaky, the path list can be extended.

**Action:** None needed. Acceptable for gated live tests.

---

### F-005: Demo script doesn't wait for DONE state before exiting — Severity: Low

**File:** `scripts/demo.sh`

After sending `dispatch move-task slot-a merge-queue`, the script immediately prints status and exits. It doesn't wait for the merge to complete (DONE state). The merge happens asynchronously on the daemon's tick interval (2s), so the "Final status" output may still show `merge_queue`.

This is cosmetic — the demo is informational, not a test. But it could confuse users who expect to see DONE.

**Action:** Minor — consider adding a small wait loop for DONE after the MoveTask dispatch. Non-blocking.

---

### F-006: `command_exists` uses `sh -lc` which loads dotfiles — Severity: Low

**File:** `live_harness.rs`

```rust
fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        ...
}
```

The `-l` flag loads login shell dotfiles. This mirrors the existing `git.rs` verification behavior (R-002) and is intentional — it ensures tools installed via `nvm`, `cargo`, or other shell-managed environments are found. Consistent with the rest of the codebase.

**Action:** None. Consistent with existing approach. R-002 already tracks the broader dotfile concern.

---

## Exit Criteria Assessment

### EC-1: I-009 resolved — non-zero exit always means failure ✅

`process.rs` line ~328:
```rust
success: status.success()
    && (completion_detected || !requires_completion_signal),
```

Four unit tests validate the truth table:
- `completion_marker_does_not_override_non_zero_exit` — exit 1 + marker = failure
- `zero_exit_requires_completion_signal_for_real_harnesses` — exit 0 + no marker + required = failure
- `zero_exit_with_completion_signal_is_success` — exit 0 + marker = success
- `zero_exit_without_completion_signal_is_success_for_mock_compat` — exit 0 + no marker + not required = success

The `TestHarness` struct cleanly parameterizes `requires_signal` and `completion_line`. All four scenarios are correctly tested.

### EC-2: I-010 resolved — AgentStateChanged(Executing) emitted after swap ✅

`engine.rs` now emits `AgentStateChanged { agent_id: swapped.new_agent_id, new_state: Executing }` immediately after the `SlotAgentSwapped` event.

`slot_agent_swapped_emits_executing_event` test subscribes to the event stream, injects a swap event, and asserts both `SlotAgentSwapped` and `AgentStateChanged(Executing)` are received.

### EC-3: I-015 resolved — JSON parsing, no false positives ✅

`harness.rs` now uses `json_field_is()` which does `serde_json::from_str` + field check. The `json_value_field_is` helper is clean and correct.

Test `real_harness_completion_detection_uses_json_instead_of_substring_matching` verifies:
- `"task completed successfully"` → `false` (no more false positives)
- `{"type":"result",...}` → `true` (Claude)
- `{"type" : "result"}` (with whitespace) → `true` (whitespace-tolerant)
- `{"event":"done"}` → `true` (Codex)
- `{"event":"progress"}` → `false`

### EC-4: R-007 resolved — command dispatch returns real result ✅

Full implementation across proto/transport/engine/CLI:
- Proto adds `command_id` (field 3) and `CommandOutcome` enum (field 4) — backward compatible
- Transport uses `oneshot::channel()` with configurable timeout (50ms test, 5s production)
- Engine validates slot existence, state transitions, and sends outcome through oneshot
- CLI prints formatted result with outcome label

`dispatch_command_returns_validated_outcomes` test validates:
- `Review → Done` (invalid transition) → `InvalidTransition`
- `SlotDispatch` to nonexistent slot → `SlotNotFound`
- `Review → MergeQueue` (valid) → `Executed`

`dispatch_command_times_out_when_engine_does_not_respond` test validates timeout path.

### EC-5: Live smoke test for at least one real CLI agent ✅

`live_claude_code_hello_world` and `live_full_lifecycle` both passed with real Claude credentials per the HANDOFF.md verification log. Codex verification is still pending but the test infrastructure is in place.

### EC-6: Demo script exists and runs end-to-end ✅

`scripts/demo.sh` is well-structured: auto-detects available harness, creates temp repo, writes session, starts daemon, polls for REVIEW, queues merge, prints results. Proper cleanup via trap.

**All 6 exit criteria are met.**

---

## Decision Compliance

| Decision | Compliant? | Notes |
|---|---|---|
| D-002 (FullStateSnapshot) | ✅ | No changes to snapshot structure |
| D-003 (YAML mode mapping) | ✅ | Unchanged |
| D-004 (Defaults cascade) | ✅ | Unchanged |
| D-005 (Multi-monitor) | N/A | Not in scope |
| D-006 (SlotDispatch) | ✅ | SlotDispatch validation added |
| D-007 (REVIEW belongs to TaskNode) | ✅ | Unchanged |
| D-008 (Phase 0 merge step) | ✅ | Unchanged |
| D-009 (Kanban state machine) | ⚠️ | See F-001 — `is_valid_task_transition` diverges slightly |
| D-010 (Resolving state) | ✅ | Resolving transitions correct |

---

## Test Coverage Summary

| Area | Tests Added | Coverage |
|---|---|---|
| I-009 exit semantics | 4 | Full truth table: 4 combinations of exit code × completion signal × requires_signal |
| I-010 swap event | 1 | Event stream verification for swap + executing |
| I-015 JSON completion | 1 | False positive prevention + correct JSON detection for both harnesses |
| I-015 JSON telemetry | 1 | Summary telemetry parsing from result lines, partial message filtering |
| R-007 command ack (engine) | 1 | Valid transition, invalid transition, missing slot — all outcomes |
| R-007 command ack (transport) | 2 | Oneshot round-trip + timeout |
| R-007 command ack (CLI) | 2 | Success + failure formatting |
| Live harness (gated) | 3 | ClaudeCode hello world, CodexCli hello world, full lifecycle |
| Process test infrastructure | 2 | `TestHarness` struct, `collect_until_first_exit` helper |

**Total new/modified tests:** 17

### Missing Test Coverage

1. **MoveTask to an invalid target** (e.g., passing a raw integer that doesn't map to a `TaskStatus`) — the `Ok(target)` branch handles this but no test covers it.
2. **`ChatDispatch` always returns `Executed`** — not tested, but trivially correct.
3. **`parse_json_summary_telemetry` with nested/missing usage fields** — only tested via the `real_harnesses_parse_json_summary_telemetry` test with specific formats. No test for the case where `total_cost_usd` is present but `usage` is missing.
4. **Demo script** — no automated validation. This is acceptable for a shell script.

---

## Open Questions / Assumptions

**Q1:** The `is_valid_task_transition` function is only used for command validation (operator-initiated transitions). Daemon-internal transitions (agent completion → REVIEW, merge success → DONE) bypass this function. Was this intentional? If so, the function name should clarify it's for operator commands only.

**Q2:** The `COMMAND_RESPONSE_TIMEOUT` uses `#[cfg(test)]` to set 50ms in tests vs 5s in production. This is pragmatic but means the timeout path is never truly tested under realistic conditions. Is there value in making this configurable via `DaemonConfig`?

**Q3:** The `parse_json_summary_telemetry` function probes a wide set of field paths (snake_case and camelCase variants). Was this based on observed CLI output, or preemptive? The Claude path was validated live; the Codex paths are untested.

**Q4:** `locate_generated_file` in the live tests only checks `hello.rs` and `src/hello.rs`. The task prompt is specific ("Add a hello() function to hello.rs"), but real agents can be creative. Is there a plan to make this more robust for CI?

---

## Follow-Up Items for ISSUES.md

| ID | Severity | Finding | Action |
|---|---|---|---|
| I-016 | Med | F-001: `is_valid_task_transition` allows `MergeQueue → Paused` (not in spec) and doesn't track pre-pause state for resume | Add pre-pause state tracking; align with kanban state machine |
| I-017 | Low | F-002: `AgentStateChanged` proto has no `slot_id` — UI must correlate with `SlotAgentSwapped` | Add `slot_id` field to proto |
| I-018 | Low | F-003: `parse_json_summary_telemetry` would double-count if CLI emits multiple result lines | Consider telemetry finalization flag |
| I-019 | Low | F-005: `demo.sh` doesn't wait for DONE after MoveTask | Add wait loop |

---

## Merge Recommendation

### `ready with follow-ups`

All 6 exit criteria are met. The three targeted bug fixes (I-009, I-010, I-015) are correctly implemented with thorough test coverage. The R-007 command acknowledgment is clean — the oneshot pattern, proto changes, and engine validation all follow the architecture doc faithfully. The Claude live verification passed end-to-end.

**Blocking before merge:** Nothing.

**Key residual risk:** Codex CLI live testing hasn't been run. The harness code for `CodexCliHarness` mirrors the Claude pattern and should work, but the CLI flags (`codex exec --full-auto --json`) and JSON output format are unverified against a real Codex process. This is the same risk category as Sprint 1's untested harnesses — this sprint reduced it from "none tested" to "one of two tested." Recommend running Codex verification before Sprint 3 begins.

**Code quality notes:**
- Heavy `cargo fmt` reformatting throughout (accounting, context, engine, git, wal, recovery) — all cosmetic, no behavioral changes. Makes the diff larger than the actual feature work.
- New test infrastructure (`TestHarness`, `collect_until_first_exit`, process fixture extensions) is well-designed and reusable.
- The `json_value_at_path` / `json_u64_at_paths` / `json_f64_at_paths` helper layer is clean and extensible for future CLI format changes.
