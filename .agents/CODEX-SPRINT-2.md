# Sprint 2: Real Agent Integration + Critical Fixes

> **Agent:** gpt (OpenAI Codex)
> **Branch:** `agent/gpt/sprint-2-real-agents`
> **Duration:** 2 weeks (target: 2026-03-29)
> **Spec version:** v2.0.1-locked
> **Prerequisite reads:** `AGENTS.md`, `DECISIONS.md`, `ISSUES.md`, `docs/reviews/sprint-1-review.md`, `docs/architecture/command-ack.md`

---

## Objective

Prove the daemon can orchestrate real CLI agents end-to-end. Sprint 1 built the harness abstraction with Mock, ClaudeCode, and CodexCli implementations — but all validation was through mock agents. This sprint fixes known bugs that would cause silent failures with real agents, hardens the command dispatch path, and delivers a working end-to-end demo.

**Three pillars:**

1. **Bug fixes** — resolve I-009, I-010, I-015 (issues that would cause incorrect behavior with real agents)
2. **Command acknowledgment** — replace fire-and-forget `CommandResponse` (R-007) with result-bearing acknowledgment
3. **Live integration** — smoke tests with real `claude`/`codex` CLI, end-to-end demo script

## What to Build

### Part 1: Bug Fixes (Week 1, Days 1–3)

#### Fix 1: I-009 — `completion_detected` overrides non-zero exit code

**File:** `crates/nexode-daemon/src/process.rs`, line ~325

**Current behavior:**
```rust
success: status.success() || completion_detected,
```
This means an agent that prints a completion marker and then crashes (non-zero exit) is reported as successful. The agent could be promoted to REVIEW with corrupt or incomplete work.

**Required change:**
```rust
success: status.success() && (completion_detected || !requires_completion_signal),
```

The semantics should be:
- If exit code is non-zero, the agent **failed** — regardless of any completion markers printed before the crash.
- If exit code is zero AND the harness requires a completion signal (ClaudeCode and CodexCli do; MockHarness may not), then `completion_detected` must also be true for success.
- If exit code is zero AND the harness does NOT require a completion signal (MockHarness), then zero exit code alone is sufficient.

Add a method to the `AgentHarness` trait:
```rust
/// Whether this harness requires a completion signal in stdout for success.
/// If true, zero exit code alone is not sufficient — detect_completion must also fire.
/// MockHarness returns false (backward compat). Real CLI harnesses return true.
fn requires_completion_signal(&self) -> bool;
```

**Decision tag:** `// DECISION: I-009-fix — non-zero exit always means failure. See ISSUES.md I-009.`

**Tests:**
- Agent prints completion marker then exits with code 1 → `success: false`
- Agent exits with code 0, no completion marker, harness requires signal → `success: false`
- Agent exits with code 0, completion marker present → `success: true`
- MockHarness agent exits with code 0, no completion marker → `success: true` (backward compat)

---

#### Fix 2: I-010 — `AgentStateChanged(Executing)` dropped after swap

**File:** `crates/nexode-daemon/src/engine.rs`, `SlotAgentSwapped` handler (~line 670)

**Current behavior:** After a crash-respawn, the engine emits `SlotAgentSwapped` but does NOT emit `AgentStateChanged(Executing)` for the new agent. gRPC subscribers see the swap but not the new agent entering `Executing` state.

**Required change:** After processing `SlotAgentSwapped`, emit an `AgentStateChanged` event with the new agent's state set to `Executing`:

```rust
AgentProcessEvent::SlotAgentSwapped(swapped) => {
    if let Some(slot) = self.slot_mut(&swapped.slot_id) {
        slot.current_agent_id = Some(swapped.new_agent_id.clone());
    }
    self.append_current_slot_state(&swapped.slot_id)?;
    self.publish_event(
        hypervisor_event::Payload::SlotAgentSwapped(swapped.clone()),
        None,
    );
    // FIX I-010: Emit Executing state for the new agent
    self.publish_event(
        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
            agent_id: swapped.new_agent_id.clone(),
            slot_id: swapped.slot_id.clone(),
            new_state: AgentState::Executing.into(),
        }),
        None,
    );
}
```

**Tests:**
- Crash-respawn scenario: verify gRPC event stream contains both `SlotAgentSwapped` AND `AgentStateChanged(Executing)` for the new agent.

---

#### Fix 3: I-015 — JSON substring matching in completion detection

**File:** `crates/nexode-daemon/src/harness.rs`

**Current behavior:**
```rust
// ClaudeCodeHarness
line.contains("\"type\":\"result\"") || line.contains("completed")

// CodexCliHarness  
line.contains("\"completed\"") || line.contains("\"event\":\"done\"")
```

This is fragile: `"completed"` can match ordinary agent output (e.g., "Task completed successfully" in a log line), and the JSON substring match breaks if the JSON has whitespace.

