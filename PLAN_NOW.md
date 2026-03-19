# PLAN_NOW.md — Sprint 11: Merge Choreography TreeView + Extension Polish

> Owner: gpt (Codex)
> Reviewer: pc (Perplexity Computer)
> Status: ready for gpt
> Spec reference: master-spec sec-11 "Weeks 2-4" (Merge Choreography) and "Weeks 5-8" (Polish)
> Previous sprint: Sprint 10 — PRs #22, #23, #24 (commits `4bfe2ff`, `9b1a8a8`, `d13add7`)

## Objective

Deliver the Merge Choreography TreeView and extension polish. After this sprint, the VS Code extension covers all sec-11 deliverables except the Chat Participant (deferred to Sprint 12), and the extension is ready for settings configuration and basic onboarding.

## Why this scope

Sprint 10 shipped all three React webview surfaces (Synapse Grid, Sidebar, Macro Kanban) with full live rendering, view modes, drag-and-drop, and observer alerts. The remaining sec-11 "Weeks 2-4" gap is the Merge Choreography TreeView — a native VS Code TreeView (not a React webview) that shows worktrees in REVIEW state with conflict risk and approve/reject actions.

The Chat Participant (`@nexode`) is deferred to Sprint 12 because:
- The VS Code Chat API requires careful prompt engineering and structured command parsing
- It depends on a stable extension surface (all TreeViews, webviews, and settings in place)
- Sprint 11 already has substantial scope with the TreeView + full polish pass

## Starting Point

Sprint 10 shipped:
- React webview pipeline: esbuild IIFE bundles, nonce-based CSP, postMessage bridge
- Synapse Grid: Project Groups, Flat View, Focus View, slot cards, agent pills, observer alerts
- Macro Kanban: live state, drag-and-drop column moves, project filter, alert badges
- StateCache: full event normalization (Phase 3 observer events), agent tracking, rolling alert buffer
- ~17 Tier 1 tests across 4 test files
- Shared formatter library (`webview/shared/format.ts`)

What's missing for sec-11 completion:
- Merge Choreography TreeView (sec-11 "Weeks 2-4" — native TreeView, not a webview)
- Settings page (session.yaml path, socket path, theme selection)
- Extension README and onboarding walkthrough
- Extension host integration tests (R-011 Tier 2 — stretch goal)

## Scope

### S11-01: Merge Choreography TreeView

**What:** A native VS Code TreeView registered in the AuxiliaryBar that visualizes the merge queue and worktrees in REVIEW state.

**Spec (sec-11):** "VS Code native TreeView (AuxiliaryBar). Shows worktrees in REVIEW state, conflict risk score, approve/reject actions."

**Implementation:**

- Register a new `TreeDataProvider` in `extensions/nexode-vscode/src/merge-tree.ts`
- Register the TreeView in `package.json` under `viewsContainers.activitybar` or `views` for the AuxiliaryBar
- **Tree structure:**
  - Root: one node per project with active merge queue entries
  - Children: one node per slot in REVIEW or MERGE_QUEUE status
  - Each node shows: slot ID, branch name, agent that produced the work, status (REVIEW / MERGE_QUEUE / RESOLVING)
- **Conflict risk indicator:** Display a risk badge per slot. The risk can be computed from:
  - Number of files changed (available from the last agent event or slot metadata)
  - Whether other slots in the same project are also in REVIEW (concurrent merge risk)
  - Simple heuristic is fine for Sprint 11 — the spec says "conflict risk score" but a Low/Medium/High label from a heuristic is acceptable. Full AST-based scoring is Phase 4 (sec-12).
- **Actions (inline TreeView buttons):**
  - **Approve → Merge Queue:** Dispatches a `MoveTask` command to transition slot from REVIEW → MERGE_QUEUE
  - **Reject → Working:** Dispatches a `MoveTask` command to transition slot from REVIEW → WORKING (sends agent back to fix)
  - **Pause:** Dispatches a `PauseSlot` command
- **Live updates:** Subscribe to StateCache EventBus. Refresh the tree when slot status changes affect REVIEW/MERGE_QUEUE/RESOLVING states.
- **Empty state:** When no slots are in REVIEW or MERGE_QUEUE, show a "No active merges" placeholder.

