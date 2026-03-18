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

### M2c: Sprint 3 — Observer Loops + Safety ✅
- **Target:** 2026-03-29
- **Status:** Complete (merged 2026-03-15, commit `9371feb`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-3-codex.md`
- **Review:** `docs/reviews/sprint-3-review.md`
- **Deliverables:**
  - [x] Loop detection (`LoopDetector` in `observer.rs` — repeated output, stuck timeout, budget velocity)
  - [x] Sandbox enforcement (`SandboxGuard` — worktree boundary checks, symlink escape prevention)
  - [x] Event sequence numbers (R-005 fix — monotonic counter, gap detection, state catch-up)
  - [x] Uncertainty routing (agent "I'm stuck" detection, auto-pause, operator resume)

### M2d: Sprint 4 — Engine Hardening + Module Decomposition ✅
- **Target:** 2026-03-29
- **Status:** Complete (merged 2026-03-15, commit `ee82552`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-4-codex.md`
- **Review:** `docs/reviews/sprint-4-review.md`
- **Deliverables:**
  - [x] Engine module decomposition (`engine.rs` → `engine/` directory with 8 sub-modules)
  - [x] Fix I-016: Task transition semantics (pre-pause state tracking)
  - [x] Fix I-022: Async observer tick (`JoinSet::spawn_blocking`)
  - [x] Fix I-008: Daemon CLI with clap

### M3a: Sprint 5 — TUI Dashboard ✅
- **Target:** 2026-04-05
- **Status:** Complete (merged 2026-03-15, commit `4e5f6cf`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-5-codex.md`
- **Review:** `docs/reviews/sprint-5-review.md`
- **Deliverables:**
  - [x] New `nexode-tui` crate with `ratatui` + `crossterm`
  - [x] Three-panel dashboard: project tree, slot detail, event log
  - [x] Live gRPC event streaming with gap recovery
  - [x] Interactive controls: navigate, pause/resume/kill, command mode
  - [x] Graceful terminal handling (raw mode, panic cleanup, signal handler)
  - [x] 18 unit tests, status colors aligned to kanban spec (D-009)

### M3a-polish: Sprint 6 — Integration Polish ✅
- **Target:** 2026-04-12
- **Status:** Complete (merged 2026-03-17, commit `3ae2ffd`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-6-codex.md`
- **Review:** `docs/reviews/sprint-6-review.md`
- **Deliverables:**
  - [x] Fix I-027: Event gap recovery drops triggering event
  - [x] Fix I-028: Timezone offset at startup (not multi-threaded)
  - [x] Fix I-025: `resume_target()` handles Review state
  - [x] Fix I-007: Immediate merge queue drain on enqueue
  - [x] Cross-crate integration test (daemon→TUI via gRPC)
  - [x] CLI cleanup: `--version` flags, I-014 doc fix

### M3a-harden: Sprint 7 — TUI Command Hardening ✅
- **Target:** 2026-04-19
- **Status:** Complete (merged 2026-03-17, PR #19, commit `a93e9af`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-7-codex.md`
- **Review:** `docs/reviews/sprint-7-review.md`
- **Deliverables:**
  - [x] Auto-reconnect on gRPC disconnect with backoff and status indicator
  - [x] Command history (↑/↓ in command mode)
  - [x] Status bar feedback with auto-clear
  - [x] Tab-complete for slot IDs
  - [x] Help overlay (`?` key)
  - [x] Fix I-019: demo.sh waits for DONE
  - [x] Improve I-024: parse LoopDetected reason strings

### M3a-cleanup: Sprint 8 — Daemon Hardening + Issue Sweep ⏳
- **Target:** 2026-04-26
- **Status:** Ready for Codex
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-8-codex.md`
- **Deliverables:**
  - [ ] I-020: Guard `observe_output` against unknown/removed slots
  - [ ] I-021: Configurable alert cooldown for repeated observer findings
  - [ ] I-023: Filter URLs and source-location patterns from sandbox candidate paths
  - [ ] I-024: Add `finding_kind` enum to `LoopDetected` proto message
  - [ ] I-029: Update Claude harness doc with `--permission-mode` flags
  - [ ] I-013: Reject malformed `TOKENS` lines with no valid key=value pairs
  - [ ] Add MSRV documentation (R-006) to Cargo.toml and README
  - [ ] Daemon integration test: TUI reconnect after daemon restart

### M3b: Phase 2 Continuation — VS Code Extension ⏳
- **Target:** TBD
- **Status:** Not Started
- **Deliverables:**
  - [ ] VS Code extension with embedded webview panel
  - [ ] Real-time slot status visualization via gRPC/WebSocket
  - [ ] Interactive command dispatch from extension
  - [ ] R-008 mitigation: bypass Extension Host for agent data streams

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
- [x] Merge queue immediate drain on event (I-007) → Sprint 6
- [x] Replace manual arg parsing with clap (I-008) → Sprint 4
- [x] Engine module decomposition (engine.rs is 64KB) → Sprint 4
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
