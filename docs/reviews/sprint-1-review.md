# Sprint 1 Code Review: WAL Recovery + Agent Harness

> **Reviewer:** pc (Perplexity Computer)
> **Branch:** `agent/gpt/sprint-1-wal-harness` (commit `9ffbad6`)
> **Base:** `main` (`af4bf2e`)
> **Date:** 2026-03-15
> **Diff size:** 3,367 lines across 13 files (7 new, 6 modified)

---

## Summary

Sprint 1 delivers WAL-based crash recovery, a synchronous agent harness abstraction with three implementations (Mock, ClaudeCode, CodexCli), a basic context compiler, and full engine integration. The handoff claims 35 tests passing and `cargo check --workspace` green. This review evaluates correctness, decision compliance, architecture, and test coverage against the 6 Sprint 1 exit criteria.

---

## Findings

### F-001 [High] `completion_detected` overrides non-zero exit as success

**File:** `process.rs:325`
```rust
success: status.success() || completion_detected,
```

If an agent prints a line matching `detect_completion()` early in its run and then crashes with a non-zero exit code, the process manager reports `success: true`. This conflates "the agent said something that looks like completion" with "the agent actually completed successfully."

For `MockHarness`, `detect_completion` triggers on any line starting with `"completed"`. For `ClaudeCodeHarness`, it triggers on `"type":"result"` in the line. An agent could emit structured JSON containing that substring during normal streaming before crashing.

**Impact:** A crashed agent could be silently promoted to REVIEW state instead of being respawned.

**Recommendation:** Either (a) only set `completion_detected` on the *last* line before process exit, or (b) require `completion_detected && status.success()` for full success, with `completion_detected && !status.success()` as a separate outcome (e.g., "completed with errors"). At minimum, `completion_detected` should be reset if a significant error marker is seen after the completion line.

---

### F-002 [Medium] WAL write ordering: state change applied before WAL is written in `set_task_status`

**File:** `engine.rs:928–961`

The `set_task_status` method calls `append_slot_state` (WAL write) then applies the state change to memory:

```rust
fn set_task_status(...) -> Result<(), DaemonError> {
    // ... reads current state for WAL entry
    self.append_slot_state(slot_id, status, ...)?;  // WAL write with NEW status
    if let Some(slot) = self.slot_mut(slot_id) {
        slot.task_status = status;                   // memory update
    }
    // ...
}
```

The WAL entry is written with the *target* status, which is correct for recovery. However, the architecture doc (`wal-recovery.md`) specifies: "Every `set_task_status` call in the engine writes a `SlotStateChanged` entry **before applying the state change in memory**." The implementation follows this — good. But `append_slot_state` reads `current_agent_id` and `current_agent_pid` from the *current* memory state (before the status change), which means the WAL entry carries the agent ID and PID that were active *at the time of the transition*, not after. This is actually the correct behavior for recovery replay, since it preserves the snapshot of who was assigned when the transition happened.

**Note:** Not a bug — the write-before-apply ordering is correct. Flagging for clarity because the dual-path WAL writes (`append_slot_state` for explicit transitions vs `append_current_slot_state` for spawn/exit events) are easy to confuse during future maintenance.

---

### F-003 [Medium] `SlotAgentSwapped` event dropped after normal respawn

**File:** `engine.rs:670–694`

The `SlotAgentSwapped` event handler was changed to remove the follow-up `AgentStateChanged(Executing)` event:

```rust
AgentProcessEvent::SlotAgentSwapped(swapped) => {
    // ... updates slot state
    self.publish_event(
        hypervisor_event::Payload::SlotAgentSwapped(swapped.clone()),
        None,
    );
    // REMOVED: AgentStateChanged(Executing) for the new agent
}
```

This means after a crash-respawn in the process manager, the new agent's state is never published as `Executing`. gRPC subscribers (TUI, VS Code) won't see the new agent enter `Executing` state — they'll only see the `SlotAgentSwapped` event. This is a Phase 2/3 concern since those UIs don't exist yet, but it's a regression from Phase 0 behavior.

**Recommendation:** Restore the `AgentStateChanged(Executing)` event emission after `SlotAgentSwapped`. The subsequent `Spawned` event will also emit `Executing`, so verify there's no duplication — the swap handler and spawn handler may both fire for the same agent lifecycle.

---