### S11-02: Extension Settings Page

**What:** VS Code settings for configuring the Nexode extension.

**Spec (sec-11 Weeks 5-8):** "Settings page (session.yaml path, socket path, theme)."

**Implementation:**

- Add `contributes.configuration` entries in `package.json`:
  - `nexode.sessionPath`: Path to `session.yaml` (string, default: `.nexode/session.yaml`)
  - `nexode.daemonHost`: Daemon gRPC host (string, default: `localhost`) — already exists, verify
  - `nexode.daemonPort`: Daemon gRPC port (number, default: `50051`) — already exists, verify
  - `nexode.socketPath`: Unix socket path (string, default: empty — uses host:port when empty)
  - `nexode.theme`: UI theme preference (`'auto' | 'synapse-dark' | 'synapse-light'`, default: `'auto'`)
  - `nexode.showStatusBar`: Toggle Status Bar HUD visibility (boolean, default: `true`)
- Ensure live reload works for connection settings (host/port/socket changes trigger reconnect)
- Settings should appear in the VS Code Settings UI under a "Nexode" section

### S11-03: Extension README + Onboarding

**What:** User-facing documentation and a lightweight onboarding walkthrough.

**Implementation:**

- **README.md** for the extension (`extensions/nexode-vscode/README.md`):
  - Overview: what Nexode is, what the extension does
  - Prerequisites: Rust daemon running, session.yaml configured
  - Installation: from VSIX or marketplace (placeholder)
  - Configuration: list of settings with defaults
  - Features: screenshots/descriptions of Synapse Grid, Kanban, Merge TreeView, Status Bar
  - Commands: list of command palette commands
  - Troubleshooting: common issues (daemon not running, connection refused, etc.)
- **Onboarding walkthrough** (optional, stretch):
  - Register a `walkthroughs` contribution in `package.json`
  - Steps: Install daemon → Configure session.yaml → Connect extension → Explore Synapse Grid
  - Each step opens the relevant view or setting

### S11-04: Test Coverage Expansion

**What:** Extend Tier 1 tests and optionally begin Tier 2.

- Test `MergeTreeDataProvider`: tree structure generation from StateCache data, refresh on state change, conflict risk calculation
- Test settings validation: verify defaults, live reload behavior
- **Stretch — Tier 2 (R-011):** If time permits, set up `@vscode/test-electron` infrastructure and add at least one activation lifecycle test. This would partially close R-011.

## Non-Goals for Sprint 11

- Chat Participant (`@nexode`) — deferred to Sprint 12
- Rich per-cell presentation (spark-lines, progress bars) — deferred
- Full AST-based conflict risk scoring — that's Phase 4 (sec-12). Sprint 11 uses heuristic risk.
- QG-1 score gate integration — backlog
- Tier 2 tests are stretch only — not a hard deliverable

## Constraints

1. **No Rust changes.** TypeScript-only. `cargo test --workspace` must still pass (114 tests).
2. **Native TreeView for Merge Choreography.** This is NOT a React webview — it uses the VS Code TreeView API (`vscode.TreeDataProvider`). This is per the spec.
3. **Webview security.** Maintain nonce-based CSP. No `'unsafe-inline'` in `style-src`.
4. **TypeScript strict mode.** Do not relax `strict: true`.
5. **D-012 compliance.** Column moves dispatch `MoveTask`. Agent assignment uses `AssignTask`. Do not conflate.
6. **Shared code convention.** Utilities shared between extension host and webviews go in `webview/shared/`. Extension-host-only code stays in `src/`.

## Verification

Before handing off, run:
```
cd extensions/nexode-vscode
npm install
npm run build          # Extension host bundle
npm run build:webview  # Webview React bundles
npm run check-types    # TypeScript strict
npm run test           # Unit tests (Tier 1)
cd ../..
cargo check --workspace
cargo test --workspace  # Must still be 114+ tests
```

## Handoff protocol

When complete:
1. Ensure all verification commands pass
2. Update `CHANGELOG.md` with Sprint 11 entry
3. Commit to a working branch: `agent/gpt/sprint-11-merge-tree-polish`
4. Update `HANDOFF.md` with completion status
5. Do NOT merge — pc will review and merge
