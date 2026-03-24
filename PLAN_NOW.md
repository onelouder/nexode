# PLAN_NOW.md — Sprint 12: Merge Choreography TreeView + Extension Polish

> Owner: claude
> Reviewer: pc (Perplexity Computer)
> Status: in progress on `agent/claude/sprint-12-merge-choreography`
> Spec reference: master-spec sec-11 "Weeks 2-4" (Merge Choreography) and "Weeks 5-8" (Polish)
> Previous sprint: Sprint 11-alt — Workspace Foundation + Feedback Surfaces (off-plan, reviewed + fixed)

## Context

Sprint 11-alt delivered off-plan but valid Phase 3 work: workspace folder management, output streaming, diagnostics, decorations, and diff commands. This was originally scoped by `pc` as Sprint 11 (Merge Choreography + Polish) but agent divergence produced different deliverables. Sprint 12 picks up the originally planned scope.

## Objective

Deliver the Merge Choreography TreeView and extension polish. After this sprint, the VS Code extension covers all sec-11 deliverables except the Chat Participant (Sprint 13), and the extension is ready for settings configuration and basic onboarding.

## Starting Point

Sprint 11-alt shipped:
- WorkspaceFolderManager: worktree-as-workspace-folder reconciliation (200ms debounce)
- OutputChannelManager: per-slot VS Code output channels with lazy creation
- DiagnosticManager: verification failure diagnostics in Problems panel (Rust, TSC, generic patterns)
- DecorationProvider: colored status badges (WK/RV/MQ/RS/DN/PA) on workspace folders
- Expanded webview message contract with Output/Diff action buttons
- Proto: AgentOutputLine, VerificationResult events
- 30 TypeScript tests, all passing

What's missing for sec-11 completion:
- **Merge Choreography TreeView** (sec-11 "Weeks 2-4" — native TreeView in AuxiliaryBar)
- **Extension Settings Page** (sec-11 "Weeks 5-8")
- **Extension README + Onboarding**

## Pre-Sprint: Merge to Main

Before starting Sprint 12 implementation:
1. Rebase `agent/gpt/sprint-10c-view-modes` onto `origin/main` (resolve any conflicts with `pc`'s Sprint 10C review/handoff commits)
2. Create PR for Sprint 11-alt, squash-merge to main
3. Create fresh branch `agent/claude/sprint-12-merge-choreography` from updated main

## Scope

### S12-01: Merge Choreography TreeView

**Spec:** sec-11 "Weeks 2-4" — "VS Code native TreeView (AuxiliaryBar). Shows worktrees in REVIEW state, conflict risk score, approve/reject actions."

**Implementation:**
- New `src/merge-tree.ts` with `TreeDataProvider`
- Register in `package.json` under AuxiliaryBar views
- Tree structure:
  - Root: one node per project with active merge queue entries
  - Children: one node per slot in REVIEW or MERGE_QUEUE status
  - Each node: slot ID, branch name, agent, status badge
- Conflict risk indicator: heuristic (file count + concurrent REVIEW slots) — Phase 4 for AST scoring
- Inline actions: Approve → MERGE_QUEUE (`MoveTask`), Reject → WORKING (`MoveTask`), Pause
- Live updates: subscribe to StateCache, refresh on REVIEW/MERGE_QUEUE/RESOLVING changes
- Empty state: "No active merges" placeholder

### S12-02: Extension Settings Page

**Spec:** sec-11 "Weeks 5-8" — "Settings page (session.yaml path, socket path, theme)."

- `contributes.configuration` in `package.json`:
  - `nexode.sessionPath` (string, default `.nexode/session.yaml`)
  - `nexode.socketPath` (string, optional Unix socket)
  - `nexode.theme` (`'auto' | 'synapse-dark' | 'synapse-light'`, default `'auto'`)
  - `nexode.showStatusBar` (boolean, default `true`)
- Verify existing `nexode.daemonHost` / `nexode.daemonPort` settings work with live reload

### S12-03: Extension README + Onboarding

- `extensions/nexode-vscode/README.md`: overview, prerequisites, installation, configuration, features, commands, troubleshooting
- Optional stretch: `walkthroughs` contribution in `package.json`

### S12-04: Tests

- `test/merge-tree.test.ts`: tree structure, refresh, conflict risk calculation
- Settings validation tests
- Stretch: Tier 2 extension host test setup (R-011)

## Non-Goals for Sprint 12

- Chat Participant (`@nexode`) — Sprint 13
- Full AST-based conflict risk scoring — Phase 4
- Rich per-cell presentation (spark-lines, progress bars)
- Output ring buffer (Sprint 15 prep)
- QG-1 score gate integration

## Constraints

1. No Rust changes. TypeScript-only. `cargo test --workspace` must still pass.
2. Native TreeView for Merge Choreography (not a React webview) — per spec.
3. Maintain nonce-based CSP. No `'unsafe-inline'` in `style-src`.
4. TypeScript strict mode maintained.
5. D-012 compliance: column moves dispatch `MoveTask`, assignment uses `AssignTask`.

## Verification

```bash
cd extensions/nexode-vscode
npm install
npm run build
npm run build:webview
npm run check-types
npm test
cd ../..
cargo check --workspace
cargo test --workspace
```
