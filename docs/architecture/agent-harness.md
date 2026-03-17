# Agent Harness Architecture

> **Status:** Proposed
> **Author:** pc
> **Date:** 2026-03-15
> **Sprint:** Sprint 1 (WAL + Agent Harness)
> **Requirements:** REQ-P1-001, REQ-P1-003, REQ-P1-013

---

## Problem

The Phase 0 daemon launches agents via `build_mock_agent_command()` — a hardcoded function that spawns a shell script emitting fake telemetry. This cannot drive real coding agents. Each agent CLI (Claude Code, Codex CLI, future agents) has its own invocation flags, output format, context injection method, and completion signaling.

Phase 1 requires a uniform abstraction that lets the engine spawn, monitor, and interpret output from any supported agent CLI through a single interface (REQ-P1-001), with context injection that assembles task description, file globs, and recent git history (REQ-P1-013).

## Design

### Core Trait

The `AgentHarness` trait is the central abstraction. Each agent CLI gets one implementation.

```
trait AgentHarness: Send + Sync + Debug
├── name() → &str                           // "mock", "claude-code", "codex-cli"
├── build_command(worktree, task, context, config) → AgentCommand
├── parse_telemetry(line: &str) → Option<ParsedTelemetry>
└── detect_completion(line: &str) → bool
```

**Design principles:**

1. **Synchronous trait methods.** `build_command` prepares a command but does not execute it. The engine's `AgentProcessManager` handles spawning, I/O streaming, and lifecycle. This keeps the harness stateless and testable.
2. **Line-oriented parsing.** Agent stdout/stderr is already streamed line-by-line through the process manager. The harness receives individual lines and returns structured data. No buffering or streaming state inside the harness.
3. **No direct process ownership.** The harness never holds a process handle. It only knows how to construct commands and interpret output. Process lifecycle remains in `process.rs`.

### Supporting Types

```
AgentCommand {
    program: String,           // e.g., "claude", "codex"
    args: Vec<String>,         // CLI arguments
    env: BTreeMap<String, String>,  // extra environment variables
    cwd: PathBuf,              // working directory (usually the worktree)
    setup_files: Vec<SetupFile>,  // files to write into worktree before launch
}

SetupFile {
    relative_path: PathBuf,    // e.g., "CLAUDE.md" or ".codex"
    content: String,           // file content
    overwrite: bool,           // whether to overwrite if it exists
}

ParsedTelemetry {
    tokens_in: Option<u64>,
    tokens_out: Option<u64>,
    cost_usd: Option<f64>,
    model: Option<String>,
}

HarnessConfig {
    model: String,                           // e.g., "claude-sonnet-4-5", "gpt-4.1"
    provider_config: BTreeMap<String, String>,  // passthrough config from session.yaml
    timeout_minutes: u64,
    max_context_tokens: Option<u64>,
}
```

### Harness Implementations

#### MockHarness

Refactors the existing `build_mock_agent_command()` into the trait interface. Preserves all current test behavior.

- `build_command`: Returns the existing shell script command that emits `NEXODE_TELEMETRY:` lines and commits mock work.
- `parse_telemetry`: Parses `NEXODE_TELEMETRY:tokens_in=N,tokens_out=N,cost_usd=N` format.
- `detect_completion`: Returns true when the process exits (detected by the process manager, not the harness).

This is the only harness used in automated tests. All existing engine and process manager tests should pass without modification by swapping `build_mock_agent_command` calls for `MockHarness::build_command`.

#### ClaudeCodeHarness

Drives the `claude` CLI in non-interactive (headless) mode.

- `build_command`:
  1. Writes a `CLAUDE.md` file into the worktree root containing the assembled context (task description, include/exclude patterns, recent diff summary, project README excerpt).
  2. Returns command: `claude --print --verbose --output-format stream-json --model {model} "{task}"` with `cwd` set to the worktree.
  3. Sets `ANTHROPIC_API_KEY` from `provider_config` if present.
- `parse_telemetry`: Parses Claude Code's final JSON summary line for token/cost data.
- `detect_completion`: Returns true when Claude emits its JSON completion record (`{"type":"result", ...}`).

**Context injection strategy:** Claude Code reads `CLAUDE.md` from the repo root automatically. The harness writes this file before launch, so no CLI flags are needed for context injection.

#### CodexCliHarness

Drives the `codex` CLI in full-auto mode.

- `build_command`:
  1. Writes a `.codex` instructions file into the worktree with context payload.
  2. Returns command: `codex exec --full-auto --json [--model {model}] "{task}"` with `cwd` set to the worktree. Omit `--model` when the session uses the CLI default model.
  3. Sets `OPENAI_API_KEY` from `provider_config` if present.
- `parse_telemetry`: Parses Codex CLI's final JSON summary line for token/cost data.
- `detect_completion`: Returns true when Codex emits a JSON completion record such as `{"type":"turn.completed", ...}`.

**Note:** Both real harness implementations should be verified against current CLI documentation before use. The exact flags and output formats may evolve. The key contract is: launch a process, capture output, detect completion.

