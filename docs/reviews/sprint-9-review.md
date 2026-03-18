# Sprint 9 Code Review: VS Code Extension Scaffold

**Branch:** `agent/gpt/sprint-9-vscode-scaffold`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-18
**Commit reviewed:** `6231e88 [gpt] handoff: complete sprint 9 -> pc review`

---

## Summary

Sprint 9 introduces the first TypeScript component in the workspace: the `nexode-vscode` extension under `extensions/nexode-vscode/`. This is the Phase 3 Week 1 deliverable per master-spec section 11 — extension scaffold, gRPC client, Status Bar HUD — and it's a clean, well-structured implementation.

The diff is +2770/-125 across 17 files. The extension provides: a gRPC daemon client with exponential-backoff reconnect (`daemon-client.ts`, 361 lines), a local state cache with event-driven updates (`state.ts`, 555 lines), a TreeView slot browser (`slot-tree-provider.ts`, 170 lines), a Status Bar HUD (`status-bar.ts`, 51 lines), command palette integration for pause/resume/move (`commands.ts`, 166 lines), and a clean activation lifecycle (`extension.ts`, 70 lines). Build tooling uses esbuild for bundling, TypeScript strict mode, and runtime proto loading via `@grpc/proto-loader`.

The architecture mirrors the TUI approach from Sprint 5: connect to daemon, fetch full state snapshot, subscribe to event stream, apply incremental updates to a local state cache, and render from the cache. The `StateCache` class is the extension's equivalent of the TUI's `AppState` — it applies snapshots and events, fires change notifications via `vscode.EventEmitter`, and exposes query methods for the UI layers. This is the right decomposition.

The most architecturally significant piece is the normalization layer in `state.ts` (lines 330-554). Because `@grpc/proto-loader` delivers raw `Record<string, unknown>` objects (not generated stubs), every field must be coerced from unknown types. The normalization functions (`normalizeSnapshot`, `normalizeEvent`, `normalizeCommandResponse`) handle this with a clean pattern: `coerceString`, `coerceNumber`, `coerceEnum` helpers that return safe defaults for missing/malformed data. This is defensive, correct, and avoids the code-generation step that would add build complexity.

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| Extension scaffold with package.json, tsconfig, esbuild | PASS | `package.json` registers commands, views, activity bar, configuration. `tsconfig.json` uses strict mode. `esbuild.mjs` bundles to `dist/extension.js` with externals for `vscode` and `@grpc/*`. |
| gRPC client connects to daemon | PASS | `DaemonClient` in `daemon-client.ts` — runtime proto loading, `waitForReady` with 5s timeout, `GetFullState` snapshot fetch, `SubscribeEvents` stream, generation-based connection tracking. |
| Event stream subscription with state mirror | PASS | `StateCache.applySnapshot()` and `applyEvent()` in `state.ts`. Handles `AgentStateChanged`, `AgentTelemetryUpdated`, `TaskStatusChanged`, `ProjectBudgetAlert`, `SlotAgentSwapped`. Fires `onDidChange` after each mutation. |
| Auto-reconnect with exponential backoff | PASS | `handleDisconnect()` in `daemon-client.ts:229-252`. 2s initial, 2x backoff, 30s max. Status transitions: connected → disconnected → reconnecting. Generation counter prevents stale connection callbacks. |
| TreeView slot browser | PASS | `SlotTreeProvider` in `slot-tree-provider.ts`. Two-level tree: projects → slots. Color-coded status icons via `statusColor()`. Markdown tooltip with slot metadata. 100ms debounced refresh. |
| Status Bar HUD | PASS | `NexodeStatusBar` in `status-bar.ts`. Shows connection state, agent count, token count. Tooltip shows session cost/budget. Click focuses slot view. Uses codicons (`$(plug)`, `$(sync~spin)`, `$(debug-disconnect)`). |
| Command palette (pause, resume, move) | PASS | `registerCommands()` in `commands.ts`. `nexode.pauseSlot`, `nexode.resumeSlot`, `nexode.moveTask`, `nexode.focusSlots`. QuickPick selectors filter by task status. `dispatchWithFeedback` shows error/success notifications. |
| Configuration settings | PASS | `nexode.daemonHost` (string, default `localhost`) and `nexode.daemonPort` (number, default 50051). `onDidChangeConfiguration` triggers `client.updateEndpoint()`. |
| Proto copy for runtime loading | PASS | `proto/hypervisor.proto` matches the canonical proto. Loaded at activation via `context.asAbsolutePath()`. |
| Build passes | PASS (per agent handoff) | `npm install`, `npm run build`, `npm run check-types`, `cargo check --workspace`, `cargo test --workspace` all pass. |
| No VS Code extension host testing | NOTED | Agent correctly noted Cursor CLI cannot launch `vsce`-based extension host tests. Residual risk: runtime activation, TreeView rendering, and QuickPick behavior are untested. Acceptable for scaffold phase. |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `npm install` | PASS |
| `npm run build` | PASS |
| `npm run check-types` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |

