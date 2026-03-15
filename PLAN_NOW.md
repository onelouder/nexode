# PLAN_NOW.md — Current Short-Horizon Plan

> What we're doing right now. Updated by the active agent during their turn.
> This replaces ambiguous "working" files. Keep it concrete and bounded.

## Current Sprint

- **Goal:** Pre-Sprint 3 — Codex CLI Live Verification
- **Deadline:** 2026-03-15
- **Active Agent:** gpt
- **Previous sprint:** Sprint 2 — Real Agent Integration + Critical Fixes (complete, merged to main)

## Tasks

### Verification Gate

- [x] Read `.agents/prompts/codex-verify.md` and required context files.
- [x] Confirm baseline tests pass: `cargo test -p nexode-daemon`, `cargo test -p nexode-ctl`, `cargo check --workspace`.
- [x] Run real Codex smoke test: `live_codex_cli_hello_world`.
- [x] Run forced-Codex full lifecycle test: `live_full_lifecycle` with `ANTHROPIC_API_KEY` unset.
- [x] Run `scripts/demo.sh` with `NEXODE_DEMO_HARNESS=codex-cli`.
- [x] Patch Codex compatibility based on real CLI output:
  - use Codex default model path for tests/demo instead of hard-coding `gpt-4.1`
  - detect completion from `type: "turn.completed"`
  - parse telemetry from Codex `turn.completed` JSON usage fields
- [x] Update `HANDOFF.md` with verification results.

## Blocked

- None

## Done This Sprint

- Verified Codex live compatibility against the merged Sprint 2 code on `main`.
- Confirmed the current Codex CLI emits `type: "turn.completed"` as the success marker.
- Confirmed the current Codex CLI records usage under `usage.input_tokens`, `usage.cached_input_tokens`, and `usage.output_tokens`.
- Fixed the Codex harness to detect and parse that real output shape.
- Switched the Codex live-test/demo default model path to Codex's own default model because `gpt-4.1` is not supported in this account context.
- Re-ran the real Codex smoke test and forced-Codex full lifecycle test successfully.
- Confirmed `scripts/demo.sh` runs successfully with Codex, with the known `merge_queue` exit timing from `I-019`.

## Next Up

- Review `agent/gpt/codex-verify`.
- Merge the Codex verification follow-up once reviewed.
- Sprint 3: Observer Loops + Safety (loop detection, uncertainty routing, sandbox enforcement, event sequence numbers)

## Notes

- Verification prompt: `.agents/prompts/codex-verify.md`
- All Phase 0 + Sprint 1 + Sprint 2 decisions remain binding
- Open issues: see `ISSUES.md` — `I-016` through `I-019` remain open
- Live tests gated behind `--features live-test` — require `claude` or `codex` CLI installed
- Real credential-backed Claude and Codex live verification are both complete
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