**Required change — ClaudeCodeHarness:**
```rust
fn detect_completion(&self, line: &str) -> bool {
    // Try JSON parse first
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) {
        if val.get("type").and_then(|v| v.as_str()) == Some("result") {
            return true;
        }
    }
    false
}
```

**Required change — CodexCliHarness:**
```rust
fn detect_completion(&self, line: &str) -> bool {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) {
        // Codex CLI JSON output: {"event": "done"} or {"status": "completed"}
        if val.get("event").and_then(|v| v.as_str()) == Some("done") {
            return true;
        }
        if val.get("status").and_then(|v| v.as_str()) == Some("completed") {
            return true;
        }
    }
    false
}
```

**Dependencies:** Add `serde_json` to `nexode-daemon/Cargo.toml` (it may already be a transitive dep — check and add explicitly if needed).

**Tests:**
- `"completed"` in a plain text log line → `false` (no longer a false positive)
- `{"type":"result","subtype":"success"}` → `true` (ClaudeCode)
- `{"type" : "result"}` (with whitespace) → `true` (ClaudeCode)
- `{"event":"done"}` → `true` (Codex)
- `{"status":"completed"}` → `true` (Codex)
- Random JSON without completion fields → `false`

---

### Part 2: Command Acknowledgment — R-007 (Week 1, Days 3–5)

See `docs/architecture/command-ack.md` for full design.

#### Proto Changes

**File:** `crates/nexode-proto/proto/hypervisor.proto`

Update `CommandResponse`:
```protobuf
message CommandResponse {
  bool success = 1;
  string error_message = 2;
  string command_id = 3;        // Echo back the command_id from OperatorCommand
  CommandOutcome outcome = 4;   // What happened
}

enum CommandOutcome {
  COMMAND_OUTCOME_UNSPECIFIED = 0;
  COMMAND_OUTCOME_EXECUTED = 1;     // Command was processed by the engine
  COMMAND_OUTCOME_REJECTED = 2;     // Command was invalid (bad slot_id, wrong state, etc.)
  COMMAND_OUTCOME_SLOT_NOT_FOUND = 3;
  COMMAND_OUTCOME_INVALID_TRANSITION = 4;
}
```

#### Transport Changes

**File:** `crates/nexode-daemon/src/transport.rs`

Replace the fire-and-forget channel with a request/response pattern:

1. Change `command_tx` from `mpsc::UnboundedSender<OperatorCommand>` to `mpsc::UnboundedSender<(OperatorCommand, oneshot::Sender<CommandResponse>)>`.
2. In `dispatch_command`, create a `oneshot::channel()`, send `(command, response_tx)` into the channel, and `await` on the `response_rx`.
3. Add a timeout (5 seconds) on the response — if the engine doesn't respond in time, return `success: false, outcome: UNSPECIFIED, error_message: "Engine did not respond in time"`.

#### Engine Changes

**File:** `crates/nexode-daemon/src/engine.rs`

1. Update the command receiver to accept `(OperatorCommand, oneshot::Sender<CommandResponse>)`.
2. In the command handler, after processing each command, send a `CommandResponse` through the `oneshot::Sender` with the actual result.
3. For each command type (PauseAgent, ResumeAgent, ChatDispatch, MoveTask):
   - Validate the slot_id exists → if not, respond with `SLOT_NOT_FOUND`
   - Validate the state transition is valid → if not, respond with `INVALID_TRANSITION`
   - Execute the command → respond with `EXECUTED`
   - If execution fails → respond with `REJECTED` and error message

#### `nexode-ctl` Changes

**File:** `crates/nexode-ctl/src/main.rs`

Update the `dispatch` command handler to print the actual `CommandResponse`:
- On success: `"✓ Command {command_id} executed"`
- On failure: `"✗ Command {command_id} failed: {error_message} ({outcome})"`

#### Tests

- **Pause existing slot:** Send `PauseAgent` for a valid running slot → `EXECUTED`
- **Pause nonexistent slot:** Send `PauseAgent` for `"slot-xyz"` → `SLOT_NOT_FOUND`
- **Invalid transition:** Send `ResumeAgent` for a slot that's not paused → `INVALID_TRANSITION`
- **Command ID echo:** Verify `CommandResponse.command_id` matches `OperatorCommand.command_id`
- **Timeout:** Drop the engine's command receiver → transport returns timeout error

---

### Part 3: Live Integration Tests (Week 2)

#### Live Harness Smoke Tests

Create `crates/nexode-daemon/tests/live_harness.rs` (integration test file):

```rust
#[cfg(feature = "live-test")]
mod live_tests {
    // These tests require real CLI tools installed and are NOT run in CI.
    // Run manually: cargo test -p nexode-daemon --features live-test -- live_tests
}
```