**Test counts:** No new Rust tests (no Rust changes). No TS tests yet (extension host test harness is Sprint 10+ scope).

---

## Findings

### F-01 [Info] `getTaskStatusForSlot` assumes task ID equals slot ID

**Location:** `state.ts:276-278`

```typescript
public getTaskStatusForSlot(slotId: string): TaskStatusName {
  return this.getTaskById(slotId)?.status ?? 'TASK_STATUS_UNSPECIFIED';
}
```

This looks up a task using the slot ID as the task ID. This works if the daemon's domain model guarantees a 1:1 mapping where slot IDs and task IDs share the same namespace (which they currently do — the engine assigns tasks to slots by ID). If this invariant ever changes (e.g., a task gets reassigned to a different slot), this lookup would silently return UNSPECIFIED. The TUI doesn't make this assumption — it uses the slot's `task` field.

**Recommendation:** Low priority. Consider adding a `taskId` field to `AgentSlot` and looking up by that instead. Not blocking for scaffold.

### F-02 [Info] `applyEvent` does not handle `agentStateChanged` state field

**Location:** `state.ts:240-247`

The `agentStateChanged` handler populates `slot.currentAgentId` when the slot has no agent, but does not store the agent's `newState` anywhere in the local state. The TUI tracks per-agent state for rendering; the extension currently doesn't expose agent state to the TreeView or StatusBar. This is fine for the scaffold phase where the TreeView shows task status (not agent state), but will matter for the Synapse Grid in Sprint 10+.

**Recommendation:** No action for now. Sprint 10+ will need an `agents: Map<string, AgentStateName>` in StateCache.

### F-03 [Low] `coerceNumber` converts `bigint` to `Number` without range check

**Location:** `state.ts:540-541`

```typescript
if (typeof value === 'bigint') {
  return Number(value);
}
```

Proto `uint64` fields arrive as `number` when using `longs: Number` in proto-loader options (line 318 of `daemon-client.ts`). The `bigint` branch would only fire if the proto-loader config changed. If it did fire with a value > `Number.MAX_SAFE_INTEGER`, the conversion silently loses precision. For token counts or event sequences that could exceed 2^53, this would be incorrect.

**Recommendation:** Low priority. The proto-loader is configured with `longs: Number` which prevents this path from being reached. If you ever switch to `longs: String` or `longs: BigInt`, add a range check here. Not blocking.

### F-04 [Low] `dispatchCommand` guard checks `this.status.state` but `getFullState` does not

**Location:** `daemon-client.ts:138` vs `daemon-client.ts:129-135`

`dispatchCommand` checks `if (this.status.state !== 'connected')` and throws before attempting the RPC. `getFullState` goes straight to `requireClient()` which only checks if `this.client` is non-null. If the client object exists but the connection is in a degraded state (e.g., between disconnect and teardown), `getFullState` could attempt an RPC that fails with a less informative error.

**Recommendation:** Low priority. Add the same `status.state` guard to `getFullState`. Not blocking since `getFullState` is only called internally during `establishConnection` where the client was just created.

### F-05 [Info] `normalizeHost` strips protocol but not path components

**Location:** `daemon-client.ts:357-359`

```typescript
const trimmed = host.trim().replace(/^https?:\/\//, '').replace(/\/+$/, '');
```

Strips `http://` or `https://` prefix and trailing slashes, but if someone enters `http://localhost:50051/api`, the result would be `localhost:50051/api` — which would be passed as a gRPC address. Unlikely since the configuration description says "hostname or IP address", but worth noting.

**Recommendation:** No action. The configuration description is clear. Edge case is not worth additional parsing complexity.

### F-06 [Info] `focusSlotsView` uses empty catch for `.focus()` command

**Location:** `extension.ts:48-49`

```typescript
try {
  await vscode.commands.executeCommand('nexodeSlots.focus');
} catch {
  // Older builds may not expose the auto-generated focus command for contributed views.
}
```

The comment explains the intent. This is correct — `.focus()` command availability depends on VS Code version. The primary `workbench.view.extension.nexode` command (line 46) opens the sidebar; the `.focus()` is just a refinement. Good defensive coding.

**Recommendation:** No action.

### F-07 [Info] `commandId` uses `Date.now()` base36 — not globally unique

