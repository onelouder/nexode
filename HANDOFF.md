---
agent: gpt
claimed: 2026-03-19T09:24:19-07:00
status: handoff
from: gpt
task: "Sprint 10 Tranche C complete. Ready for PC review."
branch: "agent/gpt/sprint-10c-view-modes"
next: pc
---

# HANDOFF.md

> Last updated: 2026-03-19 by gpt
> Sprint 10 Tranche B review complete. Merged as PR #23 at `9b1a8a8`.

## Current Session (2026-03-19)

Sprint 10 Tranche C is complete on `agent/gpt/sprint-10c-view-modes` and is ready for `pc` review.

Completed in this session:

- added Synapse Grid Project Groups, Flat View, and Focus View modes with header controls and focus-project selection
- extracted shared formatter, tone, and alert-label helpers to `extensions/nexode-vscode/webview/shared/format.ts`
- added rolling recent observer alert state to `StateCache` and included it in the host→webview `StateEnvelope`
- joined observer alerts into both slot and kanban card view models and rendered alert badges/details in Synapse Grid, sidebar, and Macro Kanban
- added flat-view sorting helpers and Focus View expanded slot detail with dependency and alert context
- expanded Tier 1 coverage with shared formatter tests, recent-alert buffer tests, flat-view sorting tests, and the `projectFilter = 'all'` selector path
- verified:
  - `npm run build`
  - `npm run build:webview`
  - `npm run check-types`
  - `npm test`
  - `cargo check --workspace`
  - `cargo test --workspace`

Outputs for review:

- `extensions/nexode-vscode/src/state.ts`
- `extensions/nexode-vscode/src/view-models.ts`
- `extensions/nexode-vscode/src/webview-support.ts`
- `extensions/nexode-vscode/webview/shared/types.ts`
- `extensions/nexode-vscode/webview/shared/format.ts`
- `extensions/nexode-vscode/webview/synapse-grid/App.tsx`
- `extensions/nexode-vscode/webview/synapse-grid/styles.css`
- `extensions/nexode-vscode/webview/kanban/App.tsx`
- `extensions/nexode-vscode/webview/kanban/styles.css`
- `extensions/nexode-vscode/test/state.test.ts`
- `extensions/nexode-vscode/test/view-models.test.ts`
- `extensions/nexode-vscode/test/format.test.ts`

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
