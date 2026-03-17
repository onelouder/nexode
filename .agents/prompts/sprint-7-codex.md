# Codex Sprint 7 Prompt — TUI Command Hardening

## Task

Execute Sprint 7: TUI Command Hardening. The TUI dashboard is functional (Sprint 5) and polished (Sprint 6). This sprint makes it production-ready by adding reconnection resilience, better command UX, a help overlay, and closing two open issues.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — focus on I-019, I-024
   - `docs/reviews/sprint-6-review.md` — previous review context

2. Read these for implementation context:
   - `crates/nexode-tui/src/main.rs` — async loop, gRPC clients, command dispatch
   - `crates/nexode-tui/src/input.rs` — key bindings, command parsing
   - `crates/nexode-tui/src/state.rs` — `AppState`, event application
   - `crates/nexode-tui/src/ui.rs` — dashboard rendering
   - `crates/nexode-tui/src/events.rs` — event formatting, severity mapping
   - `crates/nexode-proto/proto/hypervisor.proto` — event/command surface

## Branch

Create and work on: `agent/gpt/sprint-7-tui-command-hardening`

## What to Build

### Part 1: Reconnection

**Goal:** The TUI should survive daemon restarts and network interruptions without operator intervention.

1. When the gRPC event stream disconnects (server shutdown, network error, `DATA_LOSS`):
   - Set `AppState.connection_status` to `Disconnected` (or `Reconnecting`)
   - Begin exponential backoff reconnection: 1s, 2s, 4s, 8s, capped at 30s
   - On each attempt: try `HypervisorClient::connect(addr)`, then `GetFullState`, then `SubscribeEvents`
   - On success: `apply_snapshot()`, reset backoff, set status to `Connected`
   - During reconnection: the TUI keeps rendering (showing stale data with a "Disconnected" indicator)

2. Add `connection_status: ConnectionStatus` to `AppState`:
   ```rust
   enum ConnectionStatus {
       Connected,
       Disconnected { since: Instant },
       Reconnecting { attempt: u32, next_retry: Instant },
   }
   ```

3. **Header bar update:** Show connection status in the header:
   - Connected: no extra indicator (or a subtle `●` green dot)
   - Disconnected/Reconnecting: `⚠ Disconnected (retry in Xs)` in yellow/red

4. While disconnected:
   - Key bindings still work (quit, navigate, help)
   - Command dispatch (`p`, `r`, `k`, `:`) should show "Not connected" in the status bar instead of attempting gRPC calls
   - The event log should show a `[DISCONNECTED]` entry when the connection drops and a `[RECONNECTED]` entry when it recovers

5. **Implementation approach:** The reconnection loop should run as a separate task in the `tokio::select!` loop. When the event stream breaks, the gRPC receiver task sends a `GrpcMessage::Disconnected` through the channel and begins reconnection attempts. On success, it sends `GrpcMessage::Snapshot(...)` to reset state.

6. **Tests:**
   - Unit test: `ConnectionStatus` state transitions (Connected → Disconnected → Reconnecting → Connected)
   - Unit test: `AppState` rejects command dispatch when disconnected
   - The reconnection loop itself is hard to unit test (requires a real server); verify manually or via the existing integration test pattern

### Part 2: Command UX

**Goal:** Make the command mode more usable for operators who spend extended time in the TUI.

#### 2a. Command History

1. Add `command_history: Vec<String>` and `history_index: Option<usize>` to `AppState`
2. When a command is submitted (Enter in command mode):
   - Push the command string to `command_history`
   - Reset `history_index` to `None`
3. In command mode, `↑` and `↓` cycle through history:
   - `↑`: if `history_index` is `None`, set to `command_history.len() - 1` (most recent); otherwise decrement
   - `↓`: increment `history_index`; if it goes past the end, set to `None` and show empty input
   - The current history entry replaces the command input buffer
4. Cap history at 50 entries (drop oldest when full)
5. **Test:** Push 3 commands, verify `↑` cycles through them in reverse order, `↓` returns to empty

#### 2b. Status Bar Feedback

1. Add `status_message: Option<(String, Instant)>` to `AppState`
2. After command dispatch, set the status message to the `CommandResponse` result text
3. Auto-clear the status message after 5 seconds (check in the render tick)
4. Render the status message at the bottom of the screen (below the event log), replacing the command input line when not in command mode
5. Errors should be red, successes green, info white

#### 2c. Tab-Complete for Slot IDs

