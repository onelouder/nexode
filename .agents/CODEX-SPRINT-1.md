# Sprint 1: WAL Recovery + Agent Harness

> **Agent:** gpt (OpenAI Codex)
> **Branch:** `agent/gpt/sprint-1-wal-harness`
> **Duration:** 2 weeks (target)
> **Spec version:** v2.0.1-locked
> **Prerequisite reads:** `AGENTS.md`, `DECISIONS.md`, `docs/architecture/wal-recovery.md`, `docs/architecture/agent-harness.md`, `docs/spec/master-spec.md` (Sections 2, 3, 6, 9)

---

## Objective

Make Nexode survive restarts and run real coding agents. This sprint delivers two capabilities that separate "validated spike" from "usable tool":

1. **WAL-based crash recovery** — the daemon persists runtime state to disk and, after a restart, reloads session state, re-attaches to surviving agent processes, and respawns dead ones.
2. **Agent Harness abstraction** — replace mock shell scripts with a trait-based adapter layer that can launch real CLI agents (Claude Code, Codex CLI) through a uniform interface.

A secondary deliverable is a **basic context compiler** that assembles task description, include/exclude globs, and recent git diff into a context payload injected at agent dispatch time.

## What to Build

### Week 1: WAL Recovery

#### 1. WAL Persistence Layer (`crates/nexode-daemon/src/wal.rs`)

Implement a write-ahead log that captures daemon state transitions:

**Storage format:** Append-only file at `.nexode/wal.binlog` (project-local, resolved relative to session.yaml like the accounting DB). Use a simple framed format:

```
[u32 length][u32 crc32][WAL entry bytes]
```

Each WAL entry is a serialized `WalEntry` enum:

```rust
enum WalEntry {
    SessionStarted {
        timestamp_ms: u64,
        session_config_hash: [u8; 32],  // SHA-256 of session.yaml content
        daemon_instance_id: String,      // UUID generated at startup
    },
    SlotStateChanged {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        task_status: i32,         // TaskStatus proto value
        agent_id: Option<String>,
        agent_pid: Option<u32>,
        worktree_path: Option<String>,
    },
    TelemetryRecorded {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    },
    MergeCompleted {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        outcome: MergeOutcomeTag,  // enum: Success, Conflict, VerificationFailed
    },
    Checkpoint {
        timestamp_ms: u64,
        full_state: Vec<u8>,      // Serialized RuntimeState snapshot
    },
}
```

**Write semantics:**
- Every `set_task_status` call in the engine writes a `SlotStateChanged` entry before applying the state change in memory.
- Every `apply_telemetry` call writes a `TelemetryRecorded` entry.
- Every `merge_slot` completion writes a `MergeCompleted` entry.
- A `Checkpoint` entry is written every 60 seconds, containing the full serialized `RuntimeState`. This allows truncating older WAL entries.

**Serialization:** Use `serde` with `bincode` for compact binary serialization. Add `Serialize`/`Deserialize` derives to `WalEntry` and any types it contains.

#### 2. Recovery Logic (`crates/nexode-daemon/src/recovery.rs`)

On daemon startup:

1. Parse `session.yaml` as normal.
2. Check for `.nexode/wal.binlog`. If absent, start fresh (current behavior).
3. If present, find the most recent `Checkpoint` entry. Deserialize its `RuntimeState`.
4. Replay all `WalEntry` items after the checkpoint in order.
5. For each slot with a recorded `agent_pid`:
   - Check if the PID is still alive (platform-specific: `kill(pid, 0)` on Unix).
   - If alive: re-attach by monitoring its stdout/stderr (requires re-opening `/proc/{pid}/fd/1` or keeping the pipe handles — see architecture doc for trade-offs).
   - If dead: respawn into the same slot, emit `SlotAgentSwapped` with reason `"crash_recovery"`.
6. For each slot with a worktree path: verify the worktree still exists on disk.
7. Resume the engine loop.

**Session config drift:** Compare the SHA-256 of the current `session.yaml` against the WAL's `session_config_hash`. If they differ, log a warning but proceed — the operator may have intentionally changed the config. Do NOT refuse to start.

**WAL compaction:** After a successful recovery or after writing a `Checkpoint`, truncate all entries before the checkpoint. Keep the file open for new appends.

#### 3. Engine Integration

Modify `DaemonEngine`:
- Add a `Wal` field.
- Wire WAL writes into `set_task_status`, `apply_telemetry`, `merge_slot`.
- Add a periodic checkpoint write (every 60s) to the tick handler.
- Add `DaemonEngine::recover()` as an alternative to `bootstrap()` that uses WAL state.
- The `run_daemon_with_listener` function should attempt recovery before falling back to fresh bootstrap.

#### 4. Tests

