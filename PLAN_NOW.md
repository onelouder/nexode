# PLAN_NOW.md — Sprint 10: React Webviews + Extension Tests

> Owner: gpt (Codex)
> Reviewer: pc (Perplexity Computer)
> Status: in progress on `agent/gpt/sprint-10-react-webviews` (Tranche A: plan normalization + plumbing)
> Spec reference: master-spec section 11 "Weeks 2-4: Multi-Monitor React Webviews"
> Previous sprint: Sprint 9 (VS Code Extension Scaffold) — PR #21, commit `0c8cee4`

## Objective

Build the two React WebviewPanels (Synapse Grid, Macro Kanban) and add test coverage for the extension. This covers master-spec section 11 "Weeks 2-4" plus the R-011 test gap identified in Sprint 9 review.

## Checkpoint

Sprint 10 is active, but the codebase is still between scaffold and implementation. The only local WIP is extension-side plumbing in `extensions/nexode-vscode/`; it has not yet produced a runnable webview or test harness.

## Normalized Scope

Sprint 10 implementation is normalized to these authoritative references:

- `sec-05-synapse-telemetry-grid`
- `sec-05-macro-kanban-board-task-queue`
- `sec-11-week-1-grpc-bridge-state-cache`
- `sec-11-weeks-2-4-multi-monitor-react-webviews`
- `REQ-P3-004`, `REQ-P3-005`, `REQ-P3-006`, `REQ-UX-011`, `REQ-UX-012`, `REQ-UX-013`, `REQ-UX-015`
- D-005 (manual multi-monitor in Phase 3)
- D-009 (Kanban state machine uses `MERGE_QUEUE` and `RESOLVING`)
- D-010 (`RESOLVING` is present in Phase 3, with Git-conflict triggering now and AST-triggering deferred to Phase 4)

Working interpretation for Sprint 10:

1. The Synapse Grid and Macro Kanban remain Sprint 10 targets, but the sprint is delivered in implementation tranches instead of one oversized drop.
2. Phase 3 multi-monitor behavior is manual only: panels may be popped out and dragged by the operator; there is no automatic monitor routing in Sprint 10.
3. Kanban columns must match the accepted task state machine: `Pending`, `Working`, `Review`, `Merge Queue`, `Resolving`, `Done`, `Paused`, `Archived`.
4. Column movement in the Kanban board dispatches `MoveTask`. `AssignTask` remains the command for explicit agent/slot assignment, not generic status-column changes.
5. The webview transport must respect the Phase 3 EventBus direction: extension host owns `StateCache`, batches barrier-related updates, and fans out state to webviews over `postMessage`.
6. Sprint 10 is not complete until the extension has both webview surfaces and at least Tier 1 TypeScript test coverage for `state.ts`.

## Implementation Tranches

### Tranche A (active now): Planning + Plumbing + Test Foundation

- normalize the sprint plan against the accepted spec decisions
- land a separate webview build target and basic extension registration for panel/provider shells
- add missing webview entry points and shared bridge scaffolding
- add Tier 1 tests for `src/state.ts`
- restore green verification for `npm run build`, `npm run build:webview`, `npm run check-types`, and `npm test`

### Tranche B: Webview Surface Shells

- implement the Synapse Grid `WebviewPanel` and sidebar `WebviewViewProvider`
- implement the Macro Kanban `WebviewPanel`
- render live state snapshots from `StateCache` in both surfaces
- support the normalized Kanban columns, including `RESOLVING`

### Tranche C: Rich UI Behavior

- Synapse Grid layout modes: Project Groups, Flat View, Focus View
- richer per-cell presentation and sidebar compression
- Kanban drag/drop interactions and project filtering
- barrier-aware fan-out and webview acknowledgement handling consistent with `REQ-P3-004`

## Deliverables

### D-01: Synapse Grid WebviewPanel

The primary agent monitoring surface. A React app rendered in a VS Code WebviewPanel.

**Requirements (from spec section 5 + section 11):**
- Subscribe to `StateCache.onDidChange` via postMessage bridge
- Render project groups with per-cell agent cards
- Three view modes: Project Groups (default), Flat View, Focus View
- Per-cell display: agent ID, task name, status indicator (color-coded), token count, cost
- Sidebar mode: compressed vertical list in the VS Code sidebar
- Maximized mode is a normal `WebviewPanel`; multi-monitor support is manual per D-005

**Implementation guidance:**
- Create `extensions/nexode-vscode/webview/` directory for React source
- Use a lightweight React setup (React 18 + esbuild or vite for webview bundling)
- The WebviewPanel communicates with the extension host via `postMessage`/`onDidReceiveMessage`
- The extension host sends state snapshots to the webview on each `StateCache.onDidChange`
- Keep the webview bundle separate from the main extension bundle
- Full completion target remains `REQ-P3-005`; Tranche A only needs the shell and bridge in place

### D-02: Macro Kanban WebviewPanel

The task/DAG management surface. A React app in a second WebviewPanel.

**Requirements (from spec section 5 + section 11):**
- Full-screen DAG Kanban with columns: Pending, Working, Review, Merge Queue, Resolving, Done, Paused, Archived
- Cards show: task title, assigned agent, worktree branch, token cost
- Column drag-and-drop moves tasks between task-status columns via `MoveTask`
- Explicit task-to-agent assignment, when present, uses `AssignTask`
- Project filtering via dropdown

