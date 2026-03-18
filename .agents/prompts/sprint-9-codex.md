# Codex Sprint 9 Prompt — VS Code Extension Scaffold

## Task

Execute Sprint 9: VS Code Extension Scaffold. This is the first sprint of milestone M3b (Phase 3 — VS Code Integration). The daemon, TUI, and proto layers are stable. This sprint introduces the first TypeScript component to the workspace: a VS Code extension that connects to the daemon via gRPC, renders a slot status panel, and dispatches basic operator commands.

The spec reference is `docs/spec/master-spec.md` section 11, "Week 1: Extension Scaffold." Sprint 9 covers the scaffold, gRPC client, a minimal status view, and basic command dispatch. React Webviews (Synapse Grid, Kanban Board) are Sprint 10+.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ROADMAP.md` — M3b milestone

2. Read these for implementation context:
   - `crates/nexode-proto/proto/hypervisor.proto` — gRPC service definition (source of truth)
   - `crates/nexode-tui/src/events.rs` — reference for event formatting and severity mapping
   - `crates/nexode-tui/src/state.rs` — reference for state management (snapshot + incremental events)
   - `docs/spec/master-spec.md` — section 11 (Phase 3 VS Code Integration)

## Branch

Create and work on: `agent/gpt/sprint-9-vscode-scaffold`

## What to Build

### Part 1: Extension Scaffold

**Location:** `extensions/nexode-vscode/`

This is a new top-level directory, parallel to `crates/`. It is NOT part of the Rust workspace.

1. **`package.json`:**
   - Name: `nexode-vscode`
   - Publisher: `nexode`
   - Activation events: `onCommand:nexode.*`, `onView:nexodeSlots`
   - `contributes.viewsContainers.activitybar`: one entry with Nexode icon
   - `contributes.views.nexode`: `nexodeSlots` tree view
   - `contributes.commands`: `nexode.pauseSlot`, `nexode.resumeSlot`, `nexode.moveTask`
   - `contributes.configuration`: `nexode.daemonHost` (default `localhost`), `nexode.daemonPort` (default `50051`)
   - Engine: `node >= 18`
   - Dependencies: `@grpc/grpc-js`, `@grpc/proto-loader` (or `google-protobuf` + `grpc-tools` generated stubs)

2. **`tsconfig.json`:**
   - Target: `ES2022`
   - Module: `Node16` (or `CommonJS` if esbuild handles it)
   - Strict mode enabled
   - `outDir: ./dist`

3. **Build system:**
   - Use `esbuild` for bundling (`esbuild.mjs` or `esbuild.js` build script)
   - Single entry point bundled to `dist/extension.js`
   - `npm run build` produces the bundle
   - `npm run watch` for dev mode with `--watch`
   - `.vscodeignore`: exclude `node_modules/`, `src/`, `*.ts`, `.gitignore`

4. **Entry point (`src/extension.ts`):**
   - `activate(context)`: create `DaemonClient`, register TreeView, register commands, connect
   - `deactivate()`: disconnect client, dispose resources
   - Register disposables on `context.subscriptions`

5. **`.gitignore` update:**
   - Add `node_modules/` and `dist/` under `extensions/nexode-vscode/` (either a local `.gitignore` in that directory or update the root `.gitignore`)

### Part 2: gRPC Client

**Location:** `extensions/nexode-vscode/src/daemon-client.ts`

The daemon exposes these RPCs at `hypervisor.proto`:
```
service Hypervisor {
  rpc SubscribeEvents(SubscribeRequest) returns (stream HypervisorEvent);
  rpc DispatchCommand(OperatorCommand) returns (CommandResponse);
  rpc GetFullState(StateRequest) returns (FullStateSnapshot);
}
```

1. **Proto loading:**
   - Copy `crates/nexode-proto/proto/hypervisor.proto` to `extensions/nexode-vscode/proto/hypervisor.proto` (or reference it via relative path)
   - Use `@grpc/proto-loader` to load the proto at runtime, OR use `grpc-tools` / `buf` to generate static TypeScript stubs. Either approach is acceptable; runtime loading is simpler for a scaffold.

2. **`DaemonClient` class:**
   - Constructor takes `host: string`, `port: number`
   - `connect()`: create gRPC channel with `grpc.credentials.createInsecure()`
   - `disconnect()`: close channel, cancel active streams
   - `getFullState()`: unary RPC → returns `FullStateSnapshot`
   - `subscribeEvents(callback)`: server-streaming RPC → calls `callback(event)` for each `HypervisorEvent`
   - `dispatchCommand(command)`: unary RPC → returns `CommandResponse`
   - Connection state: `connected | disconnected | reconnecting`
   - Event emitter or callback pattern for state changes

3. **Reconnection:**
   - If the event stream drops, wait 2 seconds and retry
   - Exponential backoff: 2s → 4s → 8s → 16s → 30s cap
   - On reconnect: call `getFullState()` to resync, then re-subscribe
   - Emit connection state changes so the UI can reflect them

4. **Configuration:**
   - Read `nexode.daemonHost` and `nexode.daemonPort` from VS Code settings
   - Default: `localhost:50051`

### Part 3: Slot Status TreeView

**Location:** `extensions/nexode-vscode/src/slot-tree-provider.ts`

Implement a VS Code `TreeDataProvider<SlotTreeItem>` that displays the project → slot hierarchy.

1. **Data model:**
   - Maintain a local state mirror: `projects: Project[]`, `taskDag: TaskNode[]`
   - Updated from `getFullState()` snapshot on connect
   - Updated incrementally from `subscribeEvents()` stream (apply `AgentStateChanged`, `TaskStatusChanged`, `AgentTelemetryUpdated`, etc.)
   - Match the pattern in `crates/nexode-tui/src/state.rs` (`apply_snapshot` / `apply_event`)

2. **Tree structure:**
   - Root level: one node per project (`Project.display_name`)
   - Children: one node per slot within the project
   - Slot nodes show: slot ID, task status (with color icon), agent ID, token count

3. **Status icons:**
   - Use VS Code `ThemeIcon` or colored circle icons
   - Map task statuses to colors per D-009 kanban spec:
     - PENDING → Gray
     - WORKING → Cyan/Teal
     - REVIEW → Yellow
     - MERGE_QUEUE → Blue
     - RESOLVING → Red
     - DONE → Green
     - PAUSED → DarkGray
     - ARCHIVED → DarkGray

4. **Live refresh:**
   - Call `this._onDidChangeTreeData.fire()` when state changes from the event stream
   - Debounce if events arrive rapidly (e.g., 100ms coalesce)

### Part 4: Command Dispatch

**Location:** `extensions/nexode-vscode/src/commands.ts`

1. **`nexode.pauseSlot`:**
   - Show quick-pick with active (WORKING/REVIEW) slot IDs
   - Dispatch `MoveTask { task_id: selected, target: PAUSED }`
   - Show info/error message from `CommandResponse`

2. **`nexode.resumeSlot`:**
   - Show quick-pick with PAUSED slot IDs
   - Dispatch `ResumeSlot { slot_id: selected, instruction: "" }`
   - Show info/error message

3. **`nexode.moveTask`:**
   - Show quick-pick for slot ID
   - Show quick-pick for target status (WORKING, REVIEW, MERGE_QUEUE, PAUSED, ARCHIVED)
   - Dispatch `MoveTask { task_id: selected, target: selected_status }`
   - Show info/error message

4. **Status bar item:**
   - Always-visible item in VS Code footer
   - Shows connection state: `$(plug) Nexode: Connected` / `$(debug-disconnect) Nexode: Disconnected`
   - Shows agent count and aggregate token count when connected (from `FullStateSnapshot`)
   - Click opens the Nexode activity bar view

## File Structure

```
extensions/nexode-vscode/
├── .vscodeignore
├── .gitignore
├── package.json
├── tsconfig.json
├── esbuild.mjs
├── proto/
│   └── hypervisor.proto        # copy from crates/nexode-proto/proto/
├── src/
│   ├── extension.ts            # activate/deactivate entry point
│   ├── daemon-client.ts        # gRPC client with reconnect
│   ├── state.ts                # local state mirror (snapshot + events)
│   ├── slot-tree-provider.ts   # TreeDataProvider for slot panel
│   ├── commands.ts             # command palette handlers
│   └── status-bar.ts           # footer status bar item
├── resources/
│   └── nexode-icon.svg         # activity bar icon (simple placeholder)
└── dist/                       # (gitignored) esbuild output
```

## Exit Criteria

All must pass:

1. `npm install` succeeds in `extensions/nexode-vscode/`
2. `npm run build` produces `dist/extension.js` without errors
3. Extension activates in VS Code without errors (test via `code --extensionDevelopmentPath=...` or verify that the activate function doesn't throw)
4. `DaemonClient` connects to a running daemon and receives a `FullStateSnapshot`
5. `DaemonClient` subscribes to events and receives `HypervisorEvent` messages
6. TreeView renders project → slot hierarchy from snapshot data
7. TreeView updates when events arrive
8. Command palette shows `Nexode: Pause Slot`, `Nexode: Resume Slot`, `Nexode: Move Task`
9. Commands dispatch via gRPC and show response feedback
10. Status bar shows connection state
11. Reconnection works: stop daemon, verify disconnected state, restart daemon, verify reconnected

## Verification

Before marking complete:
```bash
cd extensions/nexode-vscode
npm install
npm run build        # must produce dist/extension.js
npm run lint         # if eslint is configured
```

Also verify the Rust workspace is unaffected:
```bash
cd ../..
cargo check --workspace
cargo test --workspace
```

## Constraints

- Do NOT modify any Rust code (`crates/`, `Cargo.toml`, `Cargo.lock`)
- Do NOT modify `hypervisor.proto` — the proto is stable. If you need changes, document them in HANDOFF.md as deferred requests.
- Do NOT add the extension to the Rust `Cargo.toml` workspace members
- Keep the proto file in sync: copy from `crates/nexode-proto/proto/hypervisor.proto` as-is
- Use `@grpc/grpc-js` (pure JS), NOT `grpc` (native, deprecated)
- No React in this sprint — TreeView is native VS Code API. React Webviews are Sprint 10+.

## Design Notes

- **R-008 (Extension Host IPC bottleneck):** The spec warns about Extension Host saturation at N>3 agent streams. For Sprint 9 (scaffold), standard Extension Host gRPC is fine. Flag if you see performance issues in testing. The mitigation (WebSocket bridge bypassing Extension Host) is Phase 3 Week 2+.
- **State model:** The TUI's `AppState` (`crates/nexode-tui/src/state.rs`) is a good reference. It applies a `FullStateSnapshot` on connect and then incrementally applies `HypervisorEvent` payloads. Mirror this pattern in TypeScript.
- **Connection lifecycle:** Match the TUI's reconnect pattern (`crates/nexode-tui/src/main.rs` `reconnect_event_stream`): on stream drop, attempt reconnect with backoff, resync via `GetFullState`, then re-subscribe.

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, any Rust source files
- If you need a design decision, document it in HANDOFF.md as a request for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with the new extension scaffold entry