- **Checkpoint round-trip:** Write a full `RuntimeState` as a `Checkpoint`, read it back, assert equality.
- **WAL replay:** Write 10 `SlotStateChanged` entries, read them back, verify ordering and content.
- **Recovery integration:** Start daemon, let 2 slots reach WORKING, kill daemon (drop the engine), restart from WAL, verify slots are back in WORKING state with worktrees intact.
- **Config drift:** Start daemon, modify session.yaml, restart from WAL — verify warning is logged but daemon starts.
- **CRC validation:** Corrupt a WAL entry's bytes, verify the reader skips it and logs an error.

### Week 2: Agent Harness

#### 5. Agent Harness Trait (`crates/nexode-daemon/src/harness.rs`)

Define the harness abstraction:

```rust
#[async_trait]
pub trait AgentHarness: Send + Sync + fmt::Debug {
    /// Human-readable name for this harness type (e.g., "claude-code", "codex-cli")
    fn name(&self) -> &str;

    /// Build the command to launch an agent in the given worktree.
    /// The harness is responsible for injecting context (CLAUDE.md, .codex, etc.)
    /// and constructing the right CLI invocation.
    fn build_command(
        &self,
        worktree_path: &Path,
        task: &str,
        context: &ContextPayload,
        config: &HarnessConfig,
    ) -> Result<AgentCommand, HarnessError>;

    /// Parse a line of agent output for telemetry data.
    /// Returns None if the line doesn't contain telemetry.
    fn parse_telemetry(&self, line: &str) -> Option<ParsedTelemetry>;

    /// Detect whether the agent has signaled completion.
    /// Different CLIs signal completion differently.
    fn detect_completion(&self, line: &str) -> bool;
}
```

**HarnessConfig:**

```rust
pub struct HarnessConfig {
    pub model: String,             // e.g., "claude-sonnet-4-5", "gpt-4.1"
    pub provider_config: BTreeMap<String, String>,  // passthrough config
    pub timeout_minutes: u64,
    pub max_context_tokens: Option<u64>,
}
```

**ContextPayload:**

```rust
pub struct ContextPayload {
    pub task_description: String,
    pub include_files: Vec<PathBuf>,    // from context.include globs
    pub exclude_patterns: Vec<String>,  // from context.exclude
    pub recent_diff: Option<String>,    // git diff HEAD~3..HEAD
    pub project_readme: Option<String>, // README.md content if present
}
```

#### 6. Mock Harness (refactor existing)

Extract the current `build_mock_agent_command` into a `MockHarness` that implements `AgentHarness`. This preserves all existing test behavior.

```rust
pub struct MockHarness;

impl AgentHarness for MockHarness {
    fn name(&self) -> &str { "mock" }
    // ... delegates to existing mock command builder
}
```

#### 7. Claude Code Harness

```rust
pub struct ClaudeCodeHarness;
```

Implementation:
- `build_command`: Launches `claude` CLI in the worktree with `--print` mode (non-interactive) and task as prompt. Writes a `CLAUDE.md` file into the worktree root with the context payload before launching.
- `parse_telemetry`: Parses Claude Code's cost output (look for token count and cost lines in stderr).
- `detect_completion`: Detects the Claude Code process exit or specific completion markers.

**Note:** The exact CLI flags and output format for Claude Code's headless mode should be verified against the current Claude Code documentation. If the CLI interface has changed, adapt accordingly. The key contract is: launch in a directory, pass a task, capture stdout/stderr, detect completion.

#### 8. Codex CLI Harness

```rust
pub struct CodexCliHarness;
```

Implementation:
- `build_command`: Launches `codex` CLI with `--approval-mode full-auto` and task as argument. Creates `.codex` instructions file in worktree.
- `parse_telemetry`: Parses Codex's output format for token usage.
- `detect_completion`: Detects Codex process exit.

**Same note as Claude Code:** Verify current CLI flags against documentation.

#### 9. Basic Context Compiler (`crates/nexode-daemon/src/context.rs`)

Build the Phase 1 context compiler (REQ-P1-013):

```rust
pub fn compile_context(
    worktree_path: &Path,
    slot: &SlotConfig,
    project: &ProjectConfig,
) -> Result<ContextPayload, ContextError> {
    // 1. Task description from slot.task
    // 2. Resolve include globs against worktree
    // 3. Copy exclude patterns from slot/project config
    // 4. Run `git diff HEAD~3..HEAD` in worktree (capture output)
    // 5. Read README.md from worktree root if present
    // 6. If max_context_tokens set, truncate/summarize to fit
}
```

This is intentionally simple — no AST, no vector search, no embeddings. That's Phase 4.

#### 10. Harness Selection in Engine

Modify `DaemonEngine::start_slot()`:

