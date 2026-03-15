---
agent: pc
status: ready
from: pc
timestamp: 2026-03-15T09:17:00-07:00
task: "Sprint 2 — Real Agent Integration + Critical Fixes"
branch: "agent/gpt/sprint-2-real-agents"
---

# Handoff: Sprint 2 Planning

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

## Bug Fix Summary

| Issue | File | Line | Fix |
|---|---|---|---|
| I-009 | `process.rs` | ~325 | Non-zero exit always means failure. Add `requires_completion_signal()` to trait. |
| I-010 | `engine.rs` | ~670 | Emit `AgentStateChanged(Executing)` after `SlotAgentSwapped`. |
| I-015 | `harness.rs` | ~167-204 | Replace string `contains()` with `serde_json` parsing. |

## R-007 Changes

- **Proto:** `CommandResponse` gains `command_id` (echo) and `CommandOutcome` enum
- **Transport:** Channel becomes `(OperatorCommand, oneshot::Sender<CommandResponse>)` — transport awaits response with 5s timeout
- **Engine:** Command handler validates slot existence and state transitions, sends result through oneshot
- **nexode-ctl:** Prints actual command result instead of always "success"

## Dependencies

- `serde_json` crate (for I-015 fix)
- `claude` CLI (for live tests — gated behind feature flag)
- `codex` CLI (for live tests — gated behind feature flag)

## What NOT to Change

- No observer loops — Sprint 3
- No event sequence numbers (R-005) — Sprint 3
- No engine decomposition — tracked but not blocking
- No AGENTS.md, DECISIONS.md, docs/spec/*, docs/architecture/* modifications

## Previous Sprint Summary

Sprint 1 delivered WAL recovery and agent harness. 35 tests, all passing. 10 findings from code review — 1 high (R-007, addressed this sprint), 3 medium (I-009, I-010 addressed this sprint; R-005 deferred), 6 low (deferred). See `docs/reviews/sprint-1-review.md`.
