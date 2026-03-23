# Codex Sprint 11 Prompt — Foundation: Workspace Folders + Agent Output

## Task

Execute Sprint 11: Foundation layer for the Multiplexed Native Workspace Architecture. This sprint introduces worktree-as-workspace-folder and agent output streaming — the two foundational features that all subsequent sprints depend on.

After this sprint: agent worktrees appear as VS Code workspace folders in Explorer (with SCM, intellisense, editor tabs for free), and agent stdout/stderr streams to per-slot VS Code OutputChannels.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `docs/architecture/multiplexed-workspaces.md` — full architecture plan

2. Read these for implementation context:
   - `crates/nexode-proto/proto/hypervisor.proto` — current proto (source of truth)
   - `crates/nexode-daemon/src/engine/runtime.rs` — RuntimeState and snapshot building
   - `crates/nexode-daemon/src/engine/slots.rs` — process event handling (where output publishing goes)
   - `crates/nexode-daemon/src/engine/mod.rs` — engine main loop
   - `crates/nexode-daemon/src/process.rs` — AgentProcessEvent::Output structure
   - `extensions/nexode-vscode/src/extension.ts` — extension activation (where new managers are wired)
   - `extensions/nexode-vscode/src/state.ts` — StateCache and normalization
   - `extensions/nexode-vscode/src/daemon-client.ts` — gRPC client and event flow
   - `extensions/nexode-vscode/src/slot-tree-provider.ts` — reference for state subscription pattern

## Branch

Create and work on: `agent/gpt/sprint-11-workspace-folders`

## What to Build

### Part 1: Proto Changes

**Location:** `crates/nexode-proto/proto/hypervisor.proto` AND `extensions/nexode-vscode/proto/hypervisor.proto` (must be identical)

1. **Add `worktree_path` to `AgentSlot`:**
   ```protobuf
   message AgentSlot {
     // ... existing fields 1-9 ...
     string worktree_path = 10;  // absolute filesystem path, empty when no worktree
   }
   ```

2. **Add `worktrees` to `FullStateSnapshot`:**
   ```protobuf
   message FullStateSnapshot {
     // ... existing fields 1-5 ...
     repeated Worktree worktrees = 6;  // all active worktrees
   }
   ```

3. **Add `AgentOutputLine` event:**
   ```protobuf
   message AgentOutputLine {
     string slot_id = 1;
     string agent_id = 2;
     string stream = 3;    // "stdout" or "stderr"
     string line = 4;
     uint64 timestamp_ms = 5;
   }
   ```

4. **Add to `HypervisorEvent.oneof payload`:**
   ```protobuf
   AgentOutputLine agent_output_line = 13;
   ```

### Part 2: Daemon — Populate `worktree_path` in Snapshots

**Location:** `crates/nexode-daemon/src/engine/runtime.rs`

In the `snapshot()` method (which builds `FullStateSnapshot` from `RuntimeState`):

1. When building each `AgentSlot` proto message from `SlotRuntime`, set `worktree_path` to `slot.worktree_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()`.

2. Build the `worktrees` repeated field on `FullStateSnapshot` by iterating all slots with active worktree paths and constructing `Worktree` messages with:
   - `id`: use the slot's `worktree_id` (or construct as `"{project_id}-{slot_id}"`)
   - `absolute_path`: the worktree path string
   - `branch_name`: from `slot.branch`
   - `conflict_risk`: `0.0` (placeholder until Phase 4 AST scoring)

### Part 3: Daemon — Publish Agent Output Lines

**Location:** `crates/nexode-daemon/src/engine/slots.rs`

In the `AgentProcessEvent::Output` handler (inside `handle_process_event` or equivalent):

1. After the existing telemetry extraction and observer inspection, add:
   ```rust
   self.publish_event(
       hypervisor_event::Payload::AgentOutputLine(AgentOutputLine {
           slot_id: slot_id.clone(),
           agent_id: agent_id.clone(),
           stream: match stream {
               OutputStream::Stdout => "stdout".into(),
               OutputStream::Stderr => "stderr".into(),
           },
           line: line.clone(),
           timestamp_ms: now_ms(),
       }),
       None, // no barrier
   );
   ```

2. **Output ring buffer:** Add `output_buffer: VecDeque<(String, String)>` to `SlotRuntime` (stores last 500 `(stream, line)` pairs). Push each output line, pop front when exceeding capacity. This enables reconnect backfill in Sprint 15.

3. **Important:** Do NOT filter telemetry lines (TOKENS, NEXODE_TELEMETRY) from publishing — let the extension decide what to display. But DO skip empty lines to reduce noise.

### Part 4: Daemon — Increase Event Buffer