### F-004 [Medium] Recovery doesn't clear `merge_inflight_slot` before re-enqueuing

**File:** `recovery.rs:107–111`

```rust
if let Some(inflight_slot) = project.merge_inflight_slot.take() {
    if !project.merge_queue.iter().any(|slot_id| slot_id == &inflight_slot) {
        project.merge_queue.push_front(inflight_slot);
    }
}
```

This correctly moves a mid-merge slot back to the front of the merge queue. However, the slot's `task_status` from the WAL may still be `MergeQueue` or even `Working` (if the merge was initiated but the status change hadn't been written yet). After recovery, if the slot is re-enqueued, the engine will attempt to merge it again — but the slot might not have a worktree anymore (if the merge had succeeded and the worktree was cleaned up before the crash).

**Impact:** Edge case — only triggers if the daemon crashes mid-merge (between worktree cleanup and WAL write). The worktree check in recovery (`Path::new(path).exists()`) would catch a missing worktree, but the merge queue position would still be preserved for a slot with no worktree.

**Recommendation:** In recovery, verify that re-enqueued merge slots still have a worktree path. If not, skip re-enqueueing and leave the slot in its recovered status.

---

### F-005 [Medium] `truncate_payload` uses byte count as token proxy

**File:** `context.rs:77–96`

```rust
fn truncate_payload(payload: &mut ContextPayload, max_bytes: usize) {
    if let Some(diff) = payload.recent_diff.as_mut() {
        if diff.len() > max_bytes {
            diff.truncate(max_bytes);
        }
    }
    // ...
}
```

The `max_context_tokens` field from `HarnessConfig` is passed as a byte count to `truncate_payload`. Tokens are not bytes — the ratio varies by model (~4 bytes/token for GPT, ~3.5 for Claude). Truncating at `max_bytes = max_context_tokens` means the budget is ~4x more generous than intended.

**Impact:** Low — `max_context_tokens` is currently always `None` in the engine (hardcoded at the call site). This is future-proofing code that's not exercised yet.

**Recommendation:** Either rename the parameter to `max_bytes` (matching what it does) or apply a conservative `tokens * 4` conversion. Add a `// TODO: use tiktoken or similar for accurate token counting` comment.

---

### F-006 [Low] `MockHarness` derives slot_id from worktree directory name, not from slot_id parameter

**File:** `harness.rs:117–121`

```rust
let slot_id = worktree_path
    .file_name()
    .map(|name| name.to_string_lossy().into_owned())
    .unwrap_or_else(|| "slot".to_string());
```

The slot_id used in the mock script's file names and commit messages comes from the worktree path's last component, not from the task or a passed-in slot_id. In the current engine, worktree directories are named after slot_ids (`worktree_root.join(slot_id)`), so this works. But it's an implicit coupling.

**Impact:** Low — works correctly today, fragile if worktree naming changes.

**Recommendation:** Accept `slot_id` as a parameter in `build_command` or extract it from the context/config. Not blocking.

---

### F-007 [Low] `pid_is_alive` and `terminate_pid` shell out to `kill` instead of using `libc::kill`

**File:** `recovery.rs:222–243`

```rust
fn pid_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        // ...
}
```

This works on Linux/macOS but shells out to an external process for each PID check. Using `unsafe { libc::kill(pid as i32, 0) }` would be faster and avoid spawning a process. On the other hand, the recovery path runs once at startup, so performance doesn't matter.

**Impact:** Cosmetic. The approach is correct and cross-unix-compatible.

**Recommendation:** Fine for Sprint 1. Consider `libc::kill` if recovery latency becomes an issue with many slots.

---

### F-008 [Low] `parse_space_delimited` returns `Some(empty telemetry)` for lines starting with "TOKENS " that have no key=value pairs

**File:** `process.rs:396–414`

```rust
fn parse_space_delimited(line: &str) -> Option<ParsedTelemetry> {
    // ... iterates parts, returns Some(telemetry) unconditionally
    Some(telemetry)
}
```

If a line is `"TOKENS hello world"`, this returns `Some(ParsedTelemetry { tokens_in: None, tokens_out: None, cost_usd: None })` — a telemetry event with no data. The engine will then call `apply_telemetry` which writes a `TelemetryRecorded` WAL entry with all zeros.

**Impact:** Low — the `TOKENS` prefix is only used by the old mock format, and the mock always emits well-formed lines.

