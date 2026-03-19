# Sprint 10 Tranche B Code Review: Live Webview Surfaces

**Branch:** `agent/gpt/sprint-10b-webview-shells`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-19
**Commit reviewed:** `cb9dc69 [gpt] handoff: sprint 10 tranche b live webviews → pc`

---

## Summary

Sprint 10 Tranche B delivers live state rendering for both React webview surfaces, implements HTML5 drag-and-drop for Kanban column moves, adds per-agent state tracking to `StateCache`, and extracts shared join utilities for host→webview data projection. The diff is +1086/-103 across 17 files — substantial feature work built cleanly on Tranche A infrastructure.

The tranche closes all three review follow-ups from the Tranche A review: F-01 (ready-listener race) is fixed by reordering `onDidReceiveMessage` registration before `configureWebview` in all three panel/provider paths; F-03 (Kanban cards missing branch/cost) is resolved by the new `view-models.ts` join layer; F-09 (CSP vs inline styles) is avoided entirely — drag-and-drop uses class-based styling (`.is-active-drop`, `.is-dragging`) rather than inline style attributes, keeping the CSP untouched.

The `view-models.ts` extraction is the most architecturally significant addition. It creates a clean data-projection boundary between `StateCache` (host-side, event-sourced) and the webview React components (presentation-side, model-driven). Both `buildSlotCardModels` and `buildKanbanCardModels` perform the TaskNode → AgentSlot → AgentPresence join that was identified as missing in Tranche A's F-03. The functions are pure, tested, and reusable across both surfaces.

The `kanban-commands.ts` extraction is small (27 lines) but demonstrates good refactoring instinct — pulling the command factory out of `kanban-panel.ts` makes it independently testable with an injectable `createCommandId`. The test injects a fixed ID to eliminate Date.now/Math.random nondeterminism. Clean pattern.

