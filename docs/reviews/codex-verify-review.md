# Codex Verification Review — Pre-Sprint 3 Gate

> **Date:** 2026-03-15
> **Reviewer:** pc
> **Branch:** `agent/gpt/codex-verify` (commit `a390359`)
> **Base:** `main` (commit `125b946`)
> **Diff:** 333 lines across 6 files (+110 / -73)
> **Agent:** gpt (Codex)

---

## Scope Summary

| Deliverable | Status |
|---|---|
| `live_codex_cli_hello_world` — real Codex process | ✅ Passed |
| `live_full_lifecycle` — forced Codex (Claude unavailable) | ✅ Passed |
| `scripts/demo.sh` with `codex-cli` harness | ✅ Passed (I-019 known) |
| Codex completion detection aligned to real output | ✅ Fixed |
| Codex telemetry parsing aligned to real output | ✅ Fixed |
| Default model path for Codex (no `--model` flag) | ✅ Implemented |
| `HANDOFF.md` updated with verification results | ✅ Done |

---

## Findings

### F-001: `cached_input_tokens` as fallback for `tokens_in` is semantically misleading — Severity: Low

**File:** `harness.rs`, `parse_json_summary_telemetry()`, tokens_in path list

The `tokens_in` path probe list now includes `usage.cached_input_tokens` and `usage.cachedInputTokens` as fallback paths:

```rust
tokens_in: json_u64_at_paths(
    &value,
    &[
        &["usage", "input_tokens"],      // ← first match wins
        &["usage", "inputTokens"],
        &["usage", "cached_input_tokens"], // ← fallback
        &["usage", "cachedInputTokens"],
        &["usage", "prompt_tokens"],
        ...
    ],
),
```

`json_u64_at_paths` uses `find_map`, so it returns the **first match**. When `input_tokens` is present (as in the real Codex output `{"usage":{"input_tokens":8008,"cached_input_tokens":7040,...}}`), cached tokens are never reached. This is correct in practice.

However, if a future CLI version emits only `cached_input_tokens` without `input_tokens`, it would be used as `tokens_in` — which is semantically wrong (cached input tokens are a subset of input tokens, not a replacement). The accounting would undercount.

**Risk:** Very low. Both Claude and Codex emit `input_tokens` today. The fallback path would only activate in an unlikely format change, and the undercount would surface immediately in telemetry validation.

**Action:** Acceptable as-is. If Codex telemetry format changes, revisit whether `cached_input_tokens` should be a separate tracked field rather than a `tokens_in` fallback.

---

### F-002: `"default"` as a magic string for model omission — Severity: Low

**File:** `harness.rs`, `CodexCliHarness::build_command()`

```rust
if !config.model.trim().is_empty() && config.model != "default" {
    args.push("--model".into());
    args.push(config.model.clone().into());
}
```

The string `"default"` is a sentinel value that means "don't pass `--model` to the Codex CLI." This is pragmatic — it works — but it's undocumented and only enforced in one place. If someone writes `model: "default"` in a Claude session YAML, `ClaudeCodeHarness` would pass `--model default` to `claude`, which would likely fail.

The live test and demo both use this correctly (`LiveHarness::model()` returns `"default"` for Codex, `"claude-sonnet-4-5"` for Claude; demo.sh defaults to `"default"` for codex-cli). The guard also handles empty strings via `.trim().is_empty()`.

**Risk:** Low. The sentinel only matters for Codex, and the session YAML `model` field is typically set per-harness.

**Action:** Consider documenting the `"default"` sentinel in the session schema or `agent-harness.md` in a future docs pass. Non-blocking.

---

### F-003: HANDOFF.md still carries Sprint 2 body text — Severity: Low

**File:** `HANDOFF.md`

The YAML frontmatter correctly references `"Pre-Sprint 3 — Codex CLI Live Verification"` and the `agent/gpt/codex-verify` branch. But the body section "What This Sprint Delivers" still describes Sprint 2's three pillars (bug fixes, command ack, live integration). The new Codex verification section is appended below, which is useful, but the legacy body text is misleading — this branch didn't deliver I-009/I-010/I-015/R-007; Sprint 2 did.

**Risk:** Cosmetic only. The YAML header is correct and the verification results section is accurate. Any reviewer reading the full HANDOFF.md might be briefly confused but the verification table is clear.

