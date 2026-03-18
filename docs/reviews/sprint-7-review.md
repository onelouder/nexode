# Sprint 7 Code Review: TUI Command Hardening

**Branch:** `agent/gpt/sprint-7-tui-command-hardening`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-17
**Commit reviewed:**
- `61fa31e [gpt] handoff: complete sprint 7 -> pc review`

---

## Summary

Sprint 7 makes the TUI production-ready by adding reconnection resilience, command UX improvements, a help overlay, and two issue fixes (I-019, I-024 partial). The implementation is clean, well-structured, and follows the sprint prompt closely. All four parts are delivered as specified. The reconnection architecture is sound — the `run_grpc_receiver` task owns the reconnect loop and communicates state transitions through `GrpcMessage` variants, keeping the main `tokio::select!` loop clean.

The most significant architectural change is the shift from a persistent `TuiClient` passed through `handle_action` to per-dispatch `connect_client` calls. This simplifies the reconnect model (no stale client to invalidate) at the cost of one TCP connect per command. Acceptable for an operator TUI where commands are infrequent.

Code quality is consistent with prior sprints. The `input.rs` tab-completion uses a clean `slot_id_completion_target` → `longest_common_prefix` pipeline. The `state.rs` command history uses standard shell semantics correctly. Tests are focused and cover the right edge cases.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| Auto-reconnect with backoff and status indicator | PASS | `reconnect_event_stream()` at `main.rs:457-496`; exponential backoff 1s→30s; header indicator via `connection_indicator()` in `ui.rs:293-311` |
| Command history ↑/↓ cycle, capped at 50 | PASS | `show_previous_command`/`show_next_command` in `state.rs:291-325`; cap at `MAX_COMMAND_HISTORY=50` via `remove(0)` at line 285 |
| Status bar feedback with 5-second auto-clear | PASS | `StatusMessage` struct with `expires_at`; `clear_expired_status()` called every render tick; `STATUS_MESSAGE_TTL = 5s` |
| Tab-complete for slot IDs in `:move`/`:resume-slot` | PASS | `complete_slot_id_command` in `input.rs:182-202`; single match adds trailing space; multiple matches use longest common prefix |
| `?` toggles help overlay | PASS | `render_help_overlay` in `ui.rs:323-361`; `Clear` + centered `Paragraph`; key filtering in `map_help_key_event` |
| `scripts/demo.sh` waits for DONE | PASS | Poll loop at lines 104-113; `grep -Eq` with proper regex for `status done` |
| LoopDetected labels distinguish variants | PASS | `loop_detected_label()` in `events.rs:195-205`; three keyword categories with fallback to raw reason |
| No regressions | PASS (per agent handoff) | Agent reports full `cargo test --workspace` pass with one pre-existing flaky daemon test |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `cargo fmt --all` | PASS |
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `cargo build -p nexode-tui` | PASS |
| `cargo run -p nexode-tui -- --help` | PASS |

**Test counts:**

| Crate | Sprint 6 | Sprint 7 | Delta |
|---|---|---|---|
| nexode-daemon (lib) | 67 | 67 | 0 |
| nexode-daemon (bin) | 3 | 3 | 0 |
| nexode-ctl | 4 | 4 | 0 |
| nexode-tui (lib) | 17 | 28 | +11 |
| nexode-tui (bin) | 6 | 6 | 0 |
| **Total** | **97** | **108** | **+11** |

---

## Findings

### F-01 [Low] `command_history.remove(0)` is O(n) — consider `VecDeque`

**Location:** `state.rs:285`

```rust
if self.command_history.len() == MAX_COMMAND_HISTORY {
    self.command_history.remove(0);
}
```

`Vec::remove(0)` shifts all elements left. With `MAX_COMMAND_HISTORY = 50`, the cost is negligible in practice, but `VecDeque` would make this O(1) and is a more natural data structure for a capped history buffer. The `event_log` already uses `VecDeque` for the same pattern.

**Recommendation:** Low priority. The current code is correct. Consider migrating to `VecDeque` in a future cleanup if it bothers you.

### F-02 [Low] Help overlay key filtering diverges from design guidance

**Location:** `input.rs:83-88`

The sprint prompt's Part 3 spec says "only `?` and `q`/`Ctrl+C` should be active" while help is visible. The design guidance section says "dismissable with any key (not just `?`), similar to `less` or `man`." The implementation follows the spec (only `?`/`q`/`Ctrl+C`), which is the stricter reading.

**Recommendation:** No action needed — the spec takes precedence. The current behavior is intentional and prevents accidental command dispatch from the help screen.

### F-03 [Info] Per-dispatch `connect_client` replaces persistent client

**Location:** `main.rs:312-359`

Sprint 6 passed `&mut TuiClient` through `handle_action` → `dispatch_command`. Sprint 7 replaces this with per-call `connect_client(addr)` inside `dispatch_command`. This is the right architectural choice for the reconnect model — a persistent client would go stale on disconnect and need explicit invalidation. The per-call pattern is simple and correct. The cost (one TCP handshake per command) is invisible at human interaction speeds.

**Recommendation:** No action needed. Good design tradeoff.

### F-04 [Info] `reconnect_event_stream` loop has no upper bound on attempts

**Location:** `main.rs:468-495`