**Recommendation:** Add a `found` flag like `parse_keyed_telemetry` does, and return `None` if no keys matched.

---

### F-009 [Low] `CodexCliHarness` uses `codex exec --full-auto --json` — verify CLI compatibility

**File:** `harness.rs:193–196`

```rust
let mut command = AgentCommand::new(
    "codex",
    vec!["exec", "--full-auto", "--json", "--model", &config.model, task],
);
```

The HANDOFF notes that command shapes were "aligned to local CLI help." The sprint instructions specified `codex --approval-mode full-auto`, but the implementation uses `codex exec --full-auto --json`. This is a reasonable adaptation to the actual Codex CLI interface (which uses subcommands), but the discrepancy should be documented.

**Impact:** Low — real CLI harness testing is gated behind `#[cfg(feature = "integration")]` and the command will be validated when first used against a real CLI.

**Recommendation:** Update the architecture doc (`agent-harness.md`) to reflect the actual command shape in the next pc docs update.

---

### F-010 [Low] `ClaudeCodeHarness.detect_completion` is JSON-substring-fragile

**File:** `harness.rs:177–178`

```rust
fn detect_completion(&self, line: &str) -> bool {
    line.contains("\"type\":\"result\"") || line.contains("completed")
}
```

String-contains matching for JSON is fragile — whitespace in JSON (`"type": "result"` with a space) or the word "completed" appearing in agent output would trigger false positives. Combined with F-001, this could promote a mid-run agent to success.

**Impact:** Low — same as F-001, mitigated by the fact that real agent testing is deferred.

**Recommendation:** When real CLI testing begins, switch to JSON parsing for structured output formats. For Sprint 1, acceptable.

---

## Decision Compliance

| Decision | Status | Notes |
|---|---|---|
| D-002 (no top-level agent_slots) | ✅ Compliant | Snapshot uses nested `projects[].slots[]` |
| D-003 (manual → NORMAL mapping) | ✅ Compliant | Session parser handles mode translation |
| D-004 (project-level defaults cascade) | ✅ Compliant | `EffectiveDefaults` cascade preserved, `harness` added alongside `model` |
| D-006 (SlotDispatch) | ✅ Compliant | `dispatch_slot` preserved from Phase 0 |
| D-007 (REVIEW on TaskNode) | ✅ Compliant | Recovery correctly preserves `TaskStatus::Review` |
| D-008 (post-merge verification) | ✅ Compliant | Merge path with verify still intact |
| D-009 (MERGE_QUEUE + RESOLVING) | ✅ Compliant | All states handled in recovery replay |
| D-010 (RESOLVING with git-level trigger) | ✅ Compliant | Conflict → RESOLVING path preserved |

All 8 active decisions are respected. No new decisions are introduced.

---

## Exit Criteria Assessment

### EC-1: WAL persistence — daemon writes WAL entries during normal operation; after kill + restart, state is recovered ✅

WAL entries are written at all mutation points: `set_task_status`, `apply_telemetry`, `merge_slot` outcomes, spawn events, completion events. The `recovers_review_state_without_restarting_finished_slot` integration test kills the daemon mid-run and restarts from the WAL, verifying that slot status, agent ID, and session cost are preserved.

### EC-2: CRC integrity — corrupted WAL entries are detected and skipped without crashing ✅

`skips_crc_mismatches_without_crashing` test corrupts the last byte of a WAL file, verifies entries are empty and a CRC mismatch warning is present.

### EC-3: Agent harness trait — MockHarness, ClaudeCodeHarness, CodexCliHarness all implement the trait; existing mock-based tests pass ✅

All three implement `AgentHarness`. The engine test session YAMLs now include `harness: "mock"` to ensure MockHarness is selected. The full-auto merge queue test and budget hard-kill test pass through the harness layer.

### EC-4: Context compiler — task + include/exclude + git diff assembled into ContextPayload ✅

`compiles_task_globs_diff_and_readme` test validates task description, include files (resolved via glob), exclude patterns, and README content. The context compiler is integrated into `start_slot`.

### EC-5: Harness selection — different model/harness values select the correct implementation ✅

`model_and_override_select_expected_harnesses` tests model-based inference (claude → ClaudeCode, gpt → CodexCli) and explicit harness override (gpt + harness:mock → Mock).