**Location:** `crates/nexode-daemon/src/engine/mod.rs` or `config.rs`

Increase the default broadcast channel buffer from 256 to 2048 to handle output event volume. The current default is set in `GrpcBridge::new()` or `with_event_buffer()` in `transport.rs`.

### Part 5: Extension — State Normalization Updates

**Location:** `extensions/nexode-vscode/src/state.ts`

1. **Add `worktreePath` to the `AgentSlot` interface:**
   ```typescript
   export interface AgentSlot {
     // ... existing fields ...
     worktreePath: string;  // absolute path, empty string when no worktree
   }
   ```

2. **Update `normalizeSlot()` (or wherever slots are normalized from raw proto):** Extract `worktree_path` / `worktreePath` from the raw object with `coerceString(raw, 'worktreePath', '')`.

3. **Add `AgentOutputLine` interface:**
   ```typescript
   export interface AgentOutputLine {
     slotId: string;
     agentId: string;
     stream: 'stdout' | 'stderr';
     line: string;
     timestampMs: number;
   }
   ```

4. **In `normalizeEvent()`:** Add a case for the `agentOutputLine` payload field that constructs `AgentOutputLine` from the raw data.

5. **Do NOT store output lines in StateCache.** Output is high frequency and should not trigger the debounced tree refresh. Instead, output events will bypass StateCache entirely (see Part 7).

### Part 6: Extension — Workspace Folder Manager

**Location:** NEW file `extensions/nexode-vscode/src/workspace-folder-manager.ts`

```typescript
import * as vscode from 'vscode';
import { StateCache } from './state';

export class WorkspaceFolderManager implements vscode.Disposable {
  private stateSubscription: vscode.Disposable;
  private knownFolders: Map<string, string> = new Map(); // slotId → worktreePath
  private updateTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(private state: StateCache) {
    this.stateSubscription = this.state.onDidChange(() => this.scheduleReconcile());
  }

  // ... implementation
}
```

**Key behaviors:**

1. **`scheduleReconcile()`:** Debounce 200ms. Calls `reconcile()` once timer fires.

2. **`reconcile()`:**
   - Build desired state: iterate `state.getAllSlots()`, collect slots that have a non-empty `worktreePath` and are NOT in DONE/ARCHIVED status. Map: `slotId → worktreePath`.
   - Build actual state: iterate `vscode.workspace.workspaceFolders`, identify which ones were added by Nexode (use a naming convention: folder name starts with project display name, or maintain a Set of managed URIs).
   - Diff: find folders to add (in desired but not actual) and folders to remove (in actual but not desired).
   - Apply: call `vscode.workspace.updateWorkspaceFolders()` once with all additions and removals batched.
   - Update `knownFolders` map.

3. **Folder naming:** Use `vscode.Uri.file(worktreePath)` as the URI. Set `name` to `"{projectDisplayName}/{slotId}"` for readability in Explorer.

4. **Safety:** Only mutate workspace folders when the extension has an active snapshot (`state.hasSnapshot()`). Don't remove folders during reconnection (connection status not 'connected').

5. **Escape hatch command:** Register `nexode.resetWorkspaceFolders` command that removes all Nexode-managed folders and re-reconciles from state.

6. **`dispose()`:** Clear timer, dispose subscription.

### Part 7: Extension — Output Channel Manager

**Location:** NEW file `extensions/nexode-vscode/src/output-channel-manager.ts`

```typescript
import * as vscode from 'vscode';
import { AgentOutputLine, StateCache } from './state';

export class OutputChannelManager implements vscode.Disposable {
  private channels: Map<string, vscode.OutputChannel> = new Map();
  private stateSubscription: vscode.Disposable;

  constructor(private state: StateCache) {
    this.stateSubscription = this.state.onDidChange(() => this.cleanupChannels());
  }

  appendLine(output: AgentOutputLine): void {
    let channel = this.channels.get(output.slotId);
    if (!channel) {
      channel = vscode.window.createOutputChannel(`Nexode: ${output.slotId}`);
      this.channels.set(output.slotId, channel);
    }
    const prefix = output.stream === 'stderr' ? '[stderr] ' : '';
    channel.appendLine(`${prefix}${output.line}`);
  }

  // ... implementation
}
```

**Key behaviors:**

1. **`appendLine(output)`:** Create OutputChannel lazily on first output for each slot. Prefix stderr lines with `[stderr]` for visibility.

2. **`cleanupChannels()`:** When StateCache changes, check if any slots have transitioned to DONE or ARCHIVED. Dispose and remove their output channels.

3. **`showSlotOutput(slotId)`:** Reveals the output channel for the given slot. Used by the `nexode.showSlotOutput` command.

4. **`dispose()`:** Dispose all channels and subscription.