1. Resolve the harness from the slot's `model` field:
   - `"mock"` → `MockHarness`
   - `"claude-code"` or models containing `"claude"` → `ClaudeCodeHarness`
   - `"codex"` or models containing `"codex"` or `"gpt"` → `CodexCliHarness`
   - Unknown → error
2. Compile context via `compile_context()`.
3. Call `harness.build_command()` to get the `AgentCommand`.
4. Pass the command to `AgentProcessManager::spawn_slot()` as before.

Add a `harness` field to `session.yaml` slot config (optional, defaults to inference from `model`):

```yaml
slots:
  - id: "feature-auth"
    model: "claude-sonnet-4-5"
    harness: "claude-code"    # optional, overrides model-based inference
    task: "Implement OAuth2 login flow"
```

#### 11. Tests

- **MockHarness round-trip:** Existing engine tests still pass with `MockHarness` adapter.
- **Context compiler:** Test with a fixture repo — verify task, include files, exclude patterns, and git diff are correctly assembled.
- **Harness selection:** Test that `model: "claude-code"` selects `ClaudeCodeHarness`, `model: "codex"` selects `CodexCliHarness`, `model: "mock"` selects `MockHarness`.
- **ClaudeCodeHarness command:** Verify the command structure, CLAUDE.md content, and CLI flags (unit test, no actual Claude Code process needed).
- **Integration (if CLI available):** If the build environment has `claude` or `codex` CLI installed, run a basic "hello world" task and verify the full lifecycle. Gate behind a `#[cfg(feature = "integration")]` flag.

## Exit Criteria

The sprint succeeds if ALL of the following pass:

1. **WAL persistence:** Daemon writes WAL entries during normal operation. After kill + restart, state is recovered — slots resume their prior `TaskStatus`, worktrees are intact, cost totals are correct.
2. **CRC integrity:** Corrupted WAL entries are detected and skipped without crashing.
3. **Agent harness trait:** `MockHarness`, `ClaudeCodeHarness`, and `CodexCliHarness` all implement the trait. Existing mock-based tests pass through the harness layer without behavioral changes.
4. **Context compiler:** Task description + include/exclude globs + git diff are assembled into a `ContextPayload`. Claude Code harness writes `CLAUDE.md`. Codex harness writes `.codex` instructions.
5. **Harness selection:** Session configs with different `model` or `harness` values select the correct harness implementation.
6. **Config migration:** The `harness` field is optional in session.yaml. All existing session.yaml files continue to parse correctly (backward compatible).

**Kill criterion:** If WAL recovery consistently fails to restore slot state after daemon restart, or if real CLI agent processes can't be reliably launched and monitored through the harness abstraction, we stop and reassess.

## What NOT to Build

- No TUI or VS Code extension — that's Phase 2/3
- No AST indexing or vector search in the context compiler — that's Phase 4
- No Observer loops (uncertainty routing, loop detection, sandbox enforcement) — that's Sprint 2
- No 24-hour soak test — that requires Observer loops for unattended operation
- No `provider_config` map merge in session parser — tracked as I-004, not blocking
- No event sequence numbers or command acknowledgment — tracked as R-005/R-007, not blocking

## Key Decisions to Respect

All decisions from Phase 0 remain binding (D-002 through D-010). No new decisions are proposed in this sprint — the WAL format and harness trait are implementation details that don't conflict with existing decisions.

If a decision is needed during implementation (e.g., WAL file format choices, harness CLI flag discovery), document it in a code comment with a `// DECISION:` prefix and flag it for pc review.

## Dependencies

| Dependency | Status | Impact |
|---|---|---|
| `bincode` crate | Not yet in Cargo.toml | Add to nexode-daemon dependencies |
| `sha2` crate | Not yet in Cargo.toml | Add for session config hashing |
| `crc32fast` crate | Not yet in Cargo.toml | Add for WAL entry CRC |
| `uuid` crate | Not yet in Cargo.toml | Add for daemon instance ID |
| `async-trait` crate | Not yet in Cargo.toml | Add for AgentHarness trait |
| `glob` crate | Not yet in Cargo.toml | Add for context compiler include pattern resolution |
| Claude Code CLI | Must be installed on test machine | Required for ClaudeCodeHarness integration test |
| Codex CLI | Must be installed on test machine | Required for CodexCliHarness integration test |

## Architecture References

- WAL format and recovery protocol: `docs/architecture/wal-recovery.md`
- Agent harness trait design: `docs/architecture/agent-harness.md`
- Kanban state machine: `docs/architecture/kanban-state-machine.md`
- All accepted decisions: `DECISIONS.md`

## Git Conventions

- Branch: `agent/gpt/sprint-1-wal-harness`
- Commit messages: `[gpt] type: description`
- Types: `feat`, `fix`, `test`, `refactor`, `chore`, `docs`
- PR to `main` when sprint is complete
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*` (those are pc's domain)
