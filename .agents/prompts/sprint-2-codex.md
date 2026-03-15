# Codex Sprint 2 Prompt

## Task

Execute Sprint 2: Real Agent Integration + Critical Fixes.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `.agents/CODEX-SPRINT-2.md` — **full sprint instructions with exact code locations and fix specifications**
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `DECISIONS.md` — all accepted decisions (D-002 through D-010)
   - `ISSUES.md` — open issues (I-009, I-010, I-015 are your targets)

2. Read these for implementation context:
   - `docs/architecture/command-ack.md` — R-007 design (oneshot request/response pattern, proto changes, validation rules)
   - `docs/reviews/sprint-1-review.md` — Sprint 1 code review with finding details
   - `docs/architecture/kanban-state-machine.md` — valid state transitions (needed for command validation)
   - `docs/architecture/agent-harness.md` — harness trait design

3. Read source files you'll modify:
   - `crates/nexode-daemon/src/process.rs` — I-009 fix location (~line 325)
   - `crates/nexode-daemon/src/engine.rs` — I-010 fix location (~line 670)
   - `crates/nexode-daemon/src/harness.rs` — I-015 fix location (~lines 167-204)
   - `crates/nexode-daemon/src/transport.rs` — R-007 changes
   - `crates/nexode-proto/proto/hypervisor.proto` — proto schema update
   - `crates/nexode-ctl/src/main.rs` — CLI output update

## Branch

Create and work on: `agent/gpt/sprint-2-real-agents`

## What to Build

The sprint has three parts. Complete them in order:

### Part 1: Bug Fixes (do first)

**I-009** (`process.rs`): The line `success: status.success() || completion_detected` lets a crashed agent (non-zero exit) be reported as successful if it printed a completion marker before crashing. Fix: non-zero exit always means failure. Add `requires_completion_signal() -> bool` to `AgentHarness` trait (MockHarness returns `false`, real harnesses return `true`). See `.agents/CODEX-SPRINT-2.md` Fix 1 for exact semantics.

**I-010** (`engine.rs`): The `SlotAgentSwapped` handler emits the swap event but not `AgentStateChanged(Executing)` for the new agent. Add the missing event emission after the swap. See `.agents/CODEX-SPRINT-2.md` Fix 2 for exact code.

**I-015** (`harness.rs`): `ClaudeCodeHarness.detect_completion` uses `line.contains("\"type\":\"result\"")` — fragile substring match. Replace with `serde_json::from_str` and field checking. Same for `CodexCliHarness`. Add `serde_json` to deps. See `.agents/CODEX-SPRINT-2.md` Fix 3 for exact implementations.

### Part 2: Command Acknowledgment (R-007)

Follow the design in `docs/architecture/command-ack.md` exactly:

1. **Proto**: Add `command_id` (field 3) and `CommandOutcome` enum (field 4) to `CommandResponse`. Add `CommandOutcome` enum with `UNSPECIFIED`, `EXECUTED`, `REJECTED`, `SLOT_NOT_FOUND`, `INVALID_TRANSITION`.
2. **Transport** (`transport.rs`): Change channel type from `mpsc::UnboundedSender<OperatorCommand>` to `mpsc::UnboundedSender<(OperatorCommand, oneshot::Sender<CommandResponse>)>`. Await response with 5s timeout.
3. **Engine** (`engine.rs`): Update command handler to validate commands (slot exists? valid transition?) and send result through oneshot.
4. **CLI** (`nexode-ctl`): Print actual result instead of assuming success.

### Part 3: Live Integration Tests

1. Add `live-test` feature flag to `nexode-daemon/Cargo.toml`.
2. Create `crates/nexode-daemon/tests/live_harness.rs` with `#[cfg(feature = "live-test")]` gated tests.
3. Create `scripts/demo.sh` — end-to-end demo script.

Details for all three parts are in `.agents/CODEX-SPRINT-2.md`.

## Exit Criteria

All six must pass:

1. Non-zero exit code always means agent failure (I-009)
2. `AgentStateChanged(Executing)` emitted after crash-respawn swap (I-010)
3. Completion detection uses JSON parsing, no false positives from plain text (I-015)
4. `DispatchCommand` returns real `CommandResponse` with outcome, not always `success: true` (R-007)
5. At least one real CLI agent smoke test passes (gated behind `--features live-test`)
6. `scripts/demo.sh` exists and runs end-to-end

## Verification

Before opening a PR:
```bash
cargo test -p nexode-daemon
cargo test -p nexode-ctl
cargo check --workspace
```

All existing tests must continue to pass. New tests must be added for each fix.

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- If you need a design decision, add a `// DECISION:` comment and note it in HANDOFF.md for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