### Harness Selection

The engine selects a harness based on the slot configuration in `session.yaml`:

```
Slot config → harness resolution:
1. If slot.harness is set → use that value directly
2. Else infer from slot.model:
   ├── model contains "mock"   → MockHarness
   ├── model contains "claude" → ClaudeCodeHarness
   ├── model contains "codex" or "gpt" → CodexCliHarness
   └── else → error: unknown harness for model "{model}"
```

The `harness` field in session.yaml is optional — it's an explicit override for cases where model name inference is ambiguous or a new agent CLI is added before formal support.

```yaml
# Inference (most common):
slots:
  - id: "feature-auth"
    model: "claude-sonnet-4-5"    # → ClaudeCodeHarness
    task: "Implement OAuth2 login"

# Explicit override:
slots:
  - id: "feature-db"
    model: "gpt-4.1"
    harness: "codex-cli"          # explicit, overrides model inference
    task: "Add migration system"
```

### Context Compiler

The context compiler (`context.rs`) is a standalone function, not part of the harness trait. The engine calls it before calling `harness.build_command()`, passing the result as a `ContextPayload`.

```
compile_context(worktree_path, slot_config, project_config) → ContextPayload
├── 1. Extract task description from slot.task
├── 2. Resolve include globs against worktree (using glob crate)
├── 3. Copy exclude patterns from slot/project config
├── 4. Run `git diff HEAD~3..HEAD` in worktree (shell out to git)
├── 5. Read README.md from worktree root if present
└── 6. If max_context_tokens set, truncate to fit (simple byte budget)
```

**ContextPayload:**

```
ContextPayload {
    task_description: String,
    include_files: Vec<PathBuf>,       // resolved file paths
    exclude_patterns: Vec<String>,     // raw glob patterns
    recent_diff: Option<String>,       // git diff output
    project_readme: Option<String>,    // README.md content
}
```

This is the Phase 1 context compiler — intentionally minimal. No AST parsing, no vector search, no embeddings. Those are Phase 4 (REQ-P4-xxx). The goal is to give the agent enough context to start working without manual setup.

## Interaction with Existing Modules

| Module | Change |
|---|---|
| `engine.rs` | Add `harness_registry: HashMap<String, Box<dyn AgentHarness>>`. Call `compile_context()` then `harness.build_command()` in `start_slot()`. Pass `AgentCommand` to process manager. |
| `process.rs` | Change `spawn_slot()` to accept an `AgentCommand` instead of building its own command. The process manager becomes harness-agnostic. Pipe agent stdout/stderr lines through `harness.parse_telemetry()` for structured extraction. |
| `session.rs` | Add optional `harness: Option<String>` field to slot config. Backward compatible — absent field means infer from model. |
| `wal.rs` | No change — the WAL records slot state, not harness details. |
| `git.rs` | No change — worktree operations are independent of how agents are launched. |
| `accounting.rs` | No change — receives `ParsedTelemetry` data regardless of source. |

## Data Flow

```
session.yaml
    │
    ▼
Engine.start_slot(slot)
    │
    ├─► compile_context(worktree, slot, project) → ContextPayload
    │
    ├─► resolve_harness(slot.model, slot.harness) → Box<dyn AgentHarness>
    │
    ├─► harness.build_command(worktree, task, context, config) → AgentCommand
    │       │
    │       ├─► writes setup files (CLAUDE.md, .codex, etc.)
    │       └─► returns program + args + env
    │
    └─► process_manager.spawn_slot(slot_id, agent_command)
            │
            ├─► spawns child process
            ├─► streams stdout/stderr line by line
            ├─► calls harness.parse_telemetry(line) for each line
            └─► calls harness.detect_completion(line) for each line
```

## Testing Strategy

| Test | What it proves |
|---|---|
| MockHarness backward compat | All existing engine tests pass through harness layer unchanged |
| Context compiler with fixture repo | Task, globs, diff, and README are correctly assembled |
| Harness selection | Model-based and explicit-override selection works correctly |
| ClaudeCodeHarness command shape | Correct CLI flags, CLAUDE.md content, env vars (unit test, no live CLI) |
| CodexCliHarness command shape | Correct CLI flags, .codex content, env vars (unit test, no live CLI) |
| Setup file writing | `AgentCommand.setup_files` are written to worktree before process spawn |
| Integration (gated) | If CLI is installed, run "hello world" task end-to-end. Behind `#[cfg(feature = "integration")]` |

## Future Extensions

The trait is designed to accommodate future agent types without changes to the engine or process manager:

- **JarvisHarness** (OpenClaw/DGX Spark) — if jarvis exposes a CLI or API
- **CursorHarness** — if Cursor adds a headless CLI mode
- **RemoteHarness** — wraps an SSH or container-based agent invocation
- **CompositeHarness** — chains multiple agents (e.g., plan with one, implement with another)

Each new harness is a single struct implementing `AgentHarness`. No engine changes required.
