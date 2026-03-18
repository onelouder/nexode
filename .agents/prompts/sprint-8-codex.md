# Codex Sprint 8 Prompt — Daemon Hardening + Issue Sweep

## Task

Execute Sprint 8: Daemon Hardening + Issue Sweep. The TUI is production-ready after Sprints 5-7. This sprint clears the accumulated low-severity issue backlog in the daemon and observer crates, adds a proto-level fix for observer finding classification, and adds an integration test for the TUI reconnect path.

## Setup

1. Read these files first (mandatory):
   - `AGENTS.md` — universal agent contract
   - `.agents/openai.md` — your platform config
   - `HANDOFF.md` — current handoff state
   - `PLAN_NOW.md` — current sprint plan
   - `ISSUES.md` — focus on I-013, I-020, I-021, I-023, I-024, I-029

2. Read these for implementation context:
   - `crates/nexode-daemon/src/observer.rs` — LoopDetector, SandboxGuard (I-020, I-021, I-023)
   - `crates/nexode-daemon/src/process.rs` — telemetry parsing (I-013)
   - `crates/nexode-daemon/src/engine/observer_tick.rs` — observer tick integration
   - `crates/nexode-proto/proto/hypervisor.proto` — proto schema (I-024)
   - `crates/nexode-tui/src/events.rs` — event formatting (I-024 client-side update)
   - `docs/architecture/agent-harness.md` — harness documentation (I-029)

## Branch

Create and work on: `agent/gpt/sprint-8-daemon-hardening`

## What to Build

### Part 1: Observer Hardening

#### 1a. Fix I-020 — Guard `observe_output` against unknown slots

**Location:** `crates/nexode-daemon/src/observer.rs`

`observe_output()` calls `self.slots.entry(slot_id).or_default()`, creating a `SlotLoopState` even if `observe_status()` was never called for that slot.

1. Change `observe_output()` to check whether the slot exists in `self.slots` before creating an entry
2. If the slot does not exist, return early (silently ignore the output — the slot was removed or never registered)
3. **Test:** Call `observe_output("unknown-slot", ...)` and verify no entry is created in `self.slots`

#### 1b. Fix I-021 — Configurable alert cooldown

**Location:** `crates/nexode-daemon/src/observer.rs`

Currently, `emitted_*_alert` flags suppress repeated alerts permanently. Replace with a cooldown-based approach:

1. Change the `emitted_*_alert: bool` fields in `SlotLoopState` to `last_*_alert: Option<Instant>`
2. Add an `alert_cooldown: Duration` field to `LoopDetector` (default: 5 minutes)
3. An alert is emitted only if `last_*_alert` is `None` OR `Instant::now() - last_*_alert > alert_cooldown`
4. When an alert fires, update `last_*_alert` to `Some(Instant::now())`
5. When a slot is reset (pause/kill/resume), reset `last_*_alert` to `None`
6. **Test:** Verify alert fires, is suppressed within cooldown, fires again after cooldown expires. Use `tokio::time::pause()` / `tokio::time::advance()` or just check the `Instant` comparisons directly.

#### 1c. Fix I-023 — Filter candidate paths

**Location:** `crates/nexode-daemon/src/observer.rs`, specifically the `candidate_paths` function

The path candidate extraction matches any token containing `/` or `\`. Filter out:

1. URLs: tokens starting with `http://`, `https://`, `ftp://`, `ssh://`
2. Source locations: tokens matching pattern like `src/lib.rs:42:` (path followed by colon + number)
3. MIME types: tokens matching `*/*` pattern (e.g., `application/json`, `text/plain`)
4. Rust module paths: tokens matching `::` syntax (e.g., `std::io::Error`)

The filter should be conservative — only exclude obvious non-filesystem patterns. Anything ambiguous should still be checked.

5. **Test:** Verify that `https://example.com/path`, `src/lib.rs:42:`, `application/json`, and `std::io::Error` are NOT extracted as candidate paths. Verify that `/etc/passwd`, `../escape/attempt`, and `subdir/file.rs` ARE still extracted.

### Part 2: Proto Cleanup (I-024)

**Location:** `crates/nexode-proto/proto/hypervisor.proto`, daemon observer code, TUI events

Currently, `LoopDetected`, `Stuck`, and `BudgetVelocity` observer findings all map to the same `observer_alert::Detail::LoopDetected` proto variant with the distinction only in the `reason` string.

