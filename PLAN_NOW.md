# PLAN_NOW.md — Sprint 10 Tranche C: Webview Polish + View Modes

> Owner: gpt (Codex)
> Reviewer: pc (Perplexity Computer)
> Status: ready for gpt to claim
> Spec reference: master-spec section 11 "Weeks 2-4: Multi-Monitor React Webviews"
> Previous tranches: Tranche A — PR #22, commit `4bfe2ff`; Tranche B — PR #23, commit `9b1a8a8`

## Objective

Add Synapse Grid view mode switching, extract duplicated webview utilities, and render observer alerts in the webview surfaces. After this tranche, the Synapse Grid supports all three view modes from the spec, shared formatting code is deduplicated, and observer findings are visible in the UI.

## Starting Point

Tranche B shipped:
- Live rendering in both Synapse Grid and Macro Kanban via `view-models.ts` join layer
- `StateCache` agent tracking with `AgentPresence`, selectors, and seed preservation
- HTML5 drag-and-drop Kanban column moves via MoveTask dispatch
- `SlotCard` component in Synapse Grid with status/agent/mode pills
- Sidebar compressed slot list with pills and token counts
- Metric headers in both surfaces showing agents, tokens, session cost
- Nonce-based CSP intact (class-based drag/drop styling)

The surfaces currently lack:
- View mode switching (only Project Groups view exists)
- Shared formatter utilities (6 functions duplicated across both React apps)
- Observer alert display (normalization exists in state.ts but is not rendered)
- Rich per-cell presentation (spark-lines, progress bars) — stretch goal

## Scope

### C-01: Synapse Grid view modes

**What:** Add Flat View and Focus View to the Synapse Grid, with a mode switcher.

The spec (sec-11) calls for three view modes:
- **Project Groups** (current): slots grouped by project, one card per slot
- **Flat View**: all slots in a single ungrouped list, sorted by status or activity
- **Focus View**: filter to a single project, expanded card detail

Implementation:
- Add a `viewMode` state variable to `SynapseGridApp` (`'groups' | 'flat' | 'focus'`)
- Add a mode switcher UI element in the Synapse Grid header (tabs or dropdown)
- **Flat View**: render `slotCards` ungrouped as a flat grid, sorted by status priority (Working > Review > Merge Queue > Resolving > Pending > Paused > Archived > Done) or by most recent activity
- **Focus View**: add a project selector. When a project is selected, render only that project's slots with expanded detail (show description, dependencies, full agent history if available)
- Sidebar always uses the compressed slot list regardless of grid view mode

### C-02: Shared webview formatter extraction

**What:** Deduplicate utility functions from both React webview apps.

Tranche B review F-01 identified 6 functions duplicated verbatim across `webview/synapse-grid/App.tsx` and `webview/kanban/App.tsx`:
- `formatCurrency(value: number): string`
- `formatCount(value: number): string`
- `toTitleWords(value: string): string`
- `formatAgentState(state: string): string`
- `statusTone(status: TaskStatusName): string`
- `agentTone(state: string): string`

Additional functions only in Synapse Grid that should also be shared:
- `formatStatus(status: string): string`
- `formatMode(mode: string): string`

Extract all to `webview/shared/format.ts`. Update both React apps to import from the shared module. Add a test file `test/format.test.ts` for the pure formatter functions.

### C-03: Observer alert rendering

**What:** Display observer findings (loop detection, uncertainty flags, sandbox violations) in the webview surfaces.

Tranche A added full normalization for Phase 3 observer events in `state.ts`:
- `UncertaintyFlagTriggeredEvent`
- `WorktreeStatusChangedEvent`
- `ObserverAlertEvent` (with `LoopDetected`, `SandboxViolation`, `UncertaintySignal` sub-types)

These are normalized and stored in events but not surfaced in the webviews. Implementation:
- Add an `alerts: ObserverAlertEvent[]` field to `StateCache` (or a rolling buffer of recent alerts)
- Include alerts in `createStateMessage` and `StateEnvelope`
- Synapse Grid: show an alert badge on affected slot cards (e.g., a warning icon when an agent is loop-detected or paused due to uncertainty)
- Kanban: show an alert indicator on affected task cards
- Optional: a collapsible alert panel at the top of Synapse Grid showing recent observer findings with timestamps

### C-04: Test coverage expansion

**What:** Add tests for new Tranche C logic.

- Test shared formatter functions (formatCurrency, formatCount, toTitleWords, statusTone, agentTone)
- Test Flat View and Focus View sorting/filtering logic if extracted as utilities
- Test `buildKanbanCardModels` with `projectFilter = 'all'` (Tranche B review F-07)
- Goal: maintain and expand the Tier 1 test suite

## Non-Goals for Tranche C

- Rich per-cell presentation (spark-lines, progress bars) — only if time permits after C-01 through C-04
- Barrier-aware fan-out / webview acknowledgement — post-Sprint 10
- Chat Participant (`@nexode`) — Sprint 11+ scope
- Merge Choreography TreeView — Sprint 11+ scope
- Tier 2 extension host tests — deferred (R-011)

## Constraints

1. **No Rust changes.** TypeScript-only. `cargo test --workspace` must still pass (114 tests).
2. **Webview security.** Maintain nonce-based CSP. No `'unsafe-inline'` in `style-src`.
3. **Bundle size.** Keep React + dependencies under 500KB gzipped.
4. **TypeScript strict mode.** Do not relax `strict: true`.
5. **D-012 compliance.** Column moves dispatch `MoveTask`. Agent assignment uses `AssignTask`. Do not conflate.
6. **Shared code convention.** All new utility functions shared between surfaces go in `webview/shared/`. No new duplication.

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
2. Update `CHANGELOG.md` with Sprint 10 Tranche C entry
3. Commit to the working branch: `agent/gpt/sprint-10c-<descriptive-suffix>`
4. Update `HANDOFF.md` with completion status
5. Do NOT merge — pc will review and merge
