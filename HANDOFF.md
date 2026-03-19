---
agent: pc
claimed: 2026-03-19T08:05:00-07:00
status: handoff
from: pc
task: "Sprint 10 — React Webviews + Extension Tests (Tranche A review complete)"
branch: "main"
next: gpt
---

# HANDOFF.md

> Last updated: 2026-03-19 by pc
> Sprint 10 Tranche A review complete. Merged as PR #22 at `4bfe2ff`.

## Current Session (2026-03-19)

Sprint 10 Tranche A review, merge, and handoff preparation for Tranche B.

Completed in this session:

- reviewed all 24 changed files in Sprint 10 Tranche A (+2350/-33)
- wrote formal review at `docs/reviews/sprint-10a-review.md` — APPROVED, no findings above Low
- created PR #22 and squash-merged to main at `4bfe2ff`
- updated `ISSUES.md`: R-011 downgraded from High to Medium (Tier 1 tests now present, Tier 2 still missing)
- updated `ROADMAP.md`: Sprint 10 milestone split into Tranche A (complete) and Tranche B/C (pending) deliverables
- wrote `PLAN_NOW.md` for Tranche B
- updated `HANDOFF.md` for gpt to claim Tranche B

Key review findings (all Low/Info, none blocking):

- F-01: Minor race condition in sidebar `postReady` vs `onDidReceiveMessage` listener registration. Not exploitable in practice.
- F-03: Kanban task cards don't show branch/cost (need TaskNode → AgentSlot join). Tranche B scope.
- F-07: `MOVE_TARGETS` in commands.ts omits RESOLVING and DONE. Intentional per D-009/D-010 semantics.
- F-09: CSP doesn't include `style-src 'unsafe-inline'`. Will matter if Tranche B adds inline styles for drag-and-drop.

## Current State

**Main branch:** `4bfe2ff` — Sprint 10 Tranche A: React Webview Infrastructure (#22)

### What just shipped (Sprint 10 Tranche A)

React webview infrastructure under `extensions/nexode-vscode/`. First React components in the workspace. Covers master-spec section 11 "Weeks 2-4" plumbing:

- Webview build pipeline (esbuild IIFE + browser target, minified React bundles)
- `SynapseGridPanel`, `SynapseSidebarProvider`, `KanbanPanel` shell panels
- Shared postMessage bridge (`webview/shared/bridge.ts`) with nonce-based CSP
- React 18 entry points for both surfaces
- `state.ts` decoupled from `vscode` namespace via `Emitter<T>` (555 → 734 lines)
- Full Phase 3 observer event normalization (`UncertaintyFlag`, `WorktreeStatusChanged`, `ObserverAlert`)
- D-012: MoveTask vs AssignTask command semantics
- Tier 1 unit tests for state.ts (251 lines, 4 test cases via `tsx --test`)

Review: `docs/reviews/sprint-10a-review.md` — APPROVED, no findings above Low severity.

### Codebase inventory (Sprints 0-10A)

| Component | Location | Language | Lines (approx) | Tests |
|---|---|---|---|---|
| nexode-daemon | `crates/nexode-daemon/` | Rust | ~8000 | 76 (lib+bin) |
| nexode-proto | `crates/nexode-proto/` | Proto/Rust | ~300 | 0 (generated) |
| nexode-ctl | `crates/nexode-ctl/` | Rust | ~600 | 4 |
| nexode-tui | `crates/nexode-tui/` | Rust | ~3000 | 34 (lib+bin) |
| nexode-vscode | `extensions/nexode-vscode/` | TypeScript | ~2800 | 4 (state.test.ts) |
| **Total** | | | ~14,700 | **118** |

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
| R-011 | Medium | VS Code extension: Tier 2 integration tests still missing |

## For Tranche B Agent

See `PLAN_NOW.md` for the Tranche B task definition. The infrastructure is solid. Focus on:

1. Making the Synapse Grid and Kanban render live state from `StateCache`
2. Adding the TaskNode → AgentSlot join so Kanban cards show branch and cost
3. Getting drag-and-drop working for Kanban column moves (dispatch `MoveTask`)
4. If CSP blocks inline styles for drag transforms, add `'unsafe-inline'` to `style-src` in `webview-support.ts`
