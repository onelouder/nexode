# Sprint 6 Code Review: Integration Polish

**Branch:** `agent/gpt/sprint-6-integration-polish`
**Reviewer:** pc (Claude)
**Date:** 2026-03-16
**Commits reviewed:**
- `0d130cb [gpt] feat: complete sprint 6 integration polish`
- `51d0acd [pc] docs: Sprint 5 review + Sprint 6 integration polish handoff and Codex prompt`

---

## Summary

Sprint 6 is a focused polish sprint that resolves four low-severity issues (I-007, I-025, I-027, I-028) identified during the Sprint 5 review, adds the project's first cross-crate integration test (daemon→TUI via gRPC), and cleans up CLI surfaces and documentation. No new features were introduced — all changes are fixes, hardening, and test coverage.

The code quality is high throughout. The gap recovery fix (I-027) correctly replays the triggering event after snapshot refresh using a clean predicate function. The timezone fix (I-028) properly captures `UtcOffset::current_local_offset()` in a synchronous `main()` before the tokio runtime spawns threads, and threads the offset through `AppState` to all formatting call sites with a "UTC" label fallback. The Review resume fix (I-025) is a minimal, correct one-line addition. The immediate merge drain (I-007) leverages the single-threaded async design to avoid re-entrancy risks entirely.

The integration test (`tui_app_state_tracks_daemon_events_via_grpc`) is the most ambitious addition — it spins up a real daemon engine with gRPC server, connects TUI gRPC clients, exercises the full snapshot→event→command pipeline, and tears down cleanly. The explicit `drop(stream); drop(stream_client); drop(command_client); drop(snapshot_client)` pattern before `shutdown_tx.send(())` is a pragmatic workaround for tonic holding connections alive, but is stable for single-run CI environments.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| I-027 fixed: gap recovery applies triggering event | PASS | `should_apply_event_after_gap()` predicate at `main.rs:~line 330`; tested by `gap_recovery_replays_triggering_event` and `gap_recovery_skips_already_covered_event` |
| I-028 fixed: timezone offset computed at startup | PASS | Sync `main()` calls `current_local_offset()` before `tokio_main()`; offset threaded through `AppState::with_local_offset()`; UTC fallback with label in `event_log_title()` |
| I-025 fixed: `resume_target()` returns `Some(Review)` | PASS | `commands.rs` line ~270: `Some(TaskStatus::Review) => Some(TaskStatus::Review)`; tested by `review_pause_can_resume_back_to_review` |
| I-007 fixed: immediate merge drain on enqueue | PASS | `commands.rs` lines 201-203: `drain_merge_queues()` called immediately after `enqueue_merge()`; tested by `move_task_to_merge_queue_drains_immediately` |
| Integration test passes | PASS | `tui_app_state_tracks_daemon_events_via_grpc` in `tests.rs` lines 676-784 |
| CLI cleanup: `--version` works on both binaries | PASS | Both `nexode-tui --version` and `nexode-daemon --version` output correct version strings |
| No regressions: all existing tests pass | PASS | Full `cargo test --workspace` passes |

---

## Verification Suite

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS — no formatting issues |
| `cargo check --workspace` | PASS — no errors |
| `cargo clippy --workspace -- -D warnings` | PASS — no warnings |
| `cargo test --workspace` | PASS — 97 tests total |
| `cargo build -p nexode-tui` | PASS |
| `cargo build -p nexode-daemon` | PASS |
| `cargo run -p nexode-tui -- --version` | PASS — outputs version |
| `cargo run -p nexode-daemon -- --version` | PASS — outputs version |

**Test counts:**

| Crate | Sprint 5 | Sprint 6 | Delta |
|---|---|---|---|
| nexode-daemon (lib) | 63 | 67 | +4 |
| nexode-daemon (bin) | 3 | 3 | 0 |
| nexode-ctl | 4 | 4 | 0 |
| nexode-tui (lib) | — | 17 | new target |
| nexode-tui (bin) | 18 | 6 | -12 (moved to lib) |
| **Total** | **88** | **97** | **+9** |

**Test count investigation (18→17 lib):** Sprint 6 introduced `lib.rs`, splitting the TUI crate into library + binary targets. The original 18 tests were split: 15 moved to the lib target (under `events`, `state`, `input` modules) and 3 stayed in the binary target (CLI parsing + command formatting tests in `main.rs`). Sprint 6 added 2 new lib tests (`event_log_title_labels_utc_when_using_utc_offset`, `formats_timestamp_with_supplied_offset`) bringing the lib count to 17, plus 3 new bin tests (gap recovery replay/skip + version flag), bringing the bin count to 6. **No tests were removed.** The total increased from 88 to 97.

---

## Findings

### F-01 [Low] Claude harness doc omits `--permission-mode bypassPermissions` flags
**Location:** `docs/architecture/agent-harness.md:87`

The architecture doc was updated (I-014) to document Claude flags as `claude --print --verbose --output-format stream-json`. However, the actual code in `crates/nexode-daemon/src/harness.rs` (lines 168-177) also passes `--permission-mode bypassPermissions` — two flags that are absent from the documentation. Additionally, the code uses the short form `-p` rather than `--print`.

The Codex flags are correctly documented and match the code.

**Recommendation:** Update the architecture doc to include the full Claude flag set, or add a note that `--permission-mode` is a deployment-time configuration. This is a documentation accuracy issue, not a code issue.