### Part 8: Extension — DaemonClient Output Bypass

**Location:** `extensions/nexode-vscode/src/daemon-client.ts`

Agent output events are high-frequency and must NOT flow through StateCache (which triggers debounced tree refresh). Instead:

1. Add a dedicated event emitter on DaemonClient:
   ```typescript
   private readonly outputEmitter = new Emitter<AgentOutputLine>();
   public readonly onDidReceiveAgentOutput: Event<AgentOutputLine> = this.outputEmitter.event;
   ```

2. In the event stream handler (where events are dispatched to `subscribeEvents` callbacks), detect `agentOutputLine` payload and fire `outputEmitter` instead of the normal event callback. All other event types continue through the existing path → StateCache.

3. Import the `Emitter` from `state.ts` (it's already exported for testability).

### Part 9: Extension — Wire Everything in `extension.ts`

**Location:** `extensions/nexode-vscode/src/extension.ts`

In `activate()`, after existing component creation:

1. Create `WorkspaceFolderManager`:
   ```typescript
   const workspaceFolderManager = new WorkspaceFolderManager(state);
   context.subscriptions.push(workspaceFolderManager);
   ```

2. Create `OutputChannelManager` and wire to DaemonClient:
   ```typescript
   const outputChannelManager = new OutputChannelManager(state);
   context.subscriptions.push(outputChannelManager);
   context.subscriptions.push(
     client.onDidReceiveAgentOutput((output) => outputChannelManager.appendLine(output))
   );
   ```

3. Register new commands:
   ```typescript
   context.subscriptions.push(
     vscode.commands.registerCommand('nexode.showSlotOutput', async () => {
       const slot = await selectSlot(state, undefined, 'Show Output');
       if (slot) outputChannelManager.showSlotOutput(slot.slotId);
     }),
     vscode.commands.registerCommand('nexode.resetWorkspaceFolders', () => {
       workspaceFolderManager.resetFolders();
     }),
   );
   ```

### Part 10: Extension — Package.json Updates

**Location:** `extensions/nexode-vscode/package.json`

1. Add new commands:
   ```json
   { "command": "nexode.showSlotOutput", "title": "Nexode: Show Slot Output" },
   { "command": "nexode.resetWorkspaceFolders", "title": "Nexode: Reset Workspace Folders" }
   ```

2. Add new activation events:
   ```json
   "onCommand:nexode.showSlotOutput",
   "onCommand:nexode.resetWorkspaceFolders"
   ```

### Part 11: Tests

**Location:** `extensions/nexode-vscode/test/`

1. **`test/workspace-folder-manager.test.ts`:** (NEW)
   - Test reconcile logic with mock state (slots with worktree paths)
   - Test folder addition when slot gains worktree path
   - Test folder removal when slot transitions to DONE
   - Test debounce batching (multiple state changes produce single update)
   - Test safety: no mutations when no snapshot

2. **`test/output-channel-manager.test.ts`:** (NEW)
   - Test lazy channel creation on first output
   - Test stderr prefix
   - Test channel cleanup on DONE/ARCHIVED status
   - Test multiple slots get separate channels

3. **`test/state.test.ts`:** (UPDATE)
   - Add test for normalizing `worktreePath` on AgentSlot
   - Add test for normalizing `AgentOutputLine` event
   - Verify output events do NOT trigger StateCache.onDidChange

4. **Rust tests:** All existing `cargo test --workspace` tests must continue to pass. Add at least one test in `engine/tests.rs` verifying that `AgentOutputLine` events appear in the gRPC event stream.

## Constraints

1. **No Rust API changes that break existing tests.** All 114 Rust tests must pass.
2. **All 15 TypeScript tests must continue to pass** plus new tests.
3. **TypeScript strict mode.** Do not relax `strict: true`.
4. **Proto is append-only.** Never remove or renumber existing fields.
5. **Both proto copies must be identical.** Update both files.
6. **No webview changes in this sprint.** Webview message expansion is Sprint 12.
7. **Maintain nonce-based CSP.** No `'unsafe-inline'`.

## Verification

Before handing off, run:
```bash
# Rust
cd /path/to/repo
cargo check --workspace
cargo test --workspace  # must be 114+ tests, all passing

# TypeScript
cd extensions/nexode-vscode
npm install
npm run build
npm run build:webview
npm run check-types
npm test  # must be 15+ existing + new tests, all passing
```

## Handoff Protocol

When complete:
1. Ensure all verification commands pass
2. Update `CHANGELOG.md` with Sprint 11 entry
3. Commit to working branch: `agent/gpt/sprint-11-workspace-folders`
4. Update `HANDOFF.md` with completion status
5. Do NOT merge — pc will review and merge
