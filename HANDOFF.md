---
agent: pc
claimed: 2026-03-19T09:02:00-07:00
status: handoff
from: pc
task: "Sprint 10 Tranche B review complete. Handoff for Tranche C."
branch: "main"
next: gpt
---

# HANDOFF.md

> Last updated: 2026-03-19 by pc
> Sprint 10 Tranche B review complete. Merged as PR #23 at `9b1a8a8`.

## Current Session (2026-03-19)

Sprint 10 Tranche B review, merge, and handoff preparation for Tranche C.

Completed in this session:

- reviewed all 17 changed files in Sprint 10 Tranche B (+1086/-103)
- wrote formal review at `docs/reviews/sprint-10b-review.md` — APPROVED, no findings above Low
- created PR #23 and squash-merged to main at `9b1a8a8`
- updated `ISSUES.md`: R-011 likelihood downgraded from Medium to Low (Tier 1 now ~11 test cases, Tier 2 still missing)
- updated `ROADMAP.md`: Sprint 10 Tranche B deliverables checked off, Tranche C scope defined
- wrote `PLAN_NOW.md` for Tranche C
- updated `HANDOFF.md` for gpt to claim Tranche C

Key review findings (all Low/Info, none blocking):

- F-01: Duplicate utility functions (formatCurrency, formatCount, toTitleWords, etc.) across Synapse Grid and Kanban webview components. Should be extracted to `webview/shared/format.ts` in Tranche C.
- F-07: `view-models.test.ts` doesn't test the `projectFilter = 'all'` default path. Minor test gap.

## Previous Sessions

### 2026-03-19 (Tranche A)

Sprint 10 Tranche A review, merge, and handoff preparation for Tranche B.

- Reviewed 24 files (+2350/-33). APPROVED.
- PR #22, merged at `4bfe2ff`.
- R-011 downgraded from High to Medium.
- Findings: F-01 ready-listener race (closed in Tranche B), F-03 branch/cost join (closed in Tranche B), F-09 CSP note (avoided in Tranche B).

### 2026-03-19 (Tranche B — gpt)

Sprint 10 Tranche B delivery on `agent/gpt/sprint-10b-webview-shells`.

- Live Synapse Grid and Macro Kanban rendering via `view-models.ts` join layer.
- HTML5 drag-and-drop for Kanban column moves via MoveTask dispatch.
- `StateCache` agent tracking with `AgentPresence`, `seedAgents`, and agent selectors.
- Tier 1 test expansion: 7 new test cases across 3 files.
- All verification commands passed.

## Current State

**Main branch:** `9b1a8a8` — Sprint 10 Tranche B: Live Webview Surfaces (#23)

### What just shipped (Sprint 10 Tranche B)

Live state rendering for both React webview surfaces, drag-and-drop Kanban column moves, agent state tracking, and shared join utilities. All Tranche A review follow-ups closed.

- `view-models.ts`: `buildSlotCardModels`, `buildKanbanCardModels` — data-projection layer between StateCache and React components
- `kanban-commands.ts`: Extracted `createMoveTaskCommand` with injectable ID factory
- `state.ts`: `AgentPresence` tracking, `seedAgents()` with event-state preservation, `getAgentStates()`, `getAgentState()`, `getAgentsBySlot()`
- Synapse Grid: SlotCard component, status/agent/mode pills, metric header, sidebar pills
- Macro Kanban: HTML5 drag-and-drop, joined task cards with branch/cost, project filter auto-reset
- Ready-listener race fixed in all 3 panel/provider paths
- CSP unchanged — drag/drop uses class-based styling

Review: `docs/reviews/sprint-10b-review.md` — APPROVED, no findings above Low severity.

### Codebase inventory (Sprints 0-10B)

| Component | Location | Language | Lines (approx) | Tests |
|---|---|---|---|---|
| nexode-daemon | `crates/nexode-daemon/` | Rust | ~8000 | 76 (lib+bin) |
| nexode-proto | `crates/nexode-proto/` | Proto/Rust | ~300 | 0 (generated) |
| nexode-ctl | `crates/nexode-ctl/` | Rust | ~600 | 4 |
| nexode-tui | `crates/nexode-tui/` | Rust | ~3000 | 34 (lib+bin) |
| nexode-vscode | `extensions/nexode-vscode/` | TypeScript | ~3900 | ~11 (state, view-models, kanban-commands) |
| **Total** | | | ~15,800 | **~125** |

### Open Issues

| ID | Severity | Summary |
|---|---|---|
| I-004 | Low | `provider_config` shallow merge not implemented |
| I-005 | Low | SQLite schema has no migration versioning |
| I-011 | Low | Recovery re-enqueues merge slot without worktree check |
| I-012 | Low | Token/byte conflation in `truncate_payload` |
| I-018 | Low | `parse_json_summary_telemetry` could double-count |
| R-001 | Low | Verification worktree cleanup on panic |
| R-002 | Medium | `sh -lc` in verification loads user dotfiles |
| R-003 | Low | Telemetry parsing format undocumented |
| R-008 | High | VS Code Extension Host IPC bottleneck at N>3 |
| R-009 | Medium | Semantic drift between concurrent agents |
| R-010 | Medium | Agent CLI output format instability |
| R-011 | Low | VS Code extension: Tier 2 integration tests still missing |

## For Tranche C Agent

See `PLAN_NOW.md` for the Tranche C task definition. The live rendering infrastructure and data-projection layer are solid. Focus on:

1. Synapse Grid view mode switcher (Flat View, Focus View) — the current implementation only renders Project Groups
2. Extract shared formatters from `webview/synapse-grid/App.tsx` and `webview/kanban/App.tsx` into `webview/shared/format.ts` — 6 duplicate functions
3. Observer alert rendering in the webviews (the normalization layer from Tranche A already provides the data)
4. If time permits: rich per-cell presentation (spark-lines, progress bars)
