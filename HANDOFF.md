---
agent: gpt
status: ready
from: pc
timestamp: 2026-03-15T00:17:00-07:00
task: "Sprint 1 — WAL Recovery + Agent Harness"
---

# Handoff: pc → gpt (Codex)

## What This Sprint Delivers

Two capabilities that take Nexode from "validated spike" to "usable tool":

1. **WAL-based crash recovery** — daemon persists runtime state to disk, survives restarts, recovers slot state and cost totals
2. **Agent Harness abstraction** — trait-based adapter layer replacing mock scripts, enabling real Claude Code and Codex CLI agents
3. **Basic context compiler** — assembles task description, file globs, and git diff into context payload for agent dispatch

## What Was Done (pc, Sprint 1 Prep)

1. **Phase 0 code review** — reviewed all 3 Codex commits, verified 6/6 exit criteria met
2. **ISSUES.md** — cataloged 8 issues (I-001 through I-008) and 7 risks (R-001 through R-007); 4 issues resolved, 4 remaining low-priority
3. **Merged Phase 0** — squash-merged PR #6 (Codex spike) and PR #5 (ISSUES.md) into main
4. **Sprint 1 architecture** — wrote `docs/architecture/wal-recovery.md` (WAL format, recovery protocol, compaction strategy) and `docs/architecture/agent-harness.md` (trait design, harness implementations, context compiler)
5. **Sprint 1 instructions** — wrote `.agents/CODEX-SPRINT-1.md` with full task breakdown, exit criteria, and dependency list
6. **Updated PLAN_NOW.md, ROADMAP.md** — Sprint 1 tasks and milestone tracking

## What to Do (gpt/Codex)

Read `.agents/CODEX-SPRINT-1.md` for full instructions. Key points:

- **Branch:** `agent/gpt/sprint-1-wal-harness`
- **Week 1:** WAL persistence + recovery + engine integration + tests
- **Week 2:** AgentHarness trait + MockHarness + ClaudeCodeHarness + CodexCliHarness + context compiler + harness selection + tests
- **New dependencies:** `bincode`, `sha2`, `crc32fast`, `uuid`, `async-trait`, `glob`
- **6 exit criteria** defined in sprint doc — all must pass

## Files to Read First

1. `AGENTS.md` — Rules, capabilities, git conventions
2. `.agents/CODEX-SPRINT-1.md` — Full sprint instructions (start here)
3. `docs/architecture/wal-recovery.md` — WAL format and recovery protocol
4. `docs/architecture/agent-harness.md` — Harness trait design and implementations
5. `DECISIONS.md` — All accepted architectural decisions (D-002 through D-010)
6. `ISSUES.md` — Open issues and risks from Phase 0

## Files NOT to Modify

- `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*` — pc's domain

## Key Technical Decisions

- **WAL file:** `.nexode/wal.binlog`, framed `[u32 len][u32 crc][payload]`, bincode serialization
- **Recovery:** Option A — kill + respawn surviving agents (don't re-attach via PID)
- **Checkpoint interval:** 60 seconds, full RuntimeState snapshot
- **Harness trait:** Stateless, synchronous `build_command` + line-oriented `parse_telemetry`/`detect_completion`
- **Context compiler:** Minimal Phase 1 — task + globs + git diff + README, no AST/embeddings
- **Harness selection:** Infer from `model` field, optional explicit `harness` override in session.yaml

## Open Issues (Low Priority, Not Sprint Blocking)

- I-004: `provider_config` shallow merge
- I-005: SQLite schema has no migration versioning
- I-007: Merge queue drains on tick only (2s delay)
- I-008: Manual arg parsing vs clap in daemon main.rs
