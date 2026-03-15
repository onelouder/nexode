---
agent: gpt
status: ready
from: gpt
timestamp: 2026-03-15T10:05:00-07:00
task: "Sprint 2 — Real Agent Integration + Critical Fixes"
branch: "agent/gpt/sprint-2-real-agents"
---

# Handoff: Sprint 2 Ready For Review

## What This Sprint Delivers

Sprint 2 proves the daemon works with real CLI agents end-to-end. Three pillars:

1. **Bug fixes** — I-009, I-010, I-015 (issues that would cause silent failures with real agents)
2. **Command acknowledgment** — R-007 (fire-and-forget → result-bearing response)
3. **Live integration** — smoke tests with real `claude`/`codex` CLI, end-to-end demo

## Key Documents

| Document | Location | Purpose |
|---|---|---|
| Sprint instructions | `.agents/CODEX-SPRINT-2.md` | Full task breakdown for gpt agent |
| Command ack architecture | `docs/architecture/command-ack.md` | R-007 design — oneshot pattern, proto changes |
| Sprint 1 review | `docs/reviews/sprint-1-review.md` | Source of I-009, I-010, I-015 findings |
| Issues registry | `ISSUES.md` | Full issue details with module/line references |

## Implemented

- **I-009 / `process.rs`**
  - Non-zero agent exit now always resolves as failure.
  - `AgentHarness` exposes `requires_completion_signal()`, so real harnesses require both zero exit and an explicit completion signal.
  - Mock harness compatibility is preserved for existing tests and local workflow.
- **I-010 / `engine.rs`**
  - `SlotAgentSwapped` now also emits `AgentStateChanged(Executing)` for the replacement agent, keeping observers and UI in sync after respawn.
- **I-015 / `harness.rs`**
  - Claude and Codex completion detection now parses JSON instead of relying on brittle substring matching.
- **R-007 command acknowledgment**
  - `CommandResponse` now echoes `command_id` and returns a `CommandOutcome`.
  - gRPC transport uses a oneshot request/response channel with timeout instead of fire-and-forget.
  - Engine command handling validates slot existence and task-state transitions before responding.
  - `nexode-ctl` surfaces actual command outcomes instead of always printing success.
- **Live integration**
  - Added gated `live-test` smoke tests for Claude and Codex harnesses.
  - Added `scripts/demo.sh` for an end-to-end local demo flow.
  - Fixed the Claude live harness contract so the daemon now requests JSON stream output, detects completion correctly, and records final usage/cost telemetry.

## Key Decisions Captured In Code

- Harnesses remain synchronous command builders plus line-oriented parsers. Process lifecycle, streaming, timeouts, and respawn logic stay in `process.rs`.
- Live tests are feature-gated and self-skip when required CLI binaries or API keys are unavailable.
- This environment verified the gated compile/self-skip path, not a credential-backed real CLI run.

## Dependencies

- `serde_json` crate (for I-015 fix)
- `claude` CLI (for live tests — gated behind feature flag)
- `codex` CLI (for live tests — gated behind feature flag)

## Verification

- `cargo fmt --all`
- `cargo test -p nexode-daemon`
- `cargo test -p nexode-ctl`
- `cargo check --workspace`
- `ANTHROPIC_API_KEY= OPENAI_API_KEY= cargo test -p nexode-daemon --features live-test --test live_harness -- --nocapture`
- `cargo test -p nexode-daemon --features live-test --test live_harness live_claude_code_hello_world -- --nocapture` with a real Claude API key
- `cargo test -p nexode-daemon --features live-test --test live_harness live_full_lifecycle -- --nocapture` with a real Claude API key

## Remaining Review Focus

- Codex live smoke coverage is still unverified in this environment.
- Confirm the command-ack outcome surface is sufficient for planned UI/client behavior.

## What NOT to Change

- No observer loops — Sprint 3
- No event sequence numbers (R-005) — Sprint 3
- No engine decomposition — tracked but not blocking
- No AGENTS.md, DECISIONS.md, docs/spec/*, docs/architecture/* modifications

## Previous Sprint Summary

Sprint 1 delivered WAL recovery and agent harness. 35 tests, all passing. 10 findings from code review — 1 high (R-007, addressed this sprint), 3 medium (I-009, I-010 addressed this sprint; R-005 deferred), 6 low (deferred). See `docs/reviews/sprint-1-review.md`.