1. In command mode, when the user presses `Tab`:
   - If the input starts with `:move ` or `:resume-slot `, extract the partial argument
   - Match against known slot IDs from `AppState.projects[].slots[].id`
   - If exactly one match: complete it
   - If multiple matches: complete to the longest common prefix and show candidates in the status bar
   - If no matches: do nothing
2. **Test:** With slots `["slot-a", "slot-b", "slot-alpha"]`, typing `:move slot-a` + Tab should complete to `:move slot-a` (two matches: `slot-a`, `slot-alpha`); typing `:move slot-b` + Tab should complete to `:move slot-b ` (one match)

### Part 3: Help Overlay

**Goal:** Operators should be able to see keybindings without leaving the dashboard.

1. `?` key toggles a help overlay
2. The overlay renders on top of the main dashboard (use a `ratatui::widgets::Clear` + centered `Paragraph` block)
3. Content:
   ```
   Nexode TUI — Keyboard Reference

   Navigation
     ↑/↓     Navigate project/slot tree
     Enter   Select slot
     q       Quit
     Ctrl+C  Quit

   Commands
     p       Pause selected slot
     r       Resume selected slot
     k       Kill selected slot
     :       Enter command mode
     Esc     Exit command mode

   Command Mode
     :move <task-id> <status>          Move task to status
     :resume-slot <slot-id> [instr]    Resume slot with instruction
     <text>                            Chat dispatch to selected slot

   Other
     ?       Toggle this help
   ```
4. While the help overlay is visible, only `?` and `q`/`Ctrl+C` should be active
5. Add `show_help: bool` to `AppState`
6. **Test:** Toggle `show_help` on/off, verify key filtering

### Part 4: Issue Fixes

#### 4a. Fix I-019 — demo.sh wait for DONE

**Location:** `scripts/demo.sh`

After the `dispatch move-task` command, add a poll loop that waits for the slot to reach `DONE`:
```bash
echo "Waiting for merge to complete..."
for i in $(seq 1 15); do
    STATUS=$(nexode-ctl status --json 2>/dev/null | grep -o '"status":"[^"]*"' | head -1)
    if echo "$STATUS" | grep -q "done"; then
        echo "Merge complete."
        break
    fi
    sleep 1
done
```
Adjust the parsing to match the actual `nexode-ctl status` output format. The key point: don't exit immediately after `move-task`.

#### 4b. Improve I-024 — Parse LoopDetected reason strings

**Location:** `crates/nexode-tui/src/events.rs`

Currently, `LoopDetected` observer alerts all display as `loop/stuck/budget`. The daemon includes the finding kind in the `reason` field string. Parse it to show more specific labels:

1. If `reason` contains "repeated" or "loop" → display as `Loop Detected`
2. If `reason` contains "stuck" or "timeout" → display as `Stuck`
3. If `reason` contains "budget" or "velocity" → display as `Budget Velocity`
4. Otherwise → display the raw reason string

This is a best-effort heuristic until I-024 is properly fixed with a proto split. It's better than the current blanket label.

5. **Test:** Three test cases with different reason strings, verify the label extracted from each

## Exit Criteria

All must pass:

1. TUI reconnects automatically after daemon restart, with backoff and status indicator
2. Command history works in command mode (↑/↓ cycle, capped at 50)
3. Status bar shows command results with 5-second auto-clear
4. Tab-complete works for slot IDs in `:move` and `:resume-slot` commands
5. `?` toggles a help overlay with correct keybinding reference
6. `scripts/demo.sh` waits for DONE after MoveTask
7. Event log shows specific labels for LoopDetected variants (not blanket `loop/stuck/budget`)
8. No regressions: all existing tests pass

## Verification

Before marking complete:
```bash
cargo fmt --all
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build -p nexode-tui
cargo run -p nexode-tui -- --help
```

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- Do NOT modify the daemon engine or proto crates — TUI-only changes plus `demo.sh`
- If you need a daemon change, document it in HANDOFF.md as a request for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes

## Design Guidance

- The reconnection indicator should be subtle when connected, obvious when disconnected. Don't waste header space on a green dot if the connection is healthy — only show something when there's a problem.
- Command history should feel like a shell: `↑` goes to the previous command, `↓` goes to the next, falling off the end clears the input.
- The help overlay should be dismissable with any key (not just `?`), similar to `less` or `man`. Keep it simple.
- Tab-complete should be non-blocking and instant — it's just filtering a list of strings from `AppState`.
- The status bar auto-clear prevents stale messages from lingering. 5 seconds is long enough to read, short enough to not clutter.
