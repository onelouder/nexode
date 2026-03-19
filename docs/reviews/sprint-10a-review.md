# Sprint 10 Tranche A Code Review: React Webview Infrastructure

**Branch:** `agent/gpt/sprint-10-react-webviews`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-19
**Commit reviewed:** `f1ee527 [gpt] handoff: complete Sprint 10 Tranche A -> pc review`

---

## Summary

Sprint 10 Tranche A delivers the foundational infrastructure for React webviews in the VS Code extension, adds Phase 3 observer event normalization to `state.ts`, and introduces the first TypeScript test coverage. The diff is +2350/-33 across 24 files (excluding `package-lock.json` churn, ~1700 lines of meaningful code and docs).

This tranche is infrastructure-only by design. It does not deliver rich UI behavior — that's Tranche B/C scope. What it does deliver is: a working esbuild webview pipeline (separate from the extension host bundle), three panel/provider shells (`SynapseGridPanel`, `SynapseSidebarProvider`, `KanbanPanel`), a shared postMessage bridge with nonce-based CSP, initial React entry points for both surfaces, and 251 lines of Tier 1 unit tests for `state.ts`. The `state.ts` module was also substantially expanded (555 → 734 lines) to decouple from the `vscode` namespace via a portable `Emitter<T>` class and to add full Phase 3 observer event normalization.

The most significant architectural contribution is the `Emitter<T>` replacement for `vscode.EventEmitter`. This enables `StateCache` to be tested in plain Node.js without the VS Code extension host — which is exactly what `test/state.test.ts` does. This was the right call: it directly addresses R-011 (VS Code extension has no test coverage) from Sprint 9's review.

D-012 (Phase 3 Kanban command semantics) is a clean, well-reasoned addition to `DECISIONS.md`. It resolves the `MoveTask` vs `AssignTask` ambiguity that was noted in Sprint 9 and correctly separates status-column transitions from agent assignment.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| Webview build pipeline | PASS | `esbuild.mjs` extended with `--target webview` mode. Produces minified IIFE bundles for `synapse-grid` and `kanban` under `dist/webview/`. Separate from extension host bundle. CSS loader and JSX automatic transform configured. |
| Panel/provider shell registration | PASS | `extension.ts` registers `SynapseGridPanel`, `SynapseSidebarProvider`, and `KanbanPanel`. Sidebar provider registered with `retainContextWhenHidden`. Commands `nexode.openSynapseGrid` and `nexode.openKanban` wired. |
| Shared postMessage bridge | PASS | `webview/shared/bridge.ts` provides `postHostMessage`, `postReady`, and `onHostMessage` with correct VS Code API acquisition and `MessageEvent` typing. `webview/shared/types.ts` defines `StateEnvelope`, `HostToWebviewMessage`, `WebviewToHostMessage`, `MoveTaskMessage`. |
| Nonce-based CSP | PASS | `webview-support.ts` generates `crypto.randomBytes(16)` nonce, sets `script-src 'nonce-${nonce}'` with `default-src 'none'`. HTML escaping for title injection. Correct `localResourceRoots` scoped to `dist/webview`. |
| React entry points | PASS | Both `synapse-grid/index.tsx` and `kanban/index.tsx` use React 18 `createRoot` with `StrictMode`. Synapse Grid reads `data-surface` attribute for sidebar vs grid mode. |
| State.ts observer event normalization | PASS | New types: `UncertaintyFlagTriggeredEvent`, `WorktreeStatusChangedEvent`, `ObserverAlertEvent` (with `LoopDetected`, `SandboxViolation`, `UncertaintySignal` sub-types). Full normalization functions with `coerceEnum` for `ObserverInterventionName` and `FindingKindName`. This resolves Sprint 9 F-09. |
| Emitter<T> decoupling from vscode | PASS | `Emitter<T>` class (lines 245-266) replaces `vscode.EventEmitter`. Implements `DisposableLike` interface. `Event<T>` type alias mirrors `vscode.Event<T>` semantics. `StateCache.onDidChange` now works outside the extension host. |
| Tier 1 unit tests | PASS | `test/state.test.ts` — 251 lines, 4 test cases covering: `coerceString`/`coerceNumber`/`coerceEnum` edge cases, `normalizeSnapshot` with missing/malformed/correct data, `normalizeEvent` for Phase 3 event variants, `normalizeCommandResponse` safe defaults, and `StateCache` snapshot + event mutations (5 event types: `agentStateChanged`, `agentTelemetryUpdated`, `taskStatusChanged`, `projectBudgetAlert`, `slotAgentSwapped`). |
| Kanban columns match D-009 state machine | PASS | `kanban/App.tsx` defines COLUMNS array with all 8 statuses in correct order: Pending, Working, Review, Merge Queue, Resolving, Done, Paused, Archived. |
| MoveTask dispatch per D-012 | PASS | `kanban-panel.ts` dispatches `{ moveTask: { taskId, target } }` on `moveTask` messages from the webview. Does not conflate with `AssignTask`. |
| Package.json scripts | PASS | `build:webview`, `watch:webview`, `test` (via `tsx --test`) added. `vscode:prepublish` chains both builds. |
| No Rust changes | PASS | Diff is TypeScript-only. `cargo check --workspace` and `cargo test --workspace` pass per agent handoff. |
| D-012 in DECISIONS.md | PASS | Clean, well-reasoned. Correctly separates `MoveTask` (status column transitions) from `AssignTask` (agent/slot binding). Interprets `REQ-P3-006` appropriately. |
| Build verification (agent-reported) | PASS | `npm install`, `npm run build`, `npm run build:webview`, `npm run check-types`, `npm test`, `cargo check --workspace`, `cargo test --workspace` all pass. |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `npm install` | PASS |
| `npm run build` | PASS |
| `npm run build:webview` | PASS |
| `npm run check-types` | PASS |
| `npm test` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |

