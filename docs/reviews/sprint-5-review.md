# Sprint 5 Code Review: TUI Dashboard

**Branch:** `agent/gpt/sprint-5-tui-dashboard`
**Reviewer:** pc (Claude)
**Date:** 2026-03-16
**Commits reviewed:**
- `5ed9ae5` — `[gpt] feat: add sprint 5 tui dashboard`
- `bcef136` — `[pc] docs: Sprint 4 review + Sprint 5 TUI dashboard handoff and Codex prompt`

(Earlier commits `ee82552`, `eb47ad2`, `3cd2355`, `ab0a909` are Sprint 4 / docs — already reviewed and merged to `main`.)

---

## Summary

Sprint 5 delivers a well-structured ratatui + crossterm TUI dashboard as a new workspace member `nexode-tui`. The crate is a pure gRPC client of the Nexode daemon — no daemon or proto files were modified. It connects via `GetFullState` for bootstrap, `SubscribeEvents` for live updates, and `DispatchCommand` for operator actions. The architecture is clean: a `tokio::select!` loop coordinates gRPC event ingestion, keyboard input from `spawn_blocking`, a ~15 FPS render tick, and signal-based shutdown.

Code quality is high for a Sprint 5 scope. The state module correctly applies incremental events with saturating arithmetic, recovers from event gaps and `DATA_LOSS` via snapshot refresh, and maintains a bounded event log. The input module parses structured commands (`:move`, `:resume-slot`) and falls back to natural-language chat dispatch. Terminal cleanup is handled through a `Drop` guard, panic hook, and signal handler triple — covering all exit paths. Test coverage is solid with 18 unit tests spanning CLI parsing, state application, event formatting, command parsing, and key mapping.

Three findings warrant attention before merge: the status color mapping diverges from the kanban spec (D-009) in several places, event gap recovery silently drops the triggering event, and the `time` crate's `current_local_offset()` will always fail in the multi-threaded tokio runtime (silently falling back to UTC). None are blockers, but the color divergence should ideally be fixed pre-merge since it's a trivial change, while the other two are acceptable follow-ups.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| `nexode-tui` binary connects to daemon, fetches state, subscribes to events | PASS | `fetch_snapshot` + `subscribe_events` in `run()`, gap recovery in `run_grpc_receiver` |
| Dashboard renders project tree, slot detail, and event log with live updates | PASS | Three-panel layout in `ui.rs`, `apply_event` incremental updates in `state.rs` |
| Keyboard navigation works (arrow keys, enter, quit) | PASS | `input.rs` maps ↑/↓/Enter/q/Ctrl+C correctly, tested |
| At least pause/resume/kill commands dispatch to daemon and show results | PASS | `handle_action` dispatches `PauseAgent`, `ResumeAgent`/`ResumeSlot`, `KillAgent` via gRPC |
| Terminal restored on quit, Ctrl+C, and panic | PASS | `TerminalCleanup` Drop guard + `install_panic_cleanup_hook` + `shutdown_signal` |

---

## Verification Suite

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS |
| `cargo test -p nexode-tui` | PASS — 18 tests, 0 failures |
| `cargo test -p nexode-daemon` | PASS — 63 lib + 3 bin tests, 0 failures |
| `cargo test -p nexode-ctl` | PASS — 4 tests, 0 failures |
| `cargo build -p nexode-tui` | PASS |
| `cargo run -p nexode-tui -- --help` | PASS — prints usage with `--addr` option |

---

## Findings

### F-01 [Medium] Status color mapping diverges from kanban spec (D-009)

**Location:** `crates/nexode-tui/src/ui.rs:248-263` (`status_style_for_task`)

The TUI's task-status colors diverge from the canonical Kanban column colors defined in `docs/architecture/kanban-state-machine.md` § 6:

| Status | Spec (D-009) | TUI (ui.rs) | Match? |
|---|---|---|---|
| PENDING | Gray | DarkGray | ~Close |
| WORKING | **Teal** | **Green** | No |
| REVIEW | Amber | Yellow | ~Close |
| MERGE_QUEUE | Blue | Cyan | ~Close |
| RESOLVING | **Red** | **Magenta** | No |
| DONE | **Green** | **White+Dim** | No |
| PAUSED | **Gray** | **Red** | No |
| ARCHIVED | Dim | DarkGray+Dim | ~Close |

Three clear mismatches (RESOLVING, DONE, PAUSED) and one debatable choice (WORKING: Green vs Teal). The PAUSED=Red is particularly misleading — red typically signals errors or critical states, not a user-initiated pause. DONE=White+Dim loses the "success" semantic that Green conveys.

