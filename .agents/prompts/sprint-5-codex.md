# Codex Sprint 5 Prompt — TUI Dashboard

## Task

Execute Sprint 5: TUI Dashboard. The daemon is stable, tested, and modular (Sprint 4 decomposed the engine into clean sub-modules). This sprint builds the first real user-facing interface — a terminal dashboard that connects to the running daemon via gRPC and displays live session state.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — open issues (I-024 is relevant for TUI event display)
   - `DECISIONS.md` — D-009 defines the Kanban state machine columns

2. Read these for implementation context:
   - `docs/architecture/kanban-state-machine.md` — Kanban columns map to TUI layout
   - `crates/nexode-proto/proto/hypervisor.proto` — gRPC service and event/entity definitions
   - `crates/nexode-ctl/src/main.rs` — existing gRPC client patterns (connect, subscribe, dispatch)
   - `crates/nexode-ctl/Cargo.toml` — existing tonic/clap dependency versions

3. Read the proto carefully — the TUI renders these entities:
   - `FullStateSnapshot` — projects, task_dag, session cost/budget, last_event_sequence
   - `HypervisorEvent` — all event variants (state changes, telemetry, observer alerts, etc.)
   - `OperatorCommand` — all command variants (pause, resume, kill, move-task, etc.)

## Branch

Create and work on: `agent/gpt/sprint-5-tui-dashboard`

## What to Build

A new crate: `crates/nexode-tui/`

### Part 1: Crate Setup and gRPC Client

**Goal:** New workspace member crate that connects to the daemon and maintains a local state model.

1. Add `crates/nexode-tui/` to the workspace `Cargo.toml`.
2. Dependencies:
   - `ratatui = "0.29"` — terminal UI framework (uses crossterm backend by default)
   - `crossterm` — terminal input/output
   - `nexode-proto` — shared protobuf types
   - `tonic` — gRPC client
   - `tokio` — async runtime
   - `clap` — CLI argument parsing
3. Create `src/main.rs` with:
   - CLI args: `--addr <host:port>` (default `http://[::1]:50051`)
   - Connect to daemon via `HypervisorClient`
   - Fetch initial `FullStateSnapshot` via `GetFullState`
   - Subscribe to `SubscribeEvents` stream
   - Maintain a local `AppState` struct that mirrors the snapshot and updates on each event
4. Create `src/state.rs`:
   - `AppState` struct holding: projects, task_dag, session cost, event log (last 100 events), selected panel index, command input buffer
   - `apply_event(&mut self, event: HypervisorEvent)` — update local state from each event variant
   - `apply_snapshot(&mut self, snapshot: FullStateSnapshot)` — full state replacement

### Part 2: Dashboard Layout

**Goal:** Render a three-panel terminal dashboard with live data.

**Layout:**

```
┌─────────────────────────────────────────────────────┐
│  Nexode Dashboard              Session: $12.34/$100 │  ← header bar
├──────────────┬──────────────────────────────────────┤
│  Projects    │  Slot Detail                         │
│  ┌─────────┐ │  Project: my-app                     │
│  │ my-app  │ │  Slot: slot-a                        │
│  │  slot-a │ │  Task: implement auth module         │
│  │  slot-b │ │  Status: WORKING                     │
│  │ my-lib  │ │  Agent: abc-slot-a-agent-3           │
│  │  slot-c │ │  Mode: full_auto                     │
│  └─────────┘ │  Tokens: 45,230                      │
│              │  Cost: $3.21                          │
│              │  Branch: agent/slot-a/auth            │
├──────────────┴──────────────────────────────────────┤
│  Event Log                                          │
│  [14:23:05] TaskStatusChanged slot-a → WORKING      │
│  [14:23:07] AgentTelemetry slot-a +1200 tok (42/s)  │
│  [14:23:09] ObserverAlert slot-b: loop detected     │
│  [14:23:11] TaskStatusChanged slot-b → PAUSED       │
└─────────────────────────────────────────────────────┘
```

**Implementation:**

1. Create `src/ui.rs`:
   - `render(frame: &mut Frame, state: &AppState)` — the main render function
   - Split the terminal into three regions using `ratatui::layout::Layout`:
     - **Header bar** (1 line): title + session cost/budget
     - **Main area** (split horizontal):
       - **Left panel** (~30%): project/slot tree (navigable list)
       - **Right panel** (~70%): selected slot detail view
     - **Bottom panel** (~30% of remaining height): scrolling event log

2. **Header bar**: Display "Nexode Dashboard" on the left, "Session: ${cost}/${budget}" on the right. Use `ratatui::widgets::Paragraph` with styled spans.

3. **Project/slot tree (left panel)**:
   - `ratatui::widgets::List` showing projects and their slots as a flat list with indentation
   - Each project line: project display_name + budget usage
   - Each slot line (indented): slot-id + task status indicator (colored)
   - Navigable with `↑`/`↓` arrow keys
   - Selected item highlighted
   - Task status colors:
     - `WORKING` → Green
     - `REVIEW` → Yellow
     - `MERGE_QUEUE` → Cyan
     - `PAUSED` → Red
     - `DONE` → White/dim
     - `PENDING` → Gray

