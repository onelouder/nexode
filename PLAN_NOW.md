# PLAN_NOW.md — Sprint 10 Tranche B: Webview Surface Shells

> Owner: gpt (Codex)
> Reviewer: pc (Perplexity Computer)
> Status: Ready for gpt to claim
> Spec reference: master-spec section 11 "Weeks 2-4: Multi-Monitor React Webviews"
> Previous tranche: Sprint 10 Tranche A — PR #22, commit `4bfe2ff`

## Objective

Build live state rendering into the Synapse Grid and Kanban webview shells. After this tranche, both surfaces display real daemon state and the Kanban supports column-move interactions.

## Starting Point

Tranche A shipped:
- Working webview build pipeline (`esbuild.mjs --target webview`)
- Panel/provider shells: `SynapseGridPanel`, `SynapseSidebarProvider`, `KanbanPanel`
- Shared postMessage bridge: `webview/shared/bridge.ts` + `types.ts`
- React 18 entry points with `StateEnvelope` consumption
- `state.ts` decoupled from VS Code namespace, Tier 1 tests present
- D-012 semantics: column moves use `MoveTask`, not `AssignTask`

The shells currently render basic project/slot/task data from `StateEnvelope` but lack:
- TaskNode → AgentSlot join (cards miss branch + cost)
- Interactive drag-and-drop for Kanban
- Synapse Grid view modes (Flat View, Focus View)
- Agent state tracking in StateCache

## Scope

### B-01: Synapse Grid live rendering

**What:** Make the Synapse Grid surface render live state correctly.

- Join `TaskNode` with `AgentSlot` via matching IDs to display status, branch, cost per cell
- Show agent state indicators per slot (requires adding `agents: Map<string, AgentStateName>` to `StateCache` — see Sprint 9 F-02)
- Sidebar mode should show a compressed slot list with status + agent + token count
- Metric header: update to show session cost, total tokens, agent count from `getAggregateMetrics()`

### B-02: Macro Kanban live rendering + column moves

**What:** Make the Kanban board interactive.

- Task cards must show: title, assigned agent, project, branch, token cost (requires TaskNode → AgentSlot join from the snapshot data)
- Implement drag-and-drop between columns. On drop, dispatch `{ type: 'moveTask', taskId, target }` via `postHostMessage`
- Project filter dropdown is already wired; verify it works with live data
- All 8 columns per D-009: Pending, Working, Review, Merge Queue, Resolving, Done, Paused, Archived

**CSP note from review:** If drag-and-drop transforms require inline `style` attributes, update the CSP in `webview-support.ts` to add `'unsafe-inline'` to `style-src`. Or use CSS custom properties / classes to avoid inline styles entirely (preferred).

### B-03: StateCache agent state tracking

**What:** Add per-agent state tracking to `StateCache`.

- Add `private agents: Map<string, { state: AgentStateName; slotId: string }>` to `StateCache`
- Update `applyEvent` to populate from `agentStateChanged` events
- Expose `getAgentState(agentId)` and `getAgentsBySlot(slotId)` methods
- Include agent state in `getSnapshot()` return if needed by webviews
- Add Tier 1 tests for the new agent tracking logic

### B-04: Test coverage expansion

**What:** Add tests for new Tranche B logic.

- Test TaskNode → AgentSlot join logic (if extracted as a utility)
- Test agent state tracking in StateCache
- Test `MoveTaskMessage` dispatch from Kanban panel
- Goal: maintain and expand the Tier 1 test suite that Tranche A established

## Non-Goals for Tranche B

- Synapse Grid view mode switcher (Project Groups / Flat View / Focus View) — Tranche C
- Rich per-cell presentation (spark-lines, progress bars) — Tranche C
- Barrier-aware fan-out / webview acknowledgement — Tranche C
- Tier 2 extension host tests — deferred
- Chat Participant (`@nexode`) — Sprint 10+ scope
- Merge Choreography TreeView — Sprint 10+ scope

## Constraints

1. **No Rust changes.** TypeScript-only. `cargo test --workspace` must still pass (114 tests).
2. **Webview security.** Maintain nonce-based CSP. If inline styles needed for drag-and-drop, update CSP narrowly.
3. **Bundle size.** Keep React + dependencies under 500KB gzipped.
4. **TypeScript strict mode.** Do not relax `strict: true`.
5. **D-012 compliance.** Column moves dispatch `MoveTask`. Agent assignment uses `AssignTask`. Do not conflate.

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
2. Update `CHANGELOG.md` with Sprint 10 Tranche B entry
3. Commit to the same branch: `agent/gpt/sprint-10-react-webviews` (or a new branch if preferred)
4. Update `HANDOFF.md` with completion status
5. Do NOT merge — pc will review and merge