**Location:** `commands.ts:163-164`

```typescript
function commandId(): string {
  return `vscode-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}
```

This generates IDs like `vscode-l3kxz9c-a4b2c1`. The `Date.now()` portion gives millisecond resolution and `Math.random()` adds 6 base36 chars (~31 bits of entropy). Two rapid-fire commands within the same millisecond have a ~1 in 2 billion chance of colliding. Acceptable for operator-initiated commands which happen at human speed.

**Recommendation:** No action.

### F-08 [Info] `.vscodeignore` excludes `*.ts` at root — would exclude any future root-level .ts files

**Location:** `.vscodeignore:6`

```
*.ts
```

This glob excludes all `.ts` files from the packaged VSIX. Since source files live under `src/`, this mainly catches `tsconfig.json` (already excluded on line 4) and any hypothetical root-level `.ts` files. The `src/` directory is already excluded on line 2. The `*.ts` glob is redundant but harmless — it's a belt-and-suspenders exclusion.

**Recommendation:** No action.

### F-09 [Info] Proto copy includes event types not handled by extension

**Location:** `proto/hypervisor.proto:116-124` vs `state.ts:104-114`

The proto defines `UncertaintyFlagTriggered`, `WorktreeStatusChanged`, and `ObserverAlert` event types. The TypeScript `HypervisorEvent` interface only includes `agentStateChanged`, `agentTelemetryUpdated`, `taskStatusChanged`, `projectBudgetAlert`, and `slotAgentSwapped`. The proto-loader will deliver the missing fields as `undefined` and `applyEvent` will silently skip them. This is safe since the extension has no UI for observer alerts yet (that's Sprint 10+ scope), but means extension users won't see observer findings.

**Recommendation:** No action for scaffold. Track for Sprint 10+ when the extension needs observer alert display.

---

## Architecture Assessment

### What's good

1. **Clean module boundaries.** Each file has a single responsibility: `daemon-client.ts` owns transport, `state.ts` owns domain model + normalization, `slot-tree-provider.ts` owns the TreeView, `status-bar.ts` owns the HUD, `commands.ts` owns palette integration. No circular dependencies.

2. **Defensive normalization.** The `coerceString`/`coerceNumber`/`coerceEnum` pattern handles proto-loader's untyped output gracefully. Every field has a safe default. No `as any` casts in the normalization layer.

3. **Generation-based connection tracking.** The `generation` counter in `DaemonClient` (incremented in `establishConnection`) prevents stale callbacks from corrupting state after reconnects. This is the same pattern the daemon uses for slot agent tracking. Well understood.

4. **Lifecycle management.** Every resource implements `vscode.Disposable` and is registered in `context.subscriptions`. The `activeClient` module-level variable in `extension.ts` handles deactivation cleanly.

5. **Event-driven refresh.** The TreeView and StatusBar subscribe to `StateCache.onDidChange` and refresh reactively. The 100ms debounce in `SlotTreeProvider.scheduleRefresh()` prevents rapid-fire tree rebuilds during event bursts. Correct approach.

6. **Mirrors TUI architecture.** The extension follows the same connect → snapshot → stream → apply → render pattern established in Sprint 5. Code that worked there will work here.

### What's missing (expected for scaffold phase)

1. **No test coverage.** No VS Code extension host tests, no unit tests for normalization functions. The agent correctly noted that Cursor CLI cannot run `vsce test`. This is the highest-priority addition for Sprint 10.

2. **No observer alert display.** `UncertaintyFlagTriggered`, `WorktreeStatusChanged`, and `ObserverAlert` are not wired to any UI. Expected — the extension shows slots and status, not observer findings.

3. **No React Webviews.** The Synapse Grid and Macro Kanban (spec Weeks 2-4) are not present. Expected — those are Sprint 10+ scope.

4. **No Chat Participant.** The `@nexode` chat participant (spec Weeks 5-8) is not present. Expected — that's Sprint 10+ scope.

---

## Verdict

**APPROVED.** Sprint 9 delivers a complete Phase 3 Week 1 scaffold per master-spec section 11. The extension connects to the daemon via gRPC, maintains a local state mirror, renders a TreeView and Status Bar HUD, and supports command dispatch through the palette. Build tooling (esbuild, TypeScript strict mode, runtime proto loading) is production-grade. No findings above Low severity. The two Low findings (F-03, F-04) are edge cases that don't affect current operation.

The extension scaffold establishes the right architecture for Phase 3: daemon-first rendering shell with clean module boundaries. Sprint 10 should add React Webviews (Synapse Grid, Kanban), extension host tests, and observer alert wiring.
