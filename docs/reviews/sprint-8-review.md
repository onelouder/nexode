# Sprint 8 Code Review: Daemon Hardening + Issue Sweep

**Branch:** `agent/gpt/sprint-8-daemon-hardening`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-17
**Commit reviewed:** `ccf40ea [gpt] handoff: complete sprint 8 -> pc review`

---

## Summary

Sprint 8 is a focused technical-debt sweep across the daemon and proto layers. All four parts are delivered as specified: observer hardening (I-020, I-021, I-023), proto FindingKind enum (I-024), telemetry/doc fixes (I-013, I-029), and infrastructure (R-006 MSRV, reconnect integration test). The diff is +497/-137 across 16 files — the right size for a hardening sprint.

The most architecturally significant change is the observer cooldown model in `observer.rs`. Replacing one-shot `emitted_*_alert: bool` flags with `Option<Instant>` timestamps gives operators repeated warnings if a loop persists past the cooldown window. This is the correct fix for I-021 — the previous model created a silent gap between first alert and intervention. The `should_emit_alert()` helper is clean and testable.

The `observe_output()` slot guard (I-020) is a one-line fix (`entry().or_default()` → `get_mut()`) with exactly the right semantics: unknown slots return `None` silently. The `candidate_paths()` filtering (I-023) is thorough — URL, source-location, MIME-type, and module-path filters each get their own predicate function with correct edge-case handling.

Proto work (I-024) is well-structured: the `FindingKind` enum lives in `hypervisor.proto`, the mapping function in `engine/events.rs` covers all variants including the `SandboxViolation`/`UncertaintySignal` → `Unspecified` fallback, and the TUI consumes it with a clean `or_else` chain for backward compatibility with older daemon versions.

Code quality is consistent with prior sprints. Tests are targeted and cover the right boundaries.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| I-020: Guard `observe_output` against unknown slots | PASS | `observer.rs:122` — `get_mut()` returns `None` for unknown slots; test `observe_output_ignores_unknown_slots` validates |
| I-021: Alert cooldown for repeated findings | PASS | `Option<Instant>` timestamps replace booleans; `should_emit_alert()` at `observer.rs:345-352`; configurable `alert_cooldown_seconds` (default 300); test `loop_alert_rearms_after_cooldown` validates |
| I-023: Filter non-filesystem tokens from candidate paths | PASS | `is_url_token`, `is_source_location_token`, `is_mime_type_token`, `::` filter in `candidate_paths()`; test `candidate_paths_filters_common_non_filesystem_tokens` validates 4 reject + 3 accept cases |
| I-024: Proto `FindingKind` enum | PASS | `hypervisor.proto:177-182` adds enum; `events.rs:152-160` `finding_kind_to_proto()` mapping; TUI `events.rs:202-208` `loop_detected_label_from_proto()` with fallback; engine test asserts `finding_kind` on loop kill alert |
| I-013: Reject empty `TOKENS` telemetry | PASS | `process.rs:416-418` adds `is_empty()` method; `parse_space_delimited` uses `(!telemetry.is_empty()).then_some(telemetry)` instead of `found` flag; test `rejects_empty_space_delimited_telemetry` validates garbage/abc/valid cases |
| I-029: Update Claude harness doc | PASS | `agent-harness.md:87` now reads `claude -p --verbose --output-format stream-json --permission-mode bypassPermissions` |
| R-006: MSRV in Cargo.toml and README | PASS | All 4 crate manifests have `rust-version = "1.85"`; README has Requirements section with `Rust 1.85+ (edition 2024)` |
| Integration test: daemon restart → reconnect | PASS | `daemon_restart_allows_tui_clients_to_reconnect` at `tests.rs:792-893` — full lifecycle: start, connect, shutdown, verify failed reconnect, restart on same port, verify `GetFullState` + `SubscribeEvents` |
| No regressions | PASS (per agent handoff) | `cargo fmt --all`, `cargo check`, `cargo clippy -D warnings`, `cargo test --workspace` all pass |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `cargo fmt --all` | PASS |
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `cargo build -p nexode-tui` | PASS |
| `cargo build -p nexode-daemon` | PASS |

**Test counts:**

| Crate | Sprint 7 | Sprint 8 | Delta |
|---|---|---|---|
| nexode-daemon (lib) | 67 | 73 | +6 |
| nexode-daemon (bin) | 3 | 3 | 0 |
| nexode-ctl | 4 | 4 | 0 |
| nexode-tui (lib) | 28 | 28 | 0 |
| nexode-tui (bin) | 6 | 6 | 0 |
| **Total** | **108** | **114** | **+6** |