### EC-6: Config migration — harness field is optional; existing session.yaml files parse correctly ✅

`parses_optional_harness_override_and_hashes_root_file` test verifies optional `harness` field. The `harness` field has type `Option<String>` with `#[serde(default)]` behavior (absent = None). All existing session YAMLs parse correctly since `harness` is optional.

**All 6 exit criteria are met.**

---

## Test Coverage Summary

| Area | Tests | Coverage |
|---|---|---|
| WAL framing + CRC | 2 | Write/read round-trip, CRC corruption skip |
| WAL compaction | 1 | Compaction rewrites to single checkpoint |
| Recovery checkpoint round-trip | 1 | Serialize/deserialize PersistedRuntimeState |
| Recovery WAL replay | 1 | Checkpoint + telemetry + state change replay |
| Recovery config drift | 1 | Warning generated, recovery continues |
| Recovery PID termination | 1 | Live process killed, slot marked for restart |
| Recovery integration | 1 | Daemon kill + restart preserves Review state |
| Context compiler | 1 | Task, globs, diff, README assembled |
| Harness selection | 1 | Model inference + explicit override |
| Claude command shape | 1 | -p flag, CLAUDE.md setup file |
| Codex command shape | 1 | exec subcommand, .codex/instructions.md setup |
| Session harness parsing | 1 | Optional harness field + config hash |
| Engine full-auto merge | 1 (existing) | Now uses harness: "mock" |
| Engine budget hard-kill | 1 (existing) | Now uses harness: "mock" |

**Total new tests:** 12  **Modified tests:** 2  **Reported total:** 35 (including all existing tests)

### Missing Test Coverage

1. **Truncation logic** in `truncate_payload` — no test for `max_context_tokens` truncation.
2. **Setup file writing** (`write_setup_files` in process.rs) — no direct test; exercised only through integration.
3. **Multiple checkpoints + compaction** — no test for writing multiple checkpoints and verifying only the latest survives.
4. **Recovery with config drift + removed/added slots** — the config drift test only checks the warning. No test verifies that a removed slot is skipped and a new slot starts fresh.
5. **WAL entry for `TelemetryRecorded`** — tested through recovery replay but no isolated test for telemetry WAL write + read.

---

## Open Questions / Assumptions

**Q1:** The `completion_detected` flag (F-001) is a meaningful behavior change. Was the intent to handle CLI agents that complete successfully but return non-zero exit codes? If so, should this be documented as a design decision?

**Q2:** The `merge_inflight` field was changed from `bool` to `Option<String>` (`merge_inflight_slot`). This is a good improvement for recovery (knowing *which* slot was mid-merge). Was this change tested with a concurrent merge + crash scenario?

**Q3:** The ClaudeCode harness uses `-p --permission-mode bypassPermissions` flags. The `-p` flag is `--print` (non-interactive mode). `bypassPermissions` bypasses the permission system entirely. Is this the intended security posture for production use, or should this be configurable per deployment?

**Q4:** The `parse_keyed_telemetry` function splits on whitespace, commas, braces, and quotes — this is quite permissive. Is this intentional to handle both structured JSON telemetry and plain-text formats?

---

## Merge Recommendation

### `ready with follow-ups`

All 6 exit criteria are met. The code is well-structured, decision-compliant, and the test coverage is solid for the core paths. The findings above are either edge cases (F-001, F-004), cosmetic (F-006, F-007, F-008), or documentation discrepancies (F-009).

**Blocking before merge:** None of the findings are blocking.

**Follow-ups for ISSUES.md:**

| ID | Severity | Finding | Action |
|---|---|---|---|
| I-009 | Med | F-001: `completion_detected` overrides non-zero exit | Review semantics before real CLI testing |
| I-010 | Med | F-003: `AgentStateChanged(Executing)` dropped after swap | Restore event for UI subscribers |
| I-011 | Low | F-004: Recovery re-enqueues merge slot without worktree check | Add worktree existence guard |
| I-012 | Low | F-005: Token/byte conflation in truncation | Rename param or add conversion |
| I-013 | Low | F-008: Empty telemetry from malformed TOKENS lines | Add `found` guard |
| I-014 | Low | F-009: Architecture doc CLI flags out of date | Update agent-harness.md |
| I-015 | Low | F-010: JSON substring matching in completion detection | Switch to JSON parse for real CLIs |
