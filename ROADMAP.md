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

### M3a-cleanup: Sprint 8 — Daemon Hardening + Issue Sweep ✅
- **Target:** 2026-04-26
- **Status:** Complete (merged 2026-03-17, PR #20, commit `eab7705`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-8-codex.md`
- **Review:** `docs/reviews/sprint-8-review.md`
- **Deliverables:**
  - [x] I-020: Guard `observe_output` against unknown/removed slots
  - [x] I-021: Configurable alert cooldown for repeated observer findings
  - [x] I-023: Filter URLs and source-location patterns from sandbox candidate paths
  - [x] I-024: Add `finding_kind` enum to `LoopDetected` proto message
  - [x] I-029: Update Claude harness doc with `--permission-mode` flags
  - [x] I-013: Reject malformed `TOKENS` lines with no valid key=value pairs
  - [x] Add MSRV documentation (R-006) to Cargo.toml and README
  - [x] Daemon integration test: TUI reconnect after daemon restart

### M3b: Sprint 9 — VS Code Extension Scaffold ✅
- **Target:** 2026-05-03
- **Status:** Complete (merged 2026-03-18, PR #21, commit `0c8cee4`)
- **Agent:** gpt (Codex)
- **Prompt:** `.agents/prompts/sprint-9-codex.md`
- **Review:** `docs/reviews/sprint-9-review.md`
- **Deliverables:**
  - [x] Extension scaffold: `extensions/nexode-vscode/` TypeScript project
  - [x] gRPC daemon client with exponential-backoff reconnect
  - [x] Local state cache with snapshot + event-driven updates (EventBus pattern)
  - [x] TreeView slot browser (project → slot hierarchy, color-coded status icons)
  - [x] Status Bar HUD (connection state, agent count, token count, session cost)
  - [x] Command palette: pause/resume/move via QuickPick selectors
  - [x] Configuration settings: daemonHost, daemonPort with live reload
  - [x] Build tooling: esbuild bundler, TypeScript strict mode, runtime proto loading

### M3b-next: Sprint 10 — React Webviews + Extension Tests ⏳
- **Target:** TBD
- **Status:** Tranches A+B complete. Tranche C pending.
- **Agent:** gpt (Codex)
- **Reviews:** `docs/reviews/sprint-10a-review.md`, `docs/reviews/sprint-10b-review.md`
- **Tranche A Deliverables (complete — PR #22, commit `4bfe2ff`):**
  - [x] Webview build pipeline (esbuild, IIFE + browser target, minified React bundles)
  - [x] SynapseGridPanel, SynapseSidebarProvider, KanbanPanel shells
  - [x] Shared postMessage bridge with nonce-based CSP
  - [x] React 18 entry points for Synapse Grid and Kanban surfaces
  - [x] state.ts: Emitter<T> replaces vscode.EventEmitter for testability
  - [x] state.ts: Full Phase 3 observer event normalization
  - [x] Tier 1 unit tests for state.ts (251 lines, 4 test cases)
  - [x] D-012: MoveTask vs AssignTask command semantics
- **Tranche B Deliverables (complete — PR #23, commit `9b1a8a8`):**
  - [x] Synapse Grid: live state rendering with joined slot/task/project data, agent/status pills, metric header
  - [x] Macro Kanban: live state rendering with HTML5 drag-and-drop column moves via MoveTask dispatch
  - [x] Task card join (view-models.ts: buildSlotCardModels, buildKanbanCardModels)
  - [x] StateCache agent tracking (AgentPresence, seedAgents, agent selectors)
  - [x] Tier 1 test expansion (+7 test cases across 3 files, ~11 total)
  - [x] Tranche A review follow-ups closed (F-01 race fix, F-03 join, F-09 CSP preserved)
- **Tranche C Deliverables (pending):**
  - [ ] Synapse Grid: Flat View, Focus View mode switcher
  - [ ] Shared webview formatter extraction (eliminate duplicate utility functions)
  - [ ] Observer alert display in webviews/notifications
  - [ ] Rich per-cell presentation (spark-lines, progress bars)
  - [ ] Extension host integration tests (Tier 2, if feasible)
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

### Quality Governance — Strict Score (decomposed across milestones)

> Inspired by desloppify (Peter O'Malley). At 15+ parallel agents, quality drift
> is the dominant failure mode. This feature decomposes into three pieces shipped
> incrementally — not as a single monolithic milestone.

#### QG-1: Generic Score Gate Hook (Backlog → Sprint 11/12) ⏳
- **Target:** Near-term (no dependencies)
- **Status:** Backlog
- **Size:** ~100 LOC Rust + proto event
- **What:** Add a `score_command` field to the `verify` config in `session.yaml`. Before merge, the daemon runs the command against the worktree diff. Non-zero exit = merge rejected.
- **Deliverables:**
  - [ ] `score_command` field in `VerifyConfig` (session.yaml `verify.score_command`)
  - [ ] `pre_merge_score_check()` in `engine/merge.rs`, called between MERGE_QUEUE admission and `merge_and_verify()`
  - [ ] `ScoreGateRejected` event type in `HypervisorEvent` proto (slot_id, exit_code, stderr)
  - [ ] `TASK_STATUS_ATTESTING` added to `TaskStatus` enum — kanban flow becomes `REVIEW → ATTESTING → MERGE_QUEUE`
  - [ ] `ForceAttest` operator command to override a rejected gate
  - [ ] VS Code + TUI render the ATTESTING state (color: amber)
- **Why this ships first:** It’s pure infrastructure. Users can plug in desloppify, clippy, eslint, `cargo test`, or any external tool today. No Tree-sitter, no LLM, no new daemon subsystem. Example config:
  ```yaml
  projects:
    - id: backend
      verify:
        commands: ["cargo test"]
        score_command: "desloppify scan --exit-code --min-score 60"
  ```

#### QG-2: Built-in Mechanical Scanner (M5 sub-deliverable) ⏳
- **Target:** Ships with Phase 4 Tree-sitter work (M5)
- **Status:** Planned (depends on M5 Step 1)
- **Size:** ~500 LOC Rust module
- **What:** When Tree-sitter AST indexing lands in Phase 4, add a daemon-internal scoring path that replaces the shell-out for mechanical checks. Deterministic, per-file, cacheable.
- **Deliverables:**
  - [ ] `scoring/mechanical.rs` module in `nexode-daemon`
    - Cyclomatic complexity per function (Tree-sitter CFG walk)
    - Duplication detection (AST signature hashing across files)
    - Dead code candidates (unreferenced public symbols)
    - Type coverage ratio (for TS/Python projects with type annotations)
  - [ ] Per-file numeric score cached in SQLite alongside token accounting
  - [ ] `strict_score` field on `Project` proto message (aggregate mechanical score)
  - [ ] `StrictScoreUpdated` event emitted on score change
  - [ ] Daemon-internal score gate path (bypass shell-out when built-in scanner is configured)
  - [ ] VS Code Status Bar: display `strict_score` next to session cost
  - [ ] TUI: score column in slot detail pane
- **Why this waits for M5:** The Tree-sitter parse trees are the input. Building a parallel AST pipeline would be redundant with Phase 4 Step 1.

#### QG-3: Adversarial Subjective Reviewer (spike + delivery) ⏳
- **Target:** Post-M5 (spike: 1-2 weeks; delivery: 3-4 weeks)
- **Status:** Feature Request (requires spike validation)
- **Motivation:** Mechanical scanning catches complexity and duplication but misses abstraction quality, naming coherence, and architectural taste. The key insight from desloppify: treat "taste" as a measurable, scoreable signal. The key insight for Nexode: use a different model family than the coding agent so the reviewer doesn’t share the author’s blindspots.

**Tiered reviewer architecture:**

| Tier | Engine | Runs when | Latency | Token cost | Purpose |
|---|---|---|---|---|---|
| T0 | Tree-sitter (QG-2) | Every commit | <100ms | Zero | Mechanical: complexity, duplication, dead code |
| T1 | Local LLM via llama.cpp | Every merge candidate | 3-8s/file | Zero (local) | Subjective: naming, cohesion, abstraction quality |
| T2 | Cloud LLM (cross-family) | Escalation only | 10-30s | API tokens | Adversarial: different training data, different blindspots |

**T1 — Local LLM (default subjective reviewer):**
- **Model:** `qwen3-35b-a3b` (35B MoE, 3B active parameters) via llama.cpp
- **Why this model:** MoE architecture means large model quality at small model inference cost. Only 3B active params per token → 40-60 tok/s on the Spark’s GPU. At 15 agents merging every 30-60 min, the Spark handles review load without contention.
- **Integration:** New `LocalLlmHarness` implementing the `AgentHarness` trait. Connects to a llama.cpp server (already running for other local inference tasks) or spawns one. Sends a structured review prompt with the diff + surrounding AST context. Parses structured JSON response with per-file scores and findings.
- **Prompt contract:** The reviewer receives: (1) the diff, (2) AST signatures of changed functions from QG-2, (3) a scoring rubric (naming 0-25, cohesion 0-25, abstraction 0-25, consistency 0-25). Returns JSON with scores + one-line justifications per dimension.

**T2 — Cloud LLM (adversarial escalation):**
- **Trigger:** T1 score lands in a configurable gray zone (e.g., 50-70) or operator sets `tier2_required: true` on high-stakes projects.
- **Cross-family constraint:** If the coding agent used Claude, the reviewer uses Gemini or GPT (and vice versa). Different training data → different systematic biases → genuine adversarial signal. Configured per-project in `session.yaml`.
- **Why cross-family matters:** Same-model review is theater. Claude reviewing Claude’s output shares biases in abstraction style, naming conventions, and error-handling patterns. Cross-family review is the only way to catch model-specific slop.

**Deliverables:**
  - [ ] **Spike (1-2 weeks):** Validate T1 scoring quality
    - Run qwen3-35b-a3b against 20 real diffs from Sprints 1-9
    - Compare local model scores vs human assessment
    - Measure latency per file on the Spark
    - Determine if score normalization is needed (local vs cloud calibration)
    - Kill criteria: if T1 scores correlate <0.5 with human judgment, defer to T2-only
  - [ ] **LocalLlmHarness** in `nexode-daemon`: `AgentHarness` impl for llama.cpp server
    - HTTP API to llama.cpp `/completion` endpoint
    - Structured prompt template with diff + AST context + rubric
    - JSON response parsing with score extraction
  - [ ] **Reviewer observer loop**: fourth loop alongside heartbeat/budget/semantic
    - Triggers on `TASK_STATUS_ATTESTING` entry
    - Runs T0 (instant) → T1 (local LLM) → conditionally T2 (cloud)
    - Composite score = weighted(T0 mechanical, T1/T2 subjective)
    - Emits `StrictScoreUpdated` with tier breakdown
  - [ ] **`contribution_score`** field on `AgentSlot` proto message
    - Rolling average of per-merge score deltas attributed to each agent
    - VS Code Fleet View / Synapse Grid: per-agent score column
    - TUI: score in slot detail tooltip
  - [ ] **Resolve & Attest workflow**
    - On gate rejection: agent receives structured feedback (which dimensions scored low, specific findings)
    - Agent must produce a structured attestation (JSON: which rules tripped, why delta is justified)
    - Attestation stored in WAL for audit trail
    - Operator `ForceAttest` command bypasses with logged override
  - [ ] **session.yaml schema:**
    ```yaml
    quality:
      mechanical:
        enabled: true
        max_complexity: 15
        max_duplication_ratio: 0.08
      reviewer:
        tier1:
          model: "llama-local/qwen3-35b-a3b"
          endpoint: "http://localhost:8080"  # llama.cpp server
          timeout_seconds: 30
        tier2:
          model: "claude-sonnet"  # cross-family adversarial
          trigger: "score < 70 or delta < -5"
        cross_family: true  # enforce different model family than coding agent
      gate:
        enabled: true
        min_score: 60
        block_on_regression: true
    ```
- **Open questions (resolve in spike):**
  - Quantization: Q4_K_M vs Q8_0 for qwen3-35b-a3b review quality? (Q4 is faster, Q8 may score more accurately on subjective dimensions)
  - Score normalization: are T1 local scores directly comparable to T2 cloud scores, or do we need a calibration layer?
  - Incremental scoring: score only changed files (fast, but misses cross-file coherence) vs re-score affected module (slower, catches ripple effects)?
  - Attestation format: structured JSON the daemon parses for auto-retry, or free-form markdown the operator reads?
  - llama.cpp lifecycle: daemon manages the llama.cpp process, or assume it’s externally managed?

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
- [ ] QG-1: Generic Score Gate hook in merge queue (near-term, no dependencies)
- [ ] QG-2: Built-in mechanical scanner using Tree-sitter (ships with M5/Phase 4)
- [ ] QG-3: Adversarial subjective reviewer — local LLM (qwen3-35b-a3b) + cross-family cloud escalation (spike then delivery, post-M5)
- [ ] LocalLlmHarness — `AgentHarness` impl for llama.cpp server (QG-3 dependency)