1. Add a `FindingKind` enum to the proto:
   ```protobuf
   enum FindingKind {
     FINDING_KIND_UNSPECIFIED = 0;
     FINDING_KIND_LOOP_DETECTED = 1;
     FINDING_KIND_STUCK = 2;
     FINDING_KIND_BUDGET_VELOCITY = 3;
   }
   ```

2. Add a `FindingKind finding_kind = 3;` field to the `LoopDetected` message in the proto

3. Update the daemon's observer finding → proto conversion (in `engine/observer_tick.rs` or wherever `ObserverFindingKind` is mapped to proto events) to populate the new field

4. Update the TUI's `events.rs` to use the proto `finding_kind` field instead of string parsing:
   - If `finding_kind` is not `UNSPECIFIED`, use it to select the label
   - Fall back to string parsing for backward compatibility with old daemon versions

5. **Tests:**
   - Proto: Verify `FindingKind` enum values serialize/deserialize correctly
   - Daemon: Verify observer findings emit the correct `finding_kind`
   - TUI: Verify event formatting uses the new field and falls back to string parsing when unspecified

### Part 3: Harness & Telemetry Fixes

#### 3a. Fix I-013 — Reject empty telemetry

**Location:** `crates/nexode-daemon/src/process.rs`

`parse_space_delimited` returns `Some(ParsedTelemetry { all None })` for lines starting with `TOKENS ` that have no valid key=value pairs.

1. After parsing, check if all fields in `ParsedTelemetry` are `None`
2. If so, return `None` instead of `Some`
3. **Test:** `parse_space_delimited("TOKENS garbage")` returns `None`. `parse_space_delimited("TOKENS in=100 out=50")` still returns `Some(...)`.

#### 3b. Fix I-029 — Claude harness doc

**Location:** `docs/architecture/agent-harness.md`

Update the Claude CLI invocation documentation to include the full flag set used in `harness.rs`:
- `claude -p --verbose --output-format stream-json --permission-mode bypassPermissions`

Note that the code uses `-p` (short form), not `--print`.

### Part 4: Infrastructure

#### 4a. MSRV Documentation (R-006)

1. Add `rust-version = "1.85"` to the `[package]` section of all workspace Cargo.toml files
2. Add a "Requirements" section to README.md: `Rust 1.85+ (edition 2024)`

#### 4b. Integration Test: Daemon Restart + TUI Reconnect

**Location:** `crates/nexode-daemon/src/engine/tests.rs`

Add an integration test that verifies the TUI reconnect path works against a real daemon restart:

1. Start a daemon engine with gRPC server (reuse the Sprint 6 integration test pattern)
2. Connect a TUI gRPC client and verify initial snapshot
3. Shut down the daemon
4. Verify the TUI client gets a connection error (stream returns `None` or error)
5. Restart the daemon with a new engine
6. Verify the TUI client can reconnect (new `connect` + `GetFullState` + `SubscribeEvents`)

This test exercises the connection lifecycle but does NOT test the TUI's `reconnect_event_stream` loop (which is in the TUI binary, not the library). It tests that the gRPC server correctly handles client reconnection after restart.

## Exit Criteria

All must pass:

1. `observe_output` does not create state for unknown slots
2. Observer alerts re-fire after cooldown period
3. URLs, source locations, MIME types, and Rust module paths are filtered from sandbox candidate paths
4. `LoopDetected` proto has `finding_kind` field; daemon populates it; TUI uses it
5. Malformed `TOKENS` lines produce `None` telemetry
6. Claude harness doc includes `--permission-mode` flags
7. All Cargo.toml files have `rust-version` field
8. Integration test verifies daemon restart + client reconnection
9. No regressions: all existing tests pass

## Verification

Before marking complete:
```bash
cargo fmt --all
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build -p nexode-tui
cargo build -p nexode-daemon
```

## Rules

- Commit messages: `[gpt] type: description`
- Do NOT modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`
- Do NOT modify TUI reconnect, command UX, or help overlay code (except the I-024 `events.rs` update)
- If you need a design decision, document it in HANDOFF.md as a request for pc review
- Update `PLAN_NOW.md` and `HANDOFF.md` before ending your session
- Update `CHANGELOG.md` with user-visible changes