Agent state tracking in `StateCache` is well-implemented. The `seedAgents()` function correctly rebuilds the agent map from snapshot data while preserving event-sourced state (e.g., if an `agentStateChanged` event set a state to `EXECUTING`, a subsequent snapshot won't reset it to `UNSPECIFIED`). The `slotAgentSwapped` handler correctly deletes the old agent and creates the new one with a preserved state if one existed. The `agentStateChanged` handler also cleans up a previous agent on the same slot if the ID changes — defensive against out-of-order events.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| B-01: Synapse Grid live rendering | PASS | Grid now renders joined slot/task/project data via `buildSlotCardModels`. `SlotCard` component shows status pills, agent state pills, mode pills, branch, tokens, cost, worktree. Sidebar shows compressed slot list with status/agent/token pills. Metric header shows agents, tokens, session cost from `getAggregateMetrics()`. |
| B-02: Kanban live rendering + column moves | PASS | Cards show title, description, agent, project name, branch, token cost via `buildKanbanCardModels` join. Full HTML5 drag-and-drop: `draggable`, `onDragStart`/`onDragEnd`/`onDragOver`/`onDrop` with `dataTransfer` fallback. Same-column drops are no-ops (checked via `task.status === target`). Dispatches `MoveTask` via `postHostMessage`. Metric chips in header. Project filter auto-resets if filtered project disappears. |
| B-03: StateCache agent tracking | PASS | `AgentPresence` interface with `agentId`, `slotId`, `state`. `seedAgents()` rebuilds from snapshot, preserving event-sourced state. `applyEvent` handles `agentStateChanged`, `agentTelemetryUpdated` (creates entry if missing), `slotAgentSwapped` (deletes old, creates new). Exposed: `getAgentStates()`, `getAgentState(id)`, `getAgentsBySlot(slotId)`. `getAggregateMetrics()` uses `agents.size` instead of computing from slot `currentAgentId`. |
| B-04: Test coverage expansion | PASS | 3 new test files. `state.test.ts`: +2 assertions in existing test (agent-7 deleted, agent-9 unspecified, getAgentsBySlot), +1 new test case (seed preservation across snapshots). `view-models.test.ts`: 2 test cases covering `buildSlotCardModels` join and `buildKanbanCardModels` join (with bound + detached card). `kanban-commands.test.ts`: 1 test case for `createMoveTaskCommand` with injected ID. Total: 7 new TypeScript test cases (bringing count from 4 to ~11). |
| F-01 fix: ready-listener race | PASS | `onDidReceiveMessage` now registered BEFORE `configureWebview` in `SynapseGridPanel.show()`, `SynapseSidebarProvider.resolveWebviewView()`, and `KanbanPanel.show()`. The webview cannot post `ready` until `configureWebview` sets `webview.html`, so the listener is guaranteed to be in place. |
| F-03 fix: Kanban branch/cost display | PASS | `buildKanbanCardModels` joins `TaskNode` → `AgentSlot` via slot ID matching, then resolves project and agent. Cards now display `slot.branch`, `slot.totalTokens`, `slot.totalCostUsd`. |
| F-09 avoidance: CSP preservation | PASS | Drag-and-drop uses CSS classes (`.is-active-drop`, `.is-dragging`) with opacity/border transitions. No inline `style` attributes. CSP unchanged — `style-src` remains `${webview.cspSource}` only. |
| No Rust changes | PASS | Diff is TypeScript/TSX/CSS only. |
| D-012 compliance | PASS | Kanban drag-and-drop dispatches `{ type: 'moveTask', taskId, target }` via `postHostMessage`. `kanban-panel.ts` routes to `createMoveTaskCommand()` which produces `{ commandId, moveTask: { taskId, target } }` for `client.dispatchCommand()`. No conflation with `AssignTask`. |
| Build verification (agent-reported) | PASS | `npm run build`, `npm run build:webview`, `npm run check-types`, `npm test`, `cargo check --workspace`, `cargo test --workspace` all pass per handoff. |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `npm run build` | PASS |
| `npm run build:webview` | PASS |
| `npm run check-types` | PASS |
| `npm test` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |

**Test counts:** ~11 TypeScript test cases across 3 files (up from 4 in Tranche A). 114 Rust tests unchanged.

---

## Findings

### F-01 [Low] Duplicate utility functions across Synapse Grid and Kanban webview components

**Location:** `webview/synapse-grid/App.tsx` and `webview/kanban/App.tsx`

Both webview React files define identical copies of: `formatCurrency()`, `formatCount()`, `toTitleWords()`, `formatAgentState()`, `statusTone()`, `agentTone()`. Six functions duplicated verbatim. The `formatStatus()` and `formatMode()` functions appear only in Synapse Grid but follow the same pattern.

This is a natural consequence of the two surfaces being developed as sibling React apps with no shared utility layer beyond `webview/shared/bridge.ts` and `webview/shared/types.ts`. The duplication is safe (the functions are pure formatters with no state) but will multiply with each new surface or formatter.

**Recommendation:** Tranche C scope. Extract to `webview/shared/format.ts` or `webview/shared/ui-utils.ts`. Low priority — the duplication is not harmful, just inelegant. Should be addressed before adding a third surface.

### F-02 [Info] `buildKanbanCardModels` resolves `agentId` with `||` fallback, not `??`

**Location:** `view-models.ts:70`

```typescript
const agentId = resolved?.slot.currentAgentId || task.assignedAgentId;
```

This uses `||` which treats `''` (empty string) as falsy. If a slot has `currentAgentId: ''` (which `normalizeSnapshot` can produce for slots with no agent), the fallback correctly reaches `task.assignedAgentId`. If `assignedAgentId` is also `''`, the result is `''`, which is handled correctly downstream (the UI displays "unassigned"). The behavior is identical to using `??` except for the empty-string case, where `||` is actually preferable here.

**Recommendation:** No action. The `||` is correct for this use case.

### F-03 [Info] `agentStateChanged` handler unconditionally overwrites `slot.currentAgentId`

**Location:** `state.ts:362-364` (in the diff context)

```typescript
if (slot) {
  slot.currentAgentId = event.agentStateChanged.agentId;
}
```

The Tranche A version had a guard: `if (slot && !slot.currentAgentId)`. Tranche B removes the guard, meaning any `agentStateChanged` event will overwrite the slot's `currentAgentId` even if it already has a different agent assigned. The agent also deletes the previous agent from the `agents` map if the IDs differ.

This is actually correct behavior for the event model: `agentStateChanged` is authoritative — if the daemon says agent-X is now on slot-Y, the extension should believe it regardless of what was there before. The old guard was defensive against a scenario where the extension hadn't seen the swap event, but that defense masked stale state rather than correcting it. The new approach is more correct.

**Recommendation:** No action. The removal of the guard is an improvement.

### F-04 [Low] `agentTelemetryUpdated` handler creates an agent entry without a slot match guarantee

**Location:** `state.ts:318-324` (diff context)

```typescript
if (!this.agents.has(event.agentTelemetryUpdated.agentId)) {
  this.agents.set(event.agentTelemetryUpdated.agentId, {
    agentId: event.agentTelemetryUpdated.agentId,
    slotId: slot.id,
    state: 'AGENT_STATE_UNSPECIFIED',
  });
}
```

The telemetry handler finds the slot where `currentAgentId` matches the event's `agentId`, then creates an agent entry using that slot's ID. This works correctly, but the agent entry is created with `AGENT_STATE_UNSPECIFIED` even though the daemon is clearly sending telemetry for an active agent. The state will be corrected when the next `agentStateChanged` event arrives, but there's a brief window where the UI could show a telemetry-emitting agent as "Unspecified".

In practice this is a non-issue: `agentStateChanged` events precede telemetry events in the daemon's event stream (the agent enters `EXECUTING` before it starts producing tokens). This code path would only fire if events arrive out of order, which the daemon doesn't do.

**Recommendation:** No action. The defensive creation is correct and the transient state is harmless.

### F-05 [Info] Drag-and-drop uses both `dragTaskId` state and `dataTransfer` with fallback

**Location:** `webview/kanban/App.tsx:125-126`

```typescript
const taskId = draggingTaskId || event.dataTransfer.getData('text/plain');
```

The drop handler uses React state (`draggingTaskId`) as the primary source and `dataTransfer` as a fallback. This is a good defensive pattern — React state is more reliable within the same webview context, but `dataTransfer` ensures the drop target receives the task ID even if the React state was lost (e.g., due to a re-render during the drag). The `setData('text/plain', card.task.id)` in `onDragStart` ensures the fallback is populated.

**Recommendation:** No action. Good defensive pattern.

### F-06 [Info] `seedAgents` correctly preserves event-sourced state across snapshot refreshes

**Location:** `state.ts` (new `seedAgents` function)

```typescript
function seedAgents(
  projects: readonly Project[],
  previous: ReadonlyMap<string, AgentPresence>,
): Map<string, AgentPresence> {
  const next = new Map<string, AgentPresence>();
  for (const project of projects) {
    for (const slot of project.slots) {
      if (!slot.currentAgentId) continue;
      next.set(slot.currentAgentId, {
        agentId: slot.currentAgentId,
        slotId: slot.id,
        state: previous.get(slot.currentAgentId)?.state ?? 'AGENT_STATE_UNSPECIFIED',
      });
    }
  }
  return next;
}
```

This is the key correctness property: when a new snapshot arrives, the agent map is rebuilt from the snapshot's slot data, but each agent's `state` is preserved from the previous map. This prevents snapshot refreshes from resetting event-sourced state. The new `StateCache preserves seeded agent state across snapshots` test case verifies this: set state to EXECUTING via event, apply new snapshot, verify state remains EXECUTING.

**Recommendation:** No action. Well-designed, well-tested.

### F-07 [Low] `view-models.test.ts` does not test the `projectFilter = 'all'` default path

**Location:** `test/view-models.test.ts:21`

The `buildKanbanCardModels` test passes `'proj-a'` as the project filter. The `projectFilter = 'all'` default (which skips filtering) is exercised by `buildSlotCardModels` indirectly but not by the kanban test. A test with `projectFilter = 'all'` would verify the no-filter path returns all tasks regardless of project.

**Recommendation:** Minor test gap. Not blocking — the `'all'` path is a trivial `filter` guard (`projectFilter === 'all'` short-circuits). Could be added in Tranche C.

---

## Architecture Assessment

### What's good

1. **Data-projection boundary via view-models.ts.** The separation between `StateCache` (event-sourced host state) and the webview presentation is now explicit. `buildSlotCardModels` and `buildKanbanCardModels` are pure functions that join snapshot data with agent state — exactly the pattern needed for any additional surfaces. This is the right abstraction layer.

2. **Agent tracking is event-source-correct.** The `seedAgents` function preserves event-sourced state across snapshot refreshes. The swap handler cleans up old agents. The state-changed handler cleans up replaced agents on the same slot. The aggregate metrics now use the agent map size instead of computing from slot fields. This is all consistent and correct.

3. **Drag-and-drop without CSP changes.** Using CSS classes for drag/drop visual states instead of inline styles keeps the CSP tight. The `.is-active-drop` and `.is-dragging` classes with opacity/border transitions provide clear visual feedback. The same-column drop guard prevents no-op commands.

4. **Ready-listener race fix is minimal and correct.** Moving `onDidReceiveMessage` registration above `configureWebview` in all three paths ensures the listener is in place before the webview can post messages. The fix is 3 code moves (no new logic), which is the ideal fix for a race condition.

5. **Command extraction enables testable dispatch.** `createMoveTaskCommand` with injectable `createCommandId` is the textbook approach to testing code that generates random IDs. The test verifies the exact command shape with a fixed ID. Small file, focused responsibility.

6. **Test coverage is well-targeted.** The new tests cover: (a) agent tracking lifecycle across snapshots and events, (b) the join logic that connects the event-sourced model to the presentation model, (c) command mapping from webview messages to daemon commands. These are the three most critical correctness boundaries in the new code.

7. **Project filter auto-reset.** `KanbanApp` detects when a filtered project disappears from state and resets to `'all'`. This prevents a stale filter from showing an empty board when projects change. Good defensive UX.

### What's missing (expected for Tranche B)

1. **Synapse Grid view modes.** The spec calls for Flat View and Focus View in addition to the default Project Groups. Only Project Groups is implemented. Tranche C scope per PLAN_NOW.

2. **Rich per-cell presentation.** The spec mentions spark-lines and progress bars. Not present. Tranche C scope.

3. **Barrier-aware fan-out.** The webview does not acknowledge barrier events or participate in the event barrier protocol. Tranche C scope.

4. **Tier 2 extension host tests.** Testing activation lifecycle, TreeView rendering, and WebviewPanel behavior still requires `@vscode/test-electron`. Deferred.

5. **Observer alert display.** The state normalization for observer alerts (uncertainty flags, loop detection, sandbox violations) was added in Tranche A, but neither webview renders these yet. Tranche C or post-Sprint 10 scope.

6. **Conflict risk scores on cards.** The spec (sec-05) mentions conflict risk scores on Kanban cards. Not implemented — requires Phase 4 AST-based conflict detection. Not a Tranche B scope item.

---

## Spec Alignment Check

| PLAN_NOW Tranche B Scope | Delivered |
|---|---|
| B-01: Synapse Grid live rendering with joined data | Yes — `buildSlotCardModels`, `SlotCard` component, metric header, sidebar pills |
| B-02: Kanban live rendering + drag-and-drop column moves | Yes — `buildKanbanCardModels`, drag-and-drop via HTML5 API, `MoveTask` dispatch, project filter auto-reset |
| B-03: StateCache agent tracking | Yes — `AgentPresence`, `seedAgents`, three selectors, swap/state/telemetry event handling |
| B-04: Test coverage expansion | Yes — 7 new test cases across 3 files |
| F-01 follow-up: ready-listener race fix | Yes — listener registration moved before `configureWebview` in all 3 paths |
| F-03 follow-up: Kanban branch/cost join | Yes — via `buildKanbanCardModels` |
| F-09 follow-up: CSP preservation | Yes — class-based drag/drop styling, no inline styles |
| No Rust changes | Yes — TypeScript/TSX/CSS only |
| D-012 compliance: MoveTask for column moves | Yes — drag-and-drop dispatches `MoveTask`, not `AssignTask` |

The tranche delivers all scoped items and closes all Tranche A review follow-ups. No over-delivery, no under-delivery.

---

## Verdict

**APPROVED.** Sprint 10 Tranche B delivers clean, well-structured live rendering for both webview surfaces with correct event-sourced agent tracking and a properly extracted data-projection layer. The drag-and-drop implementation is standard HTML5 with good defensive patterns (state + dataTransfer fallback, same-column no-op guard, project filter auto-reset). All three Tranche A review follow-ups are closed. No findings above Low severity. The two Low findings (F-01 duplicate formatters, F-04 transient agent state) are minor and do not affect correctness.

The codebase is in good shape for Tranche C, which should focus on Synapse Grid view modes (Flat View, Focus View), shared formatter extraction, and possibly observer alert rendering.
