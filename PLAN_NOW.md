# PLAN_NOW.md — Sprint 11: Foundation — Workspace Folders + Agent Output

> Owner: claude
> Reviewer: pc (Perplexity Computer)
> Status: in progress on `agent/claude/sprint-11-workspace-folders`
> Architecture reference: `docs/architecture/multiplexed-workspaces.md`
> Sprint prompt: `.agents/prompts/sprint-11-codex.md`
> Previous sprint: Sprint 10 Tranche C — complete on `agent/gpt/sprint-10c-view-modes`

## Objective

Introduce the foundation layer for the Multiplexed Native Workspace Architecture: worktree-as-workspace-folder (F-01) and agent output streaming (F-02). After this sprint, agent worktrees appear as VS Code workspace folders in Explorer (with SCM, intellisense, and editor tabs for free), and agent stdout/stderr streams to per-slot VS Code OutputChannels.

## Starting Point

Sprint 10 Tranche C shipped:
- Synapse Grid with 3 view modes (Project Groups, Flat, Focus)
- Shared webview formatters in `webview/shared/format.ts`
- Observer alert rendering in both webview surfaces
- 15 TypeScript tests, 114 Rust tests, all passing

The extension currently has no awareness of worktree filesystem paths (proto `AgentSlot` lacks `worktree_path`), no agent output streaming (raw lines are not published to gRPC), and no workspace folder management.

## Scope

### S11-01: Proto changes

Add to both proto copies (`crates/nexode-proto/proto/hypervisor.proto` and `extensions/nexode-vscode/proto/hypervisor.proto`):
- `string worktree_path = 10` on `AgentSlot`
- `repeated Worktree worktrees = 6` on `FullStateSnapshot`
- `AgentOutputLine` message (slot_id, agent_id, stream, line, timestamp_ms)
- `AgentOutputLine agent_output_line = 13` in `HypervisorEvent.oneof payload`

### S11-02: Daemon — worktree_path in snapshots

Populate `AgentSlot.worktree_path` and `FullStateSnapshot.worktrees` in `RuntimeState::snapshot()` from existing `SlotRuntime.worktree_path` data.

### S11-03: Daemon — output event publishing

In `engine/slots.rs`, publish `AgentOutputLine` events for each non-empty agent output line. Add 500-line output ring buffer to `SlotRuntime`. Increase broadcast buffer from 256 to 2048.

### S11-04: Extension — state normalization

Add `worktreePath: string` to `AgentSlot` interface in `state.ts`. Add `AgentOutputLine` interface. Update normalization functions. Output events must NOT trigger `StateCache.onDidChange`.

### S11-05: Extension — WorkspaceFolderManager

New `src/workspace-folder-manager.ts`: subscribes to state changes, reconciles workspace folders against slot worktree paths. Debounced 200ms batched updates. Adds folders for active slots, removes for DONE/ARCHIVED.

### S11-06: Extension — OutputChannelManager

New `src/output-channel-manager.ts`: creates per-slot OutputChannels lazily. Subscribes to `DaemonClient.onDidReceiveAgentOutput` bypass event (not StateCache). Disposes channels on slot completion.

### S11-07: Extension — DaemonClient output bypass

Add `onDidReceiveAgentOutput` event emitter to `DaemonClient`. Route `agentOutputLine` events directly to this emitter, bypassing StateCache.

### S11-08: Extension — wiring and commands

Wire WorkspaceFolderManager and OutputChannelManager in `extension.ts`. Register `nexode.showSlotOutput` and `nexode.resetWorkspaceFolders` commands. Update `package.json`.

### S11-09: Tests

- `test/workspace-folder-manager.test.ts` — reconciliation logic
- `test/output-channel-manager.test.ts` — channel lifecycle
- `test/state.test.ts` updates — worktreePath normalization, output event handling
- Rust: verify AgentOutputLine events in gRPC stream

## Non-Goals for Sprint 11

- File decorations — Sprint 12
- Diagnostic/Problems panel — Sprint 12
- Diff commands — Sprint 12
- Expanded webview messages — Sprint 12
- Lifecycle scripts — Sprint 13
- Failure re-routing — Sprint 13
- Comments API — Sprint 14
- Pre-merge gating — Sprint 14

## Constraints

1. All 114 Rust tests must continue to pass, plus new output event test(s)
2. All 15 TypeScript tests must continue to pass, plus new tests
3. TypeScript strict mode maintained
4. Proto is append-only; both copies must be identical
5. No webview changes in this sprint
6. Maintain nonce-based CSP

## Verification

```bash
cargo check --workspace
cargo test --workspace  # 114+ tests

cd extensions/nexode-vscode
npm install
npm run build
npm run build:webview
npm run check-types
npm test  # 15+ existing + new tests
```