**Action:** None required for merge. Could be cleaned up on the next HANDOFF.md update by the next active agent.

---

## Regression Analysis

| Harness | Regression Risk | Analysis |
|---|---|---|
| **MockHarness** | None | Zero lines changed. Not in diff. |
| **ClaudeCodeHarness** | None | `build_command`, `detect_completion`, `parse_telemetry` all untouched. `parse_json_summary_telemetry` gate adds `type: "turn.completed"` but Claude emits `type: "result"` — no overlap. `cached_input_tokens` path is a fallback after `input_tokens` which Claude also provides first. |
| **CodexCliHarness** | Positive (intentional) | Completion detection widened to include `type: "turn.completed"`. Existing `event: "done"` and `status: "completed"` paths retained as fallbacks. Model flag conditionally omitted for `"default"`. |

The `OsString` arg construction in `CodexCliHarness::build_command` is a type-level change from `Vec<&str>` to `Vec<OsString>`, but `AgentCommand::new` accepts `impl IntoIterator<Item = impl Into<OsString>>`, so both `Vec<&str>` (Claude) and `Vec<OsString>` (Codex) are valid. No API contract change.

---

## Test Coverage

| Change | Test Coverage |
|---|---|
| `detect_completion` for `turn.completed` | ✅ `real_harness_completion_detection_uses_json_instead_of_substring_matching` — asserts `codex.detect_completion(r#"{"type":"turn.completed"}"#)` returns true |
| `parse_telemetry` for Codex `turn.completed` usage | ✅ `real_harnesses_parse_json_summary_telemetry_without_counting_partial_messages` — asserts `tokens_in: 8008`, `tokens_out: 27`, `cost_usd: None` from real Codex output shape |
| `build_command` with explicit model | ✅ `codex_command_writes_codex_instructions_and_uses_exec` — asserts `--model gpt-5-codex` in args |
| `build_command` with `"default"` model | ✅ `codex_command_omits_model_flag_for_default_model` — asserts no `--model` in args |
| Live smoke test (Codex) | ✅ `live_codex_cli_hello_world` — passed with real credentials |
| Live lifecycle (Codex) | ✅ `live_full_lifecycle` — passed with forced Codex path |
| Demo script (Codex) | ✅ `scripts/demo.sh` — ran with `NEXODE_DEMO_HARNESS=codex-cli` |

### Missing Test Coverage

1. **`build_command` with empty string model** — the guard handles `config.model.trim().is_empty()` but no test covers `model: ""`. Low risk since the session parser would typically provide a non-empty value.
2. **`parse_json_summary_telemetry` with only `cached_input_tokens`** (no `input_tokens`) — the fallback path is untested. See F-001.

---

## Open Questions / Assumptions

**Q1:** The HANDOFF.md records `codex-cli 0.104.0-alpha.1`. Is this the version pinned for the project, or should live tests tolerate version drift? The `turn.completed` completion signal may change in future Codex releases.

**Q2:** The `"default"` model sentinel means Codex picks its own default model. Is this acceptable for production sessions, or should the session schema require an explicit model for production use (with `"default"` only valid for testing)?

---

## Merge Recommendation

### `ready`

This is a clean, minimal verification follow-up. The three changes (completion detection, telemetry parsing, default model path) are all correct responses to discovering real Codex CLI output format. The changes are additive — existing Claude and mock paths are untouched, and all fallback detection for `event: "done"` / `status: "completed"` is preserved in case future Codex versions change again.

**Blocking before merge:** Nothing.

**Residual risks:**
- Codex CLI is alpha (`0.104.0-alpha.1`). The `turn.completed` signal could change in future versions. The retained fallback paths provide some buffer.
- The `"default"` sentinel is undocumented but functional. Worth a line in `agent-harness.md` next time docs are updated.
- I-019 (demo.sh doesn't wait for DONE) remains unchanged — expected and already tracked.

**Code quality notes:**
- The diff is appropriately small for a verification follow-up (110 lines added, 73 removed — mostly PLAN_NOW.md cleanup).
- New tests cover both the `turn.completed` detection and the `"default"` model omission path.
- HANDOFF.md verification results table is well-structured with specific observations about model compatibility.