**Implementation guidance:**
- Reuse the postMessage bridge from D-01
- Full sprint target should satisfy `REQ-P3-006`'s DAG/assignment surface expectations, but Tranche A/B may begin with a status-column board before richer DAG behavior lands
- On status-column drop, send `{ type: 'moveTask', taskId, target }` to the extension host
- Extension host dispatches via `DaemonClient.dispatchCommand()`

### D-03: Extension Test Harness (R-011)

Add test coverage for the TypeScript extension. Two tiers:

**Tier 1: Unit tests for `state.ts` normalization (no VS Code dependency)**
- Test `normalizeSnapshot` with missing/malformed/correct data
- Test `normalizeEvent` for each event type
- Test `normalizeCommandResponse` edge cases
- Test `coerceString`, `coerceNumber`, `coerceEnum` edge cases
- Test `StateCache.applySnapshot` and `applyEvent` state mutations
- Include the Phase 3 observer/alert event shapes already present in `hypervisor.proto`
- Runner: plain `mocha` or `vitest` (no VS Code extension host needed)

**Tier 2: Extension host integration tests (VS Code test runner)**
- Test activation lifecycle (`activate` → `deactivate`)
- Test TreeView renders projects/slots from a mock state
- Test StatusBar renders connection states
- Test command registration
- Runner: `@vscode/test-electron` + mocha
- Note: if Cursor CLI cannot run `vsce test`, Tier 2 may be deferred. Tier 1 is non-negotiable.

### D-04: Observer Alert Wiring (stretch)

Wire the remaining `HypervisorEvent` types to the extension:
- Add `UncertaintyFlagTriggered`, `WorktreeStatusChanged`, `ObserverAlert` to the TypeScript event model
- Show observer alerts as VS Code notifications (`vscode.window.showWarningMessage`)
- Add observer alert entries to the TreeView or a new "Alerts" view

This is a stretch goal. If time is constrained, skip D-04 and focus on D-01/D-02/D-03.

## Non-Goals For Tranche A

- automatic monitor assignment or any Phase 4 monitor orchestration
- full `react-grid-layout` / `reactflow` performance tuning before the shells compile
- VS Code Chat Participant work (`sec-11-weeks-5-6-native-vscode-integrations`)
- Merge Choreography TreeView implementation
- claiming Sprint 10 complete without a passing webview build and Tier 1 tests

## Constraints

1. **No Rust changes.** This sprint is TypeScript-only. `cargo test --workspace` must still pass (114 tests).
2. **Webview security.** Use `nonce`-based CSP in WebviewPanels. No inline scripts. Follow VS Code Webview API security guidelines.
3. **Bundle size.** Keep React + dependencies under 500KB gzipped. The extension should activate in <500ms on a cold start.
4. **TypeScript strict mode.** `tsconfig.json` already has `strict: true`. Do not relax it.
5. **esbuild for main extension, separate bundler for webviews.** The main `esbuild.mjs` bundles the extension host code. Add a second build step for webview React bundles (can be esbuild or vite).

## Verification

Before handing off, run:
```
cd extensions/nexode-vscode
npm install
npm run build          # Extension host bundle
npm run build:webview  # Webview React bundles (new)
npm run check-types    # TypeScript strict
npm run test           # Unit tests (new, Tier 1)
cd ../..
cargo check --workspace
cargo test --workspace  # Must still be 114+ tests
```

## File structure (expected after sprint)

```
extensions/nexode-vscode/
├── src/
│   ├── extension.ts          (updated: register webview panels)
│   ├── daemon-client.ts      (unchanged)
│   ├── state.ts              (updated: add observer event types)
│   ├── slot-tree-provider.ts (unchanged)
│   ├── status-bar.ts         (unchanged)
│   ├── commands.ts           (updated: register webview commands)
│   ├── synapse-grid-panel.ts (new: WebviewPanel for Synapse Grid)
│   └── kanban-panel.ts       (new: WebviewPanel for Kanban)
├── webview/
│   ├── synapse-grid/         (new: React app)
│   │   ├── index.tsx
│   │   ├── App.tsx
│   │   └── components/
│   ├── kanban/               (new: React app)
│   │   ├── index.tsx
│   │   ├── App.tsx
│   │   └── components/
│   └── shared/               (new: shared types, bridge)
│       ├── types.ts
│       └── bridge.ts
├── test/
│   ├── state.test.ts         (new: Tier 1 unit tests)
│   └── extension.test.ts     (new: Tier 2 integration tests, may be deferred)
├── proto/
├── resources/
├── package.json              (updated: new scripts, new dependencies)
├── tsconfig.json
├── esbuild.mjs               (updated or split)
└── ...
```

## Handoff protocol

When complete:
1. Ensure all verification commands pass
2. Update `CHANGELOG.md` with Sprint 10 entry
3. Commit to a new branch: `agent/gpt/sprint-10-react-webviews`
4. Update `HANDOFF.md` with completion status
5. Do NOT merge — pc will review and merge