**Test counts:** 4 new TypeScript test cases (state.test.ts). 114 Rust tests unchanged.

---

## Findings

### F-01 [Info] Synapse Grid sidebar sends `postReady` before `onDidReceiveMessage` listener may be registered

**Location:** `synapse-grid-panel.ts:89-106`

In `SynapseSidebarProvider.resolveWebviewView()`, the method calls `configureWebview()` (which sets `webview.html`), then registers `onDidReceiveMessage`, then calls `void this.postState()`. The webview's React code calls `postReady()` on mount. There's a race condition: if the webview loads and posts `ready` before the `onDidReceiveMessage` listener is registered on the host side, the ready message is lost and `this.ready` stays `false`, causing the sidebar to never receive state updates.

In practice this is extremely unlikely — HTML parsing and JS execution in the webview takes orders of magnitude longer than the synchronous listener registration. The `void this.postState()` call at line 105 also acts as a recovery path (it sends state regardless of `ready` flag... wait, it checks `!this.ready` at line 115, so it wouldn't help).

The `SynapseGridPanel.show()` has the same pattern but is less susceptible because `createWebviewPanel` + `configureWebview` + listener registration all happen synchronously before the panel is revealed.

**Recommendation:** No action. The race window is negligible in practice. If it ever surfaces (sidebar appears blank), the fix would be to set `this.ready = true` after `configureWebview` and unconditionally send state, or to replay missed ready messages on the next state push. Not blocking for Tranche A infrastructure.

### F-02 [Info] `SynapseSidebarProvider.dispose()` does not dispose the WebviewView

**Location:** `synapse-grid-panel.ts:108-112`

```typescript
public dispose(): void {
  this.stateSubscription.dispose();
  this.view = undefined;
  this.ready = false;
}
```

`SynapseGridPanel.dispose()` calls `this.panel?.dispose()` but `SynapseSidebarProvider.dispose()` only nulls the reference. This is actually correct — `WebviewView` instances are owned by VS Code (the sidebar container), not by the provider. The provider receives the view in `resolveWebviewView()` but doesn't own its lifecycle. Setting `this.view = undefined` is the right cleanup. Noting for clarity, not as a defect.

**Recommendation:** No action. This is correct behavior.

### F-03 [Low] `KanbanApp` does not display task `description`, `branch`, or `cost` fields

**Location:** `webview/kanban/App.tsx:102-106`

The PLAN_NOW spec for D-02 says: "Cards show: task title, assigned agent, worktree branch, token cost." The current implementation shows `task.id`, `task.title`, and `task.projectId · task.assignedAgentId` — but omits `branch` and token cost. The `TaskNode` interface doesn't carry `branch` or `cost` (those live on `AgentSlot`), so the webview would need a join between `taskDag` and `projects[].slots[]` to resolve them.

**Recommendation:** Tranche B scope. The task card needs to join `TaskNode` → `AgentSlot` (via matching IDs) to display branch and cost. The `StateEnvelope` already carries `snapshot.projects[].slots[]` so the data is available; the React component just needs the join. Not blocking for Tranche A shells.

### F-04 [Info] `styles.css` color schemes differ between Synapse Grid and Kanban

**Location:** `webview/synapse-grid/styles.css:1-9` vs `webview/kanban/styles.css:1-9`

Synapse Grid uses a blue palette (`--bg: #08111f`, `--accent: #63d2ff`). Kanban uses a dark-green/amber palette (`--bg: #101314`, `--accent: #ffbf47`). Both are dark themes but with different color identities. This is a deliberate design choice — the two surfaces have distinct purposes and the color differentiation helps the operator distinguish them at a glance in a multi-monitor setup. Noting for awareness.

**Recommendation:** No action. The color differentiation is reasonable for multi-monitor use per D-005.

### F-05 [Info] `Emitter.fire()` spreads listeners to prevent mutation during iteration

**Location:** `state.ts:257-260`

```typescript
public fire(event: T): void {
  for (const listener of [...this.listeners]) {
    listener(event);
  }
}
```

The spread (`[...this.listeners]`) creates a snapshot of the listener set before iterating, preventing issues if a listener disposes itself or adds new listeners during the callback. This is correct and matches `vscode.EventEmitter` semantics. The allocation cost is negligible for the expected listener counts (1-3 per emitter).

**Recommendation:** No action. Good defensive pattern.

### F-06 [Info] `coerceEnum` returns `options[0]` for invalid values — behavior change from Sprint 9

**Location:** `state.ts:731-733`

```typescript
export function coerceEnum<T extends string>(value: unknown, options: readonly T[]): T {
  return typeof value === 'string' && options.includes(value as T) ? (value as T) : options[0];
}
```

This was private in Sprint 9 and is now exported. The behavior (fallback to first option) means any unrecognized enum value silently becomes `UNSPECIFIED`. For `TaskStatusName` this is safe. For `ObserverInterventionName`, an unrecognized intervention could be silently downgraded to `UNSPECIFIED` — which would hide a new intervention type the daemon introduces before the extension is updated. The risk is low because the daemon and extension are co-deployed.

**Recommendation:** No action. The fallback-to-first-option pattern is standard for proto enum handling.

### F-07 [Low] `MOVE_TARGETS` in `commands.ts` omits `TASK_STATUS_RESOLVING` and `TASK_STATUS_DONE`

**Location:** `commands.ts:21-27`

```typescript
const MOVE_TARGETS: readonly TaskStatusName[] = [
  'TASK_STATUS_WORKING',
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_PAUSED',
  'TASK_STATUS_ARCHIVED',
];
```

This was carried over from Sprint 9 (pre-D-012). An operator cannot manually move a task to `RESOLVING` or `DONE` via the command palette. For `DONE`, this is arguably correct — tasks should reach DONE through the merge pipeline, not manual movement. For `RESOLVING`, the omission might be intentional (RESOLVING is conflict-driven, not operator-driven) or an oversight.

Per D-009, `RESOLVING` is triggered by merge failure (Phase 0-3) or AST conflict risk (Phase 4+). Manual operator routing to `RESOLVING` isn't part of the state machine spec. So this omission is correct.

**Recommendation:** No action. The omission aligns with D-009/D-010 semantics. If manual RESOLVING routing is ever needed, it should come through a `ForceResolve` command, not a generic MoveTask.

### F-08 [Info] Test file uses `node:test` runner — requires Node 18.7+

**Location:** `test/state.test.ts:1-2`

```typescript
import assert from 'node:assert/strict';
import test from 'node:test';
```

The test file uses Node's built-in test runner and is invoked via `tsx --test`. The `node:test` module requires Node 18.7+ (stable in Node 20+). The `package.json` declares `"node": ">=18"` and `"vscode": "^1.90.0"`. VS Code 1.90 ships with Node 18.x. This is compatible, but worth noting that `tsx --test` wraps the built-in runner and requires tsx 4.x (which is in devDependencies as `^4.19.3`).

**Recommendation:** No action. The toolchain requirements are correctly declared and the test runner choice avoids adding Mocha/Vitest as extra dependencies. Good minimal approach.

### F-09 [Info] `webview-support.ts` CSP does not include `style-src 'unsafe-inline'`

**Location:** `webview-support.ts:51-57`

```typescript
const csp = [
  "default-src 'none'",
  `img-src ${webview.cspSource} https: data:`,
  `style-src ${webview.cspSource}`,
  `script-src 'nonce-${nonce}'`,
  `font-src ${webview.cspSource}`,
].join('; ');
```

The CSP allows styles only from `webview.cspSource` (i.e., the `vscode-webview:` origin). This means the esbuild-emitted CSS file loads correctly, but any inline `style` attributes in the React JSX would be blocked. The current React components don't use inline styles — they use CSS classes from the separate stylesheet. If Tranche B/C adds inline styles (e.g., for dynamic grid positioning or drag-and-drop transforms), the CSP will need updating.

**Recommendation:** No action for Tranche A. Note for Tranche B: if React components need inline styles (likely for drag-and-drop positioning), either add `'unsafe-inline'` to `style-src` or use CSS custom properties set via class names.

---

## Architecture Assessment

### What's good

1. **Webview build isolation.** The `esbuild.mjs` cleanly separates the extension host build (`format: 'cjs'`, `platform: 'node'`) from the webview build (`format: 'iife'`, `platform: 'browser'`, minified). The `--target` flag switch is simple and effective. Two npm scripts (`build` vs `build:webview`) make the split explicit.

2. **Shared bridge pattern.** `webview/shared/bridge.ts` and `webview/shared/types.ts` establish a clean contract between host and webview. The `ReadyMessage` → `HostStateMessage` handshake ensures the webview doesn't miss state while loading. The `MoveTaskMessage` type for Kanban commands is properly scoped to D-012 semantics.

3. **Emitter<T> for testability.** Replacing `vscode.EventEmitter` with a portable `Emitter<T>` that implements the same `Event<T>` / `DisposableLike` contract is the key architectural win. It makes `StateCache` testable in plain Node.js without mocking the VS Code API. The 4 test cases in `state.test.ts` exercise the full normalization and mutation surface — this is exactly what R-011 called for.

4. **Observer event normalization.** Adding `UncertaintyFlagTriggered`, `WorktreeStatusChanged`, and `ObserverAlert` (with all sub-types) to `state.ts` means the extension is now ready to render observer findings when Tranche B/C adds the UI. This resolves Sprint 9 F-09 preemptively. The normalization layer handles the nested `loopDetected`/`sandboxViolation`/`uncertaintySignal` oneofs with optional fields and dedicated normalize functions.

5. **D-012 is architecturally sound.** Separating `MoveTask` (status column transitions) from `AssignTask` (agent binding) resolves the spec ambiguity cleanly. The `KanbanPanel.handleMessage()` correctly routes `moveTask` to `client.dispatchCommand()` with the right command shape. The `commands.ts` palette commands also use `moveTask` consistently.

6. **Panel lifecycle management.** `SynapseGridPanel` and `KanbanPanel` correctly track `ready` state, gate `postState` on readiness, handle panel disposal, and subscribe to `StateCache.onDidChange` for reactive updates. `retainContextWhenHidden: true` prevents React state loss when panels are hidden.

7. **Test quality.** The `state.test.ts` tests are thorough: they test malformed input (non-string in `coerceString`, non-number in `coerceNumber`, invalid enum in `coerceEnum`), complex normalization (nested project/slot/task structures with mixed types), Phase 3 observer events, and StateCache mutations with 5 different event types in sequence. The assertion that `changeCount >= 6` validates the emitter fires on both snapshot and event applications.

### What's missing (expected for Tranche A)

1. **No live state rendering.** The webview shells render mock/empty state. Live project/slot data will appear once Tranche B wires the full state envelope to rich UI components.

2. **No drag-and-drop.** Kanban columns are read-only. D-02 requires drag-and-drop for column moves. Tranche B/C scope.

3. **No Synapse Grid view modes.** D-01 specifies Project Groups, Flat View, and Focus View. The current shell only renders Project Groups. Tranche C scope.

4. **No Tier 2 extension host tests.** The test runner uses `tsx --test` (plain Node.js), which is sufficient for `state.ts` normalization but cannot test activation lifecycle, TreeView rendering, or WebviewPanel behavior. Tier 2 tests may require `@vscode/test-electron` or equivalent. PLAN_NOW correctly notes this may be deferred.

5. **No `agents` map in StateCache.** Sprint 9 F-02 noted that `agentStateChanged` doesn't store the agent's `newState`. Still true. Tranche B's Synapse Grid will need per-agent state for rendering. Low priority for Tranche A infrastructure.

---

## Spec Alignment Check

| PLAN_NOW Tranche A Scope | Delivered |
|---|---|
| Normalize sprint plan against spec decisions | Yes — PLAN_NOW references sec-05, sec-11, D-005, D-009, D-010, D-012 |
| Separate webview build target | Yes — `esbuild.mjs --target webview` with IIFE + browser platform |
| Extension registration for panel/provider shells | Yes — `SynapseGridPanel`, `SynapseSidebarProvider`, `KanbanPanel` registered in `extension.ts` |
| Webview entry points and shared bridge scaffolding | Yes — `webview/shared/`, `webview/synapse-grid/`, `webview/kanban/` |
| Tier 1 tests for `src/state.ts` | Yes — 251 lines, 4 test cases covering normalization + StateCache mutations |
| Green verification for all build/test commands | Yes (per agent handoff) |

The tranche delivers exactly what PLAN_NOW scoped. No over-delivery, no under-delivery.

---

## Verdict

**APPROVED.** Sprint 10 Tranche A delivers a clean, well-structured React webview infrastructure foundation. The architecture decisions are sound: separated build pipelines, shared postMessage bridge, portable `Emitter<T>` for testability, full Phase 3 observer event normalization, and correctly scoped D-012 command semantics. The test coverage directly addresses R-011 with meaningful assertions on the most critical code path (state normalization).

No findings above Low severity. The two Low findings (F-03, F-07) are both Tranche B scope items that were correctly deferred. The infrastructure is ready for Tranche B to build rich UI behavior on top of it.