### F-02 [Info] Integration test uses explicit client drop as shutdown workaround
**Location:** `crates/nexode-daemon/src/engine/tests.rs:765-768`

```rust
drop(stream);
drop(stream_client);
drop(command_client);
drop(snapshot_client);
```

This pattern is necessary because tonic gRPC connections keep the server alive during shutdown. The explicit drops before `shutdown_tx.send(())` are a pragmatic workaround rather than a fundamental fix. The pattern is deterministic and stable for single-run CI — it will not be flaky. However, if future tests need to test reconnection or concurrent client scenarios, this pattern should be abstracted into a helper.

**Recommendation:** Acceptable as-is. Consider adding a comment explaining *why* the drops are needed (tonic connection keepalive) for future maintainers.

### F-03 [Info] `drive_engine_until` timeout increased from 3s to 5s
**Location:** `crates/nexode-daemon/src/engine/test_support.rs`

The integration test exercises a more complex pipeline (gRPC + engine + TUI state) than unit tests, so the timeout increase is appropriate. Worth noting for CI performance budgeting.

**Recommendation:** No action needed.

### F-04 [Info] `find_slot` matches task by slot ID, not task ID
**Location:** `crates/nexode-tui/src/state.rs:288`

Carried forward from Sprint 5 (F-06). `find_slot()` looks up tasks by `slot.id` matching `task.id`. This works because of the current 1:1 slot↔task mapping but would break if IDs diverge in the future.

**Recommendation:** No action for Sprint 6. Track as a known assumption.

---

## Positive Notes

1. **Clean gap recovery design** — The `should_apply_event_after_gap()` predicate is a pure function extracted from the streaming loop, making it independently testable. The two tests cover both the replay and skip cases clearly.

2. **Correct timezone threading** — The offset capture in sync `main()` before `tokio_main()` is exactly the right pattern for the `time` crate's thread-safety requirement. The `UtcOffset::UTC` fallback with "Event Log (UTC)" labeling is user-friendly and transparent.

3. **Minimal, surgical fixes** — Each issue fix touches only the code it needs to. `resume_target()` got one new match arm. `move_task()` got one drain call. No unnecessary refactoring or scope creep.

4. **Strong test coverage** — 9 new tests total. The integration test exercises a real gRPC pipeline end-to-end. The Review pause/resume test covers the full lifecycle. The merge drain test verifies immediate completion without tick.

5. **Single-threaded async eliminates race conditions** — The engine's single-threaded design means `drain_merge_queues()` called from `move_task()` cannot re-enter or deadlock. The immediate drain is safe by design, not by locking.

6. **No proto changes** — Sprint 6 introduces no protobuf schema changes, keeping the wire format stable.

7. **Scope containment** — Daemon changes are entirely within `engine/`. TUI changes are limited to the four source files (`main.rs`, `events.rs`, `state.rs`, `ui.rs`) and `lib.rs` addition. No modifications to `nexode-proto` or `nexode-ctl`.

8. **Good lib/bin split** — Extracting `lib.rs` enables the cross-crate integration test (daemon dev-depends on `nexode-tui`) and is a clean architectural improvement.

9. **Comprehensive documentation updates** — HANDOFF.md, PLAN_NOW.md, CHANGELOG.md, ISSUES.md, and ROADMAP.md are all updated consistently and accurately reflect the sprint work.

---

## Issues

### Resolved

| Issue | Resolution |
|---|---|
| I-007 Immediate merge drain | `drain_merge_queues()` called immediately after `enqueue_merge()` in `move_task()` |
| I-014 Architecture doc CLI flags | Updated `agent-harness.md` Claude and Codex flag documentation |
| I-025 Review→Paused resume | Added `Some(Review) => Some(Review)` to `resume_target()` match |
| I-027 Gap recovery drops event | `should_apply_event_after_gap()` conditionally replays triggering event post-snapshot |
| I-028 TUI timestamps always UTC | Offset captured in sync `main()`, threaded through `AppState`, UTC labeled in title |

### Open

| Issue | Notes |
|---|---|
| I-004 `provider_config` shallow merge | Deferred, low severity |
| I-005 SQLite schema migration versioning | Deferred, low severity |
| I-011 Recovery re-enqueue without worktree check | Low severity |
| I-012 Token/byte conflation in `truncate_payload` | Low severity |
| I-013 Empty telemetry from malformed TOKENS lines | Low severity |
| I-018, I-019, I-024 | Spec-alignment gaps, deferred |

### New

**I-029 [Low] Architecture doc omits Claude `--permission-mode` flags**
`docs/architecture/agent-harness.md` documents Claude CLI invocation as `claude --print --verbose --output-format stream-json` but the actual harness code also passes `--permission-mode bypassPermissions`. The doc should either include these flags or note that permission mode is a deployment-time configuration detail. Non-blocking.

---

## Verdict

**APPROVE** — Sprint 6 delivers exactly what was planned: four clean issue fixes, a solid cross-crate integration test, and CLI/doc polish. All exit criteria pass. The verification suite is clean. Code changes are minimal, surgical, and well-tested. The one new finding (I-029, incomplete Claude flag documentation) is low-severity and non-blocking. The gap recovery logic is correct for all edge cases. The timezone threading follows the `time` crate's documented requirements. The immediate merge drain is safe by design in the single-threaded async engine. The integration test is stable. No regressions. Ready to merge.