**Recommendation:** Fix RESOLVING→Red, DONE→Green+Dim, PAUSED→DarkGray pre-merge. WORKING→Cyan could approximate Teal if desired. These are all one-line changes in `status_style_for_task`. Consider filing an issue if the spec colors are intentionally different for terminal contrast.

### F-02 [Low] Event gap recovery drops the triggering event

**Location:** `crates/nexode-tui/src/main.rs:322-330`

When `run_grpc_receiver` detects an event sequence gap (line 323: `event.event_sequence != last_sequence + 1`), it fetches a fresh snapshot and `continue`s the loop — but the event that triggered the gap detection is silently discarded. If the snapshot's `last_event_sequence` is behind the dropped event, the TUI will miss that specific state change until the next event arrives.

```rust
if last_sequence != 0 && event.event_sequence != last_sequence + 1 {
    match fetch_snapshot(&addr).await {
        Ok(snapshot) => {
            last_sequence = snapshot.last_event_sequence;
            if tx.send(GrpcMessage::Snapshot(snapshot)).await.is_err() {
                break;
            }
            continue; // ← triggering event is dropped
        }
        ...
    }
}
```

**Recommendation:** After sending the snapshot, also apply the triggering event if `event.event_sequence > snapshot.last_event_sequence`. This is a minor data-loss edge case that's acceptable for Sprint 5, but should be tracked for follow-up.

### F-03 [Low] `time` crate local offset always fails under multi-threaded tokio

**Location:** `crates/nexode-tui/src/events.rs:112-117`

`UtcOffset::current_local_offset()` uses `localtime_r` under the hood, which the `time` crate refuses to call when multiple threads are running (soundness concern on some platforms). Since the TUI runs under `#[tokio::main]` (multi-threaded runtime), this will always return `Err` and fall back to UTC silently.

```rust
fn format_timestamp(timestamp_ms: u64) -> String {
    match UtcOffset::current_local_offset() {
        Ok(offset) => format_timestamp_with_offset(timestamp_ms, offset),
        Err(_) => format_timestamp_with_offset(timestamp_ms, UtcOffset::UTC),
    }
}
```

**Recommendation:** Either (a) compute the local offset once at startup before spawning the tokio runtime and pass it through `AppState`, or (b) use `chrono` with its `Local` timezone which handles multi-threaded contexts, or (c) accept UTC and label the timestamps as UTC in the event log header. Not a blocker — the fallback is safe and the timestamps display correctly in UTC.

### F-04 [Low] Status glyphs are simplified vs design intent

**Location:** `crates/nexode-tui/src/ui.rs:266-277` (`status_glyph`)

The status glyphs use `*` for active states and `-` for inactive states instead of the Unicode characters mentioned in the sprint prompt (○/◉/◎/⟳/⚠/✓/‖/✗). This is functional but reduces visual distinctiveness between slot states in the project tree.

**Recommendation:** Acceptable for Sprint 5 MVP. Consider using Unicode symbols in a future pass when terminal compatibility testing is feasible. The simplified approach avoids rendering issues on terminals without Unicode support.

### F-05 [Info] `apply_event` wildcards 4 event types without state mutation

**Location:** `crates/nexode-tui/src/state.rs:150-151`

The `apply_event` catch-all `_ => {}` silently ignores `AgentStateChanged`, `WorktreeStatusChanged`, `UncertaintyFlagTriggered`, and `ObserverAlert` payloads (beyond logging them). These events are still logged to the event log via `push_log_entry`, which runs before the match.

This is acceptable for Sprint 5: `AgentStateChanged` could plausibly update a slot's display state, but the current proto doesn't expose agent state on `AgentSlot` directly. `WorktreeStatusChanged` and `ObserverAlert` have no natural home in the current `AppState` model. Future sprints may add richer state tracking.

**Recommendation:** No action needed. Document as a known limitation if the TUI is expected to show agent lifecycle states.

### F-06 [Info] `find_slot` matches task by slot ID, not task ID

**Location:** `crates/nexode-tui/src/state.rs:273`

```rust
let task = self.task_dag.iter().find(|task| task.id == slot.id);
```

This finds a task whose ID equals the slot's ID. In the current Nexode model this is correct (slot IDs and task IDs are 1:1), but it would break if task IDs ever diverge from slot IDs. The same pattern appears in `ui.rs:96`.

**Recommendation:** No action needed for Sprint 5. The 1:1 mapping is established by the daemon's slot allocation. Worth a comment if clarity matters.

### F-07 [Info] No `--version` flag on CLI

**Location:** `crates/nexode-tui/src/main.rs:39-47`