4. **Slot detail (right panel)**:
   - Show details for the currently selected slot
   - Fields: project, slot_id, task, status, agent_id, mode, tokens, cost, branch, worktree_id
   - If no slot selected, show "Select a slot from the project tree"

5. **Event log (bottom panel)**:
   - `ratatui::widgets::List` showing the last 50-100 events in reverse chronological order
   - Each event formatted as: `[HH:MM:SS] EventType description`
   - Auto-scrolls to newest event
   - Observer alerts highlighted in yellow/red

6. Create `src/events.rs`:
   - Format each `HypervisorEvent` variant into a human-readable string for the event log
   - Handle all event types: `AgentStateChanged`, `AgentTelemetryUpdated`, `TaskStatusChanged`, `ObserverAlert`, `SlotAgentSwapped`, `ProjectBudgetAlert`, `WorktreeStatusChanged`, `UncertaintyFlagTriggered`

### Part 3: Keyboard Input and Command Dispatch

**Goal:** Interactive controls — navigation, commands, and a command input mode.

1. Create `src/input.rs`:
   - Event loop using `crossterm::event::poll` / `crossterm::event::read`
   - Key bindings:
     - `q` or `Ctrl+C` — quit
     - `↑`/`↓` — navigate project/slot tree
     - `Enter` — select slot (show detail)
     - `p` — pause selected slot (dispatch `PauseAgent`)
     - `r` — resume selected slot (dispatch `ResumeAgent` or `ResumeSlot`)
     - `k` — kill selected slot (dispatch `KillAgent`)
     - `:` — enter command mode (bottom bar becomes a text input)
     - `Esc` — exit command mode
   - Command mode accepts free-form text dispatched as `ChatDispatch` or structured commands:
     - `:move <task-id> <status>` → `MoveTask`
     - `:resume-slot <slot-id> [instruction]` → `ResumeSlot`

2. Commands are dispatched via the gRPC `DispatchCommand` RPC.
3. Show `CommandResponse` result briefly in the header bar or as a status message.

### Part 4: Async Architecture

**Goal:** Clean separation of the async event loop, terminal rendering, and input handling.

The TUI has three concurrent tasks:

1. **gRPC event receiver** — reads from the `SubscribeEvents` stream, sends events to a channel
2. **Input handler** — reads terminal key events, sends actions to a channel
3. **Render loop** — runs at ~15 FPS, reads from both channels, updates `AppState`, and renders

Use `tokio::select!` in the main loop:
```rust
loop {
    tokio::select! {
        Some(event) = grpc_rx.recv() => { state.apply_event(event); }
        Some(action) = input_rx.recv() => { handle_action(&mut state, action, &mut client).await; }
        _ = tick_interval.tick() => { terminal.draw(|f| render(f, &state))?; }
    }
}
```

The crossterm input reader should run in a `spawn_blocking` task (it's synchronous) and forward key events through a channel.

### Part 5: Graceful Terminal Handling

**Goal:** Ensure the terminal is always restored to its normal state, even on panic.

1. On startup: enable raw mode, enter alternate screen, hide cursor
2. On shutdown (normal or panic): disable raw mode, leave alternate screen, show cursor
3. Use a `Drop` guard or `std::panic::set_hook` to ensure cleanup happens on panic
4. Handle `SIGINT` / `SIGTERM` gracefully — clean up terminal before exit

## Exit Criteria

All five must pass:

1. `nexode-tui` binary connects to the daemon, fetches state, and subscribes to events
2. Dashboard renders project tree, slot detail, and event log with live updates
3. Keyboard navigation works (arrow keys, enter, quit)
4. At least pause/resume/kill commands dispatch to the daemon and show results
5. Terminal is properly restored on quit, Ctrl+C, and panic

## Verification

Before marking complete:
```bash
cargo fmt --all
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p nexode-tui  # if any tests exist
cargo build -p nexode-tui
```

The TUI is inherently interactive and can't be fully verified with unit tests. Focus on:
- State management tests (apply_event, apply_snapshot)
- Event formatting tests
- CLI argument parsing tests
- The binary compiles and runs with `--help`

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- Do NOT modify the daemon or proto crates — the TUI is a read-only client of the existing gRPC surface
- If you need a proto change, document it in HANDOFF.md as a request for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes

## Design Guidance

- Keep the TUI simple and functional. Don't over-engineer visual polish in Sprint 5.
- Use `ratatui`'s built-in widgets (List, Paragraph, Block, Table). Don't build custom widgets.
- The event log is the most important panel — operators spend most of their time watching events scroll by.
- Color-code task status consistently (the status → color mapping above).
- The TUI should work in an 80x24 terminal minimum but look better in larger terminals.
- If the daemon isn't running when the TUI starts, show a clear error message and exit.
