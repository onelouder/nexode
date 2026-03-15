# ROADMAP.md — Project Milestones & Backlog

## Milestones

### M0: Phase 0 Spike — Core Daemon Runtime ✅
- **Target:** 2026-03-15
- **Status:** Complete
- **Agent:** gpt (Codex)
- **Deliverables:**
  - [x] Cargo workspace: `nexode-daemon`, `nexode-proto`, `nexode-ctl`
  - [x] Session config parser with include resolution and D-004 cascade
  - [x] SQLite token accountant with budget alerts
  - [x] Git worktree orchestrator with post-merge verification
  - [x] Mock agent process manager with telemetry parsing and crash respawn
  - [x] gRPC transport skeleton (events, commands, full-state)
  - [x] Daemon engine loop with merge queue and budget hard-kill
  - [x] `nexode-ctl` CLI client (status, watch, dispatch)

### M1: Sprint 1 — WAL Recovery + Agent Harness ✅
- **Target:** 2026-03-29
- **Status:** Complete (merged 2026-03-15)
- **Agent:** gpt (Codex)
- **Review:** `docs/reviews/sprint-1-review.md`
- **Deliverables:**
  - [x] WAL persistence layer (`.nexode/wal.binlog`, framed binary, CRC integrity)
  - [x] Crash recovery (checkpoint scan, WAL replay, PID check, worktree verification)
  - [x] Engine integration (WAL writes on state changes, periodic checkpoint)
  - [x] `AgentHarness` trait + `MockHarness` (backward compat with Phase 0 tests)
  - [x] `ClaudeCodeHarness` (claude CLI, CLAUDE.md context injection)
  - [x] `CodexCliHarness` (codex CLI, .codex context injection)
  - [x] Basic context compiler (task + globs + git diff + README)
  - [x] Harness selection (model inference + explicit override in session.yaml)

### M2a: Sprint 2 — Real Agent Integration + Critical Fixes ✅
- **Target:** 2026-03-29
- **Status:** Complete (merged 2026-03-15)
- **Agent:** gpt (Codex)
- **Review:** `docs/reviews/sprint-2-review.md`
- **Deliverables:**
  - [x] Fix I-009: `completion_detected` semantics (non-zero exit = failure)
  - [x] Fix I-010: Emit `AgentStateChanged(Executing)` after agent swap
  - [x] Fix I-015: JSON parsing for completion detection (replace substring matching)
  - [x] R-007: Command acknowledgment (oneshot request/response, `CommandOutcome` enum)
  - [x] Live smoke tests for ClaudeCode and CodexCli harnesses (`--features live-test`)
  - [x] End-to-end demo script (`scripts/demo.sh`)

### M2b: Codex CLI Verification ✅
- **Target:** 2026-03-15
- **Status:** Complete (merged 2026-03-15)
- **Agent:** gpt (Codex)
- **Review:** `docs/reviews/codex-verify-review.md`
- **Deliverables:**
  - [x] Live Codex CLI smoke test (`live_codex_cli_hello_world`)
  - [x] Forced-Codex full lifecycle test (`live_full_lifecycle`)
  - [x] Codex completion detection aligned to real `turn.completed` output
  - [x] Codex telemetry parsing aligned to real usage fields
  - [x] Default model path for Codex (no `--model` flag when `"default"`)
  - [x] Demo script with `codex-cli` harness

### M2c: Sprint 3 — Observer Loops + Safety 🔄
- **Target:** 2026-03-29
- **Status:** In Progress
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-3-codex.md`
- **Deliverables:**
  - [ ] Loop detection (`LoopDetector` in `observer.rs` — repeated output, stuck timeout, budget velocity)
  - [ ] Sandbox enforcement (`SandboxGuard` — worktree boundary checks, symlink escape prevention)
  - [ ] Event sequence numbers (R-005 fix — monotonic counter, gap detection, state catch-up)
  - [ ] Uncertainty routing (agent "I'm stuck" detection, auto-pause, operator resume)

### M3: Phase 2 — TUI + VS Code Extension ⏳
- **Target:** TBD
- **Status:** Not Started
- **Deliverables:**
  - [ ] Terminal UI for session monitoring
  - [ ] VS Code extension with embedded panel
  - [ ] Real-time slot status visualization
  - [ ] Interactive command dispatch from TUI/extension

### M4: Phase 3 — Multi-Project Orchestration ⏳
- **Target:** TBD
- **Status:** Not Started
- **Deliverables:**
  - [ ] Cross-project dependency tracking
  - [ ] Constellation-level scheduling
  - [ ] Resource allocation across projects
  - [ ] Multi-repo merge coordination

### M5: Phase 4 — Advanced Context ⏳
- **Target:** TBD
- **Status:** Not Started
- **Deliverables:**
  - [ ] AST indexing for codebase understanding
  - [ ] Vector search for semantic context retrieval
  - [ ] Embedding-based context compilation
  - [ ] Adaptive context budgeting per agent

## Backlog

> Unscheduled ideas and future work. Prioritize by moving into a milestone.

- [ ] `provider_config` deep merge for session cascade (I-004)
- [ ] SQLite schema migration versioning (I-005)
- [ ] Merge queue immediate drain on event (I-007)
- [ ] Replace manual arg parsing with clap (I-008)
- [ ] Engine module decomposition (engine.rs is 64KB)
- [ ] Agent process re-attachment via PID (Option B recovery — deferred from Sprint 1)
- [ ] Remote agent harness (SSH/container-based invocation)
- [ ] Composite harness (chain multiple agents per slot)
- [ ] Agent-initiated branching (agent creates sub-branches within worktree)
- [ ] Cost prediction before agent dispatch
- [ ] Telemetry format documentation (R-003)
- [ ] VS Code Extension Host IPC mitigation (R-008 — gRPC bypass for agent streams)
- [ ] Pre-merge semantic conflict detection (R-009 — AST signature comparison)
- [ ] Harness version pinning / self-test (R-010)
- [ ] Phase 5 / Pool requirements (see `docs/spec/deferred.md`)
