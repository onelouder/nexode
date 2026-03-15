# Codex Prompt — Codex CLI Live Verification

## Task

Run the Codex CLI live verification that was deferred from Sprint 2. This is a pre-Sprint 3 gate — no new features, no code changes. The goal is to confirm the `CodexCliHarness` works end-to-end with a real `codex` process.

## Prerequisites

- `codex` CLI installed and on PATH (via `npm install -g @openai/codex` or equivalent)
- `OPENAI_API_KEY` set with a valid key
- Rust toolchain (1.85+) — the codebase uses `edition = "2024"`
- Sprint 2 branch merged to main (or use `agent/gpt/sprint-2-real-agents` if not yet merged)

## Setup

1. Read these files for context:
   - `HANDOFF.md` — Sprint 2 handoff state, verification commands
   - `ISSUES.md` — open issues (I-016 through I-019 are the latest)
   - `crates/nexode-daemon/tests/live_harness.rs` — the test infrastructure you'll be running
   - `crates/nexode-daemon/src/harness.rs` — `CodexCliHarness` implementation (completion detection, telemetry parsing, CLI flags)
   - `scripts/demo.sh` — end-to-end demo script

2. Understand what you're verifying:
   - `CodexCliHarness` spawns `codex exec --full-auto --json --model <model> <task>`
   - Completion detection: `json_field_is(line, "event", "done") || json_field_is(line, "status", "completed")`
   - Telemetry parsing: `parse_json_summary_telemetry` probes for `tokens_in`, `tokens_out`, `cost_usd` (and camelCase variants) in JSON result lines
   - The test task: "Add a hello() function to hello.rs that returns the string 'Hello from Nexode'. Keep the change minimal and commit it."

## What to Run

### Step 1: Confirm all existing tests pass

```bash
cargo test -p nexode-daemon
cargo test -p nexode-ctl
cargo check --workspace
```

All must pass before proceeding.

### Step 2: Run Codex CLI live smoke test

```bash
OPENAI_API_KEY=<your-key> cargo test -p nexode-daemon \
  --features live-test \
  --test live_harness \
  live_codex_cli_hello_world \
  -- --nocapture
```

**Expected outcome:** The test spawns a real `codex` process, gives it a temp git repo, asks it to create `hello.rs` with a `hello()` function, waits for the daemon to reach REVIEW state, then asserts:
- `hello.rs` exists in the worktree (at `hello.rs` or `src/hello.rs`)
- File contents contain `"hello"`
- `total_tokens > 0` — telemetry was parsed from the Codex JSON output

**Watch for:**
- Does `codex exec --full-auto --json` actually emit JSON to stdout? The `--json` flag is assumed to produce streaming JSON lines. If Codex uses a different output format, completion detection will fail and the test will time out at 120s.
- Does the JSON output include an `{"event": "done"}` or `{"status": "completed"}` line? That's what `detect_completion` looks for. If neither appears, the agent will be marked as failed even if the task succeeded.
- Does the JSON output include token/cost fields? The telemetry parser looks for `tokens_in`/`tokensIn`, `tokens_out`/`tokensOut`, `cost_usd`/`costUsd`, and `total_cost_usd`/`totalCostUsd` in lines matching `type == "result"`, `event == "done"`, or `status == "completed"`.

### Step 3: Run Codex CLI full lifecycle test

```bash
OPENAI_API_KEY=<your-key> cargo test -p nexode-daemon \
  --features live-test \
  --test live_harness \
  live_full_lifecycle \
  -- --nocapture
```

This test only runs if Claude isn't available (it prefers Claude via `any_available()`). To force Codex:

```bash
# Unset ANTHROPIC_API_KEY so the test falls through to Codex
unset ANTHROPIC_API_KEY
OPENAI_API_KEY=<your-key> cargo test -p nexode-daemon \
  --features live-test \
  --test live_harness \
  live_full_lifecycle \
  -- --nocapture
```

**Expected outcome:** Same as Step 2, plus:
- After REVIEW, the test dispatches `MoveTask → MergeQueue`
- Asserts command outcome is `Executed`
- Waits for DONE state
- Asserts the generated file exists in the main repo (not just the worktree)

### Step 4: Run the demo script with Codex

```bash
NEXODE_DEMO_HARNESS=codex-cli \
OPENAI_API_KEY=<your-key> \
bash scripts/demo.sh
```

**Expected outcome:** The script starts the daemon, the agent reaches REVIEW, merge is queued, and repo contents show the generated file. Note: the script may show `merge_queue` instead of `done` in final status due to I-019 (known, non-blocking).

## What to Record

After running, update `HANDOFF.md` with the verification results:

```markdown
## Codex CLI Verification (Pre-Sprint 3)

- **Date:** <date>
- **Agent:** gpt
- **`codex` version:** <output of `codex --version`>
- **Model used:** gpt-4.1

### Results

| Test | Result | Notes |
|---|---|---|
| `live_codex_cli_hello_world` | ✅/❌ | <notes> |
| `live_full_lifecycle` (Codex) | ✅/❌ | <notes> |
| `scripts/demo.sh` (Codex) | ✅/❌ | <notes> |

### Observations

<any issues with JSON format, completion detection, telemetry, etc.>
```

## If Tests Fail

If the Codex CLI output format doesn't match expectations:

1. **Capture the raw output.** Run the test with `--nocapture` and save the full stdout/stderr.
2. **Identify the gap.** Is the completion signal missing? Is telemetry in a different JSON shape? Are the CLI flags wrong?
3. **Fix the harness.** Adjust `CodexCliHarness` in `harness.rs` to match the real output:
   - `detect_completion()` — update the JSON field/value checks
   - `parse_telemetry()` / `parse_json_summary_telemetry()` — update the field paths
   - `build_command()` — update CLI flags if needed
4. **Add a regression test.** Add a unit test in `harness.rs` tests that uses the actual Codex output line as input.
5. **Re-run verification** to confirm the fix works.

If you make code changes, commit with message format: `[gpt] fix: <description>`

## Rules

- Commit messages: `[gpt] fix: description` (only if code changes are needed)
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*`
- Do NOT start Sprint 3 work — this is verification only
- If the harness needs changes, keep them minimal and focused on Codex CLI compatibility
- Update `HANDOFF.md` with results before ending your session

## Exit Criteria

1. `live_codex_cli_hello_world` passes with a real Codex process
2. `live_full_lifecycle` passes with Codex (Claude unavailable)
3. `scripts/demo.sh` runs successfully with `NEXODE_DEMO_HARNESS=codex-cli`
4. `HANDOFF.md` updated with verification results
5. All existing tests still pass (`cargo test -p nexode-daemon && cargo test -p nexode-ctl`)