The reconnect loop retries indefinitely with exponential backoff capped at 30s. If the daemon is permanently offline (decommissioned, wrong address), the TUI will retry forever. This is arguably correct behavior for a monitoring dashboard — the operator can always press `q` to quit. However, a configurable max-retry or a status bar message showing total elapsed disconnect time would improve operator awareness.

**Recommendation:** Not blocking. Consider adding elapsed disconnect time to the header indicator in a future sprint (e.g., "Disconnected 5m42s (retry #23)").

### F-05 [Info] `GrpcMessage::Fatal` variant removed

**Location:** `main.rs:50-56`

Sprint 6 had `GrpcMessage::Fatal(String)` which caused the TUI to exit with an error. Sprint 7 replaces it with `Disconnected` + `Reconnecting`, meaning no gRPC error is fatal anymore — the TUI always attempts reconnection. This is the correct production behavior. The only remaining exit paths are operator quit, channel closure, and OS signals.

**Recommendation:** No action needed. Good simplification.

### F-06 [Info] `find_slot` still matches task by slot ID (carried forward)

**Location:** `state.rs:418`

Carried forward from Sprint 5 (F-06) and Sprint 6 (F-04). `find_slot()` looks up tasks by `slot.id == task.id`. Still works due to 1:1 slot↔task mapping. No Sprint 7 changes affect this assumption.

**Recommendation:** Track as known assumption. Will need addressing if the data model evolves.

---

## Positive Notes

1. **Clean reconnect architecture** — The `run_grpc_receiver` task owns the full reconnect lifecycle. The main loop only reacts to `GrpcMessage` variants. No connection state leaks into the action handler. This separation will make it easy to add features like configurable retry limits later.

2. **Reconnect reuses gap recovery** — When reconnecting, `reconnect_event_stream` fetches a fresh snapshot and sends it as `GrpcMessage::Snapshot`. The main loop's existing snapshot handler detects `was_disconnected` and emits the proper `[RECONNECTED]` log entry and status message. No code duplication.

3. **Status bar unification** — The footer rendering cleanly falls through: command mode → status message → default keybinding hints. The `StatusMessage` struct with `expires_at: Instant` is cleaner than a separate timer. The `clear_expired_status()` call in the render tick is the right place.

4. **Shell-like history semantics** — The `show_previous_command`/`show_next_command` pair correctly implements shell behavior: `↑` from no-index goes to most recent, `↑` at index 0 stays at 0, `↓` past the end clears input and resets index. The test at `state.rs:700-728` covers the full cycle.

5. **Tab-completion is well-factored** — `slot_id_completion_target` extracts the command prefix and partial argument. `longest_common_prefix` is a generic utility. `complete_slot_id_command` combines them. Each piece is independently testable. The trailing space on single-match is a nice UX touch.

6. **LoopDetected parsing is appropriately conservative** — The `loop_detected_label` function returns `Option<&str>`, falling back to the raw reason string when no keyword matches. This prevents information loss while improving readability for the common cases. The three-keyword categories align with the three `ObserverFindingKind` variants in the daemon.

7. **Help overlay includes Sprint 7 features** — The help text documents `Up/Down` for command history and `Tab` for slot ID completion, not just the Sprint 5 keybindings. Good attention to keeping documentation current.

8. **Demo script uses proper regex** — `grep -Eq 'slot slot-a[[:space:]]+status done'` is more robust than the prompt's suggested substring match. Correct use of `|| true` to prevent `set -e` from killing the script on grep no-match.

9. **Scope containment** — TUI-only changes plus `scripts/demo.sh`. No daemon, proto, or ctl modifications. The only non-TUI file touched is the demo script, exactly as scoped.

10. **Clean commit history** — Single commit on the sprint branch. HANDOFF.md, PLAN_NOW.md, and CHANGELOG.md all updated consistently.

---

## Issues

### Resolved

| Issue | Resolution |
|---|---|
| I-019 `demo.sh` doesn't wait for DONE | Poll loop with `grep -Eq` for done status, 15-iteration timeout |
| I-024 (partial) LoopDetected labels | `loop_detected_label()` parses reason strings into Loop Detected / Stuck / Budget Velocity |

### Open (unchanged)

| Issue | Notes |
|---|---|
| I-004 `provider_config` shallow merge | Deferred, low severity |
| I-005 SQLite schema migration versioning | Deferred, low severity |
| I-011 Recovery re-enqueue without worktree check | Low severity |
| I-012 Token/byte conflation in `truncate_payload` | Low severity |
| I-013 Empty telemetry from malformed TOKENS lines | Low severity |
| I-018 Double-count risk in telemetry parser | Low severity |
| I-020 `observe_output` creates state for unknown slots | Low severity |
| I-021 Alert-only loop findings suppress re-alerting | Low severity |
| I-023 `candidate_paths` false-positive on URLs | Low severity |
| I-024 (remaining) LoopDetected proto flattening | Partial fix in Sprint 7 via string parsing; proto split deferred |
| I-029 Claude harness doc omits `--permission-mode` flags | Low severity, documentation only |

### New

No new issues. Sprint 7 findings are all [Low] or [Info] severity.

---

## Verdict

**APPROVE** — Sprint 7 delivers all four parts as specified: reconnection resilience, command UX (history, status bar, tab-complete), help overlay, and two issue fixes. All exit criteria pass. The reconnect architecture is clean and correct. The per-dispatch client pattern is the right tradeoff. Test coverage increased by 11 tests. No new issues above Info severity. No regressions. The TUI is now production-ready for operator use.

Ready to merge.