The `Cli` struct derives `Parser` but doesn't include `#[command(version)]`. Running `nexode-tui --version` fails, though `--help` works correctly. Minor UX gap.

**Recommendation:** Add `version` to the `#[command(...)]` attribute. One-line fix.

---

## Positive Notes

1. **Clean async architecture** — The three-task `tokio::select!` loop (gRPC receiver, input handler, render tick) with channel coordination is well-structured and avoids common pitfalls like blocking the async runtime.
2. **Robust terminal cleanup** — The `TerminalCleanup` Drop guard + panic hook + signal handler triple covers all exit paths. No terminal state leaks observed.
3. **Event gap recovery** — The dual-path recovery (sequence gap detection + `DATA_LOSS` status code) with snapshot refresh is a thoughtful approach that handles real-world gRPC streaming failure modes.
4. **Good test coverage** — 18 tests across 4 modules covering CLI parsing, state application (5 event types), event formatting (4 severity cases), command parsing (4 cases), and key mapping. Tests are focused and well-named.
5. **Correct saturating arithmetic** — `saturating_add` for token increments (state.rs:128) prevents overflow panics on telemetry events.
6. **Bounded event log** — `VecDeque` with `MAX_EVENT_LOG = 100` prevents unbounded memory growth from chatty daemons.
7. **Clean workspace integration** — New crate added as workspace member with path dependency on `nexode-proto`, no version conflicts, no workspace-level dependency changes beyond the new member.
8. **No daemon or proto modifications** — Sprint 5 stayed within its approved surface area. Confirmed via `git diff ee82552..HEAD --stat` — only TUI crate and docs were touched.
9. **I-024 treatment is honest** — The `loop/stuck/budget` label in event formatting correctly acknowledges the proto flattening rather than guessing which condition triggered the alert.
10. **Command parsing has good UX** — The fallback from structured commands (`move`, `resume-slot`) to natural-language chat dispatch (slot-scoped when a slot is selected, global otherwise) is a sensible progressive disclosure pattern.

---

## Open Issue Interaction

### I-024

**ObserverAlert::LoopDetected conflates loop/stuck/budget-velocity into one proto variant.**

The TUI handles this correctly by labeling all `LoopDetected` alerts as `loop/stuck/budget` in the event log (`events.rs:66-71`). Severity mapping is also correct: `Kill`/`Pause` interventions get `Critical`, `Alert`/`UncertaintySignal` get `Warning`. This is the right approach for Sprint 5 without proto changes. When I-024 is resolved (by splitting `LoopDetected` into separate proto variants), the TUI formatting can be updated to show specific labels. No blocking impact.

### I-025

**Review→Paused creates a state that's awkward to resume via ResumeAgent/ResumeSlot.**

The TUI exposes the workaround via `:move <task-id> review` command, which allows the operator to move the task back to REVIEW status. The `r` key binding dispatches `ResumeAgent` (if agent exists) or `ResumeSlot` (if no agent), which is the correct current behavior. The TUI doesn't introduce or worsen this issue — it's purely daemon-side. Should remain a follow-up, not a blocker.

---

## Issues

### Resolved

| Issue | Resolution |
|---|---|
| (none) | |

### Open

| Issue | Notes |
|---|---|
| I-024 | TUI handles correctly with conservative `loop/stuck/budget` label. Awaits proto split. |
| I-025 | TUI exposes `:move <task-id> review` workaround. Daemon-side fix needed. |

### New

| ID | Severity | Title | Notes |
|---|---|---|---|
| I-026 | Medium | TUI status colors diverge from kanban spec (D-009) | RESOLVING=Magenta (spec: Red), DONE=White (spec: Green), PAUSED=Red (spec: Gray). See F-01. |
| I-027 | Low | Event gap recovery drops triggering event | `run_grpc_receiver` discards the event that triggered gap detection. See F-02. |
| I-028 | Low | Timestamps always UTC in multi-threaded tokio runtime | `time` crate `current_local_offset()` always fails under multi-threaded runtime. See F-03. |

---

## Verdict

**APPROVE WITH FOLLOW-UPS** — Sprint 5 delivers a functional, well-tested TUI dashboard that meets all exit criteria. The codebase is clean, the async architecture is sound, terminal cleanup is thorough, and no regressions were introduced. The three new issues (I-026, I-027, I-028) are real but none are blockers: the color divergence (I-026) is the most visible and could ideally be fixed pre-merge with a trivial 4-line change, but it's not functionally incorrect. The event gap edge case (I-027) and UTC-only timestamps (I-028) are acceptable for Sprint 5 scope. Recommend merging after optionally addressing I-026 (status colors).
