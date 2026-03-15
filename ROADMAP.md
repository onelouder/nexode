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

### M1: Sprint 1 — WAL Recovery + Agent Harness 🔄
- **Target:** 2026-03-29
- **Status:** In Progress
- **Agent:** gpt (Codex)
- **Branch:** `agent/gpt/sprint-1-wal-harness`
- **Deliverables:**
  - [ ] WAL persistence layer (`.nexode/wal.binlog`, framed binary, CRC integrity)
  - [ ] Crash recovery (checkpoint scan, WAL replay, PID check, worktree verification)
  - [ ] Engine integration (WAL writes on state changes, periodic checkpoint)
  - [ ] `AgentHarness` trait + `MockHarness` (backward compat with Phase 0 tests)
  - [ ] `ClaudeCodeHarness` (claude CLI, CLAUDE.md context injection)
  - [ ] `CodexCliHarness` (codex CLI, .codex context injection)
  - [ ] Basic context compiler (task + globs + git diff + README)
  - [ ] Harness selection (model inference + explicit override in session.yaml)

### M2: Sprint 2 — Observer Loops + Safety ⏳
- **Target:** TBD
- **Status:** Not Started
- **Deliverables:**
  - [ ] Uncertainty routing (agent can signal "I'm stuck" or "I need guidance")
  - [ ] Loop detection (detect agent spinning without progress)
  - [ ] Sandbox enforcement (agent can't break out of worktree)
  - [ ] Unattended operation soak test (24-hour run)
  - [ ] Event sequence numbers (fix R-005 broadcast stream drops)
  - [ ] Command acknowledgment (fix R-007 fire-and-forget)

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
- [ ] Agent process re-attachment via PID (Option B recovery — deferred from Sprint 1)
- [ ] Remote agent harness (SSH/container-based invocation)
- [ ] Composite harness (chain multiple agents per slot)
- [ ] Agent-initiated branching (agent creates sub-branches within worktree)
- [ ] Cost prediction before agent dispatch
- [ ] Phase 5 / Pool requirements (see `docs/spec/deferred.md`)