New tests: `observe_output_ignores_unknown_slots`, `loop_alert_rearms_after_cooldown`, `candidate_paths_filters_common_non_filesystem_tokens`, `loop_style_observer_findings_map_to_proto_finding_kind`, `rejects_empty_space_delimited_telemetry`, `daemon_restart_allows_tui_clients_to_reconnect`.

---

## Findings

### F-01 [Info] `should_emit_alert` uses `<=` for cooldown check — suppresses on exact boundary

**Location:** `observer.rs:348`

```rust
Some(previous) if now.duration_since(*previous) <= cooldown => false,
```

Using `<=` means an alert at exactly the cooldown boundary is suppressed. Using `<` would re-arm at exactly `cooldown` elapsed. In practice, `Instant` granularity makes this a non-issue — the probability of hitting the exact nanosecond boundary is zero. Noting for completeness.

**Recommendation:** No action.

### F-02 [Info] `is_source_location_token` returns false for column-only patterns like `file:42`

**Location:** `observer.rs:381-398`

The function correctly handles `src/lib.rs:42:10` (path:line:col) and `src/lib.rs:42` (path:line). However, a token like `file:42` without a path separator would pass `is_line_number` for the trailing `42` but fail `is_path_like_token` for `file` — which is correct, since `file` alone is not path-like. The function correctly requires the path portion to look like a filesystem path. Good.

**Recommendation:** No action.

### F-03 [Info] `is_mime_type_token` excludes tokens with dots in subtype

**Location:** `observer.rs:418`

```rust
|| subtype.contains('.')
```

This prevents `application/vnd.openxmlformats-officedocument.wordprocessingml.document` from being detected as a MIME type. But it also prevents `text/html.old` (not a real MIME type anyway). Since real MIME subtypes don't contain dots, this filter is fine. The primary concern — filtering `application/json` and `text/plain` — is handled correctly.

Actually, standard MIME types do have dots in subtype trees (e.g., `application/vnd.api+json`, `application/vnd.ms-excel`). The dot check will let these through to the path candidate list. They won't resolve to anything in the worktree so the sandbox guard won't fire, but they'd be better filtered. Low priority since the sandbox guard is the second line of defense.

**Recommendation:** Low priority. Consider changing the dot check to only reject tokens where the "subtype" contains a path separator (`/`), or remove the dot check entirely and rely on the top-level MIME type name matching. Not blocking.

### F-04 [Info] Reconnect integration test does not exercise the TUI retry loop

**Location:** `tests.rs:792-893`

The test verifies the gRPC layer: daemon down → connection refused → daemon restart → fresh `GetFullState`/`SubscribeEvents` succeed. It does NOT verify the TUI's `reconnect_event_stream()` background retry loop with exponential backoff. The handoff notes this explicitly as a residual risk. This is acceptable — the reconnect retry loop was already tested in Sprint 7 (the TUI-level reconnect behavior is its own code path). The Sprint 8 test proves the server-side is clean across restart boundaries.

**Recommendation:** No action. TUI retry loop is Sprint 7 scope.

### F-05 [Info] `decision_marker_triggers_uncertainty_signal` test now requires pre-registration

**Location:** `observer.rs:538-539`

The existing test gained `detector.observe_status("slot-a", TaskStatus::Working)` before calling `observe_output`. This is required after the I-020 fix since `observe_output` now silently ignores unknown slots. Good — the test was updated to match the new contract. No behavioral regression.

**Recommendation:** No action.

### F-06 [Low] `nexode-ctl` test struct initializer hard-codes `finding_kind: 0`

**Location:** `crates/nexode-ctl/src/main.rs:524`

```rust
finding_kind: 0,
```

The test uses a raw `0` instead of `FindingKind::Unspecified as i32` for the new field. It compiles and works, but using the enum constant would be self-documenting and resistant to enum reordering (though proto enum `0` is stable by convention).

**Recommendation:** Low priority. Consider `FindingKind::Unspecified as i32` in a future cleanup. Not blocking.

---

## Verdict

**APPROVED.** All exit criteria met. Six new tests, zero regressions per agent handoff. Issues I-013, I-020, I-021, I-023, I-024, and I-029 are resolved. R-006 MSRV is now documented. No findings above Info severity. One low-priority finding (F-03/F-06) for future cleanup.

Sprint 8 clears the accumulated daemon issue debt cleanly. The codebase is ready for the VS Code extension milestone (M3b).