**Test: `live_claude_code_hello_world`**
1. Create a temporary git repo with a single `README.md`.
2. Write a `session.yaml` that assigns one slot with `model: "claude-sonnet-4-5"`, `harness: "claude-code"`, task: `"Add a hello() function to a new file called hello.rs that returns the string 'Hello from Nexode'"`.
3. Start the daemon engine (in-process, not as a separate binary).
4. Wait for the slot to reach `REVIEW` state (timeout: 120 seconds).
5. Assert: `hello.rs` exists in the worktree and contains a `hello()` function.
6. Assert: telemetry recorded non-zero `tokens_in` and `tokens_out`.

**Test: `live_codex_cli_hello_world`**
Same as above but with `model: "gpt-4.1"`, `harness: "codex-cli"`, and the Codex CLI.

**Test: `live_full_lifecycle`**
1. Create a temporary git repo.
2. Session with one slot, real harness (either claude or codex — parameterize).
3. Start daemon. Wait for REVIEW.
4. Send `MoveTask` command to move slot to MERGE_QUEUE.
5. Wait for DONE state (successful merge).
6. Assert: the agent's changes are on the target branch.
7. Assert: `CommandResponse` for the `MoveTask` returned `EXECUTED`.

**Gating:** All live tests behind `#[cfg(feature = "live-test")]`. Add to `Cargo.toml`:
```toml
[features]
live-test = []
```

#### End-to-End Demo Script

Create `scripts/demo.sh`:

```bash
#!/usr/bin/env bash
# Nexode end-to-end demo — runs a single agent on a test repo
# Usage: ./scripts/demo.sh [claude|codex]
set -euo pipefail

HARNESS="${1:-claude}"
# ... setup temp repo, write session.yaml, start daemon, watch output
```

The script should:
1. Create a temporary git repo with a small Rust project skeleton.
2. Write a `session.yaml` with one slot targeting the test repo.
3. Start `nexode-daemon` pointing at the session.
4. Run `nexode-ctl watch` in the background, piping to stdout.
5. Wait for the slot to reach DONE (merged) or ARCHIVED (failed).
6. Print a summary: success/failure, tokens used, cost, time elapsed.
7. Clean up temp files.

---

## Exit Criteria

The sprint succeeds if ALL of the following pass:

1. **I-009 resolved:** Non-zero exit code always means failure. Agent that prints completion marker then crashes is NOT promoted to REVIEW. Verified by unit test.
2. **I-010 resolved:** After crash-respawn, gRPC event stream contains `AgentStateChanged(Executing)` for the new agent. Verified by integration test.
3. **I-015 resolved:** Completion detection uses proper JSON parsing. Plain-text "completed" in agent output does NOT trigger false positive. Verified by unit tests.
4. **R-007 resolved:** `DispatchCommand` returns `CommandResponse` with actual execution result, not always `success: true`. Invalid commands return appropriate error. Verified by unit tests.
5. **Live smoke test:** At least one real CLI agent (claude or codex) completes a trivial task through the harness and reaches REVIEW state. Gated behind `--features live-test`.
6. **Demo script:** `scripts/demo.sh` runs an end-to-end session from scratch and reports results.

**Kill criterion:** If real CLI agents cannot be reliably launched and monitored through the harness (e.g., CLI flags have changed, output format is unrecognizable, process management breaks under real I/O load), stop and document the incompatibilities for a targeted fix sprint.

## What NOT to Build

- No observer loops (loop detection, uncertainty routing, sandbox enforcement) — that's Sprint 3
- No TUI or VS Code extension — that's M3
- No AST indexing or vector search — that's M5
- No event sequence numbers (R-005) — Sprint 3
- No engine decomposition — tracked but not blocking
- No `provider_config` deep merge (I-004) — not blocking
- No schema migration versioning (I-005) — not blocking

## Dependencies

| Dependency | Status | Impact |
|---|---|---|
| `serde_json` crate | May be transitive; add explicit | For I-015 JSON parsing |
| `claude` CLI | Must be installed for live tests | Gate behind `--features live-test` |
| `codex` CLI | Must be installed for live tests | Gate behind `--features live-test` |

## Key Decisions to Respect

All decisions D-002 through D-010 remain binding. This sprint introduces one new implementation decision:

- **I-009-fix:** Non-zero exit code always means agent failure, regardless of completion markers. This is a behavioral change from Sprint 1. Document with `// DECISION: I-009-fix` comment.

If a decision is needed during implementation, document it with `// DECISION:` prefix and flag for pc review.

## Architecture References

- Command acknowledgment design: `docs/architecture/command-ack.md`
- Sprint 1 review findings: `docs/reviews/sprint-1-review.md`
- Open issues: `ISSUES.md` (I-009, I-010, I-015, R-007)
- Kanban state machine: `docs/architecture/kanban-state-machine.md`
- Agent harness trait: `docs/architecture/agent-harness.md`

## Git Conventions

- Branch: `agent/gpt/sprint-2-real-agents`
- Commit messages: `[gpt] type: description`
- Types: `feat`, `fix`, `test`, `refactor`, `chore`, `docs`
- PR to `main` when sprint is complete
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*` (those are pc's domain)
