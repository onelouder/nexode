---
spec_version: "2.0.1-locked"
locked_date: "2026-03-14"
status: "locked-for-decomposition"
---

# Nexode Agent IDE — Master Specification v2

**Architecture and Implementation Blueprint**

Supersedes: Master Specification v1 (March 2026)
Incorporates: Errata 001 (E-001 through E-011), Triage Dispositions
Author: jwells@gmail.com / Xcognis + Perplexity Computer
Date: March 13, 2026

This document is the authoritative engineering specification for the Nexode Agent IDE. It
incorporates all accepted errata items (7 ACCEPT, 4 MODIFY) from the multi-project
orchestration review, the revised session.yaml v2 schema, and the updated Phase 0-1
implementation plan. Where the original specification described single-codebase swarm
orchestration, this revision reframes Nexode as a multi-project agent command center: 10-15
agents working across 10-15 separate codebases and tasks.

## Contents

- Executive Summary and Core Philosophy
- System Architecture (4-Layer Model)
- Domain Types and State Management
  - 3.1 Domain Model Hierarchy
  - 3.2 Fungible Workers and AgentSlot Abstraction
  - 3.3 Shared Memory Bus
  - 3.4 hypervisor.proto v2 Contract
- Session Configuration (session.yaml v2)
  - 4.1 Design Principles
  - 4.2 Complete Schema
  - 4.3 Field Reference
  - 4.4 Per-Project .nexode.yaml
  - 4.5 Backward Compatibility
- UI/UX Surfaces (The Command Center)
- Orchestration and Autonomy
- Phased Development Timeline (Overview)
- Phase 0: Spike and Validate (2 Weeks)
- 9. Phase 1: Headless Orchestrator (4-6 Weeks)
- Phase 2: TUI Command Center (4-6 Weeks)
- Phase 3: VS Code Integration (6-8 Weeks)
- Phase 4: Smart Context and Semantic Memory (6-8 Weeks)
- Phase 5: Deep Fork (Conditional)
- Appendices
  - A Agent Pools and .swarm/ Protocol (Phase 3+)
  - B Licensing and Distribution Strategy
  - C Karpathy Alignment Matrix
  - D Competitive Differentiation
  - E Errata Incorporation Log

---

<a id="sec-01-executive-summary-core-philosophy"></a>
## 1. Executive Summary and Core Philosophy

The Nexode Agent IDE shifts the paradigm of developer tooling from file-centric editing to agent-centric
multi-project orchestration. Designed to fulfill the vision of an "agent command center" (Karpathy, 2025),
Nexode enables a single developer to effectively manage 10 to 15 parallel autonomous coding agents
(Claude Code, Codex, Gemini CLI, and local models) across 10 to 15 separate codebases and tasks.

> **Core Reframing (v2):** The primary use case is not 15 agents on one codebase. It is 10-15 agents
> distributed across 10-15 separate projects: a SaaS backend, a CLI tool, a client API, documentation,
> and a research spike, all running simultaneously under a single orchestration session. This is the
> pattern described by Karpathy, practiced by power users and consultants, and unserved by any
> existing tool.

Nexode is optimized for horizontal task distribution. Rather than forcing agents to compete for real-time
semantic locks within the same file — which introduces high research risk and code quality degradation —
agents are isolated in Git worktrees to execute separate, loosely coupled tasks. Nexode serves as "tmux
with a brain," providing rich visual orchestration, shared memory, and a central control plane that spans
multiple repositories.

<a id="sec-01-user-personas"></a>
### User Personas

| Persona | Agents | Projects | Key Need |
|---|---|---|---|
| Solo Developer | 2-5 | 1-3 | Run agents across front-end, back-end, and tests in parallel. Single dashboard. |
| Power User / Indie Hacker | 5-10 | 3-8 | Manage multiple personal projects simultaneously. Per-project cost tracking. |
| Consultant / Freelancer | 10-15 | 8-15 | Run agents on separate client codebases. Budget isolation per client project. |
| Team Lead (Future) | 15+ | 5-10 | Orchestrate agents across team repos. Shared decision log. Phase 3+ scope. |

<a id="sec-01-design-principles"></a>
### Design Principles

- **Extension-first, fork-never (for now):** Build as a VS Code extension pack, not a fork. The 2.1M-line VS Code codebase is a maintenance liability. Phase 5 (Deep Fork) is conditional.
- **Daemon-first, UI-second:** The Rust hypervisor daemon is the product. The TUI and VS Code extension are rendering shells. If the daemon works headlessly, every UI is optional.
- **Multi-project as the primitive:** The session schema, domain model, cost tracking, and UI grouping all assume multiple projects. Single-project is a degenerate case that works automatically.
- **Open source as structural moat:** The core daemon and TUI are MIT/Apache 2.0. The VS Code extension is freemium. See Appendix B.
- **Model-agnostic orchestration:** Nexode treats agent CLIs as fungible compute. Claude Code, Codex, Gemini CLI, and local models are interchangeable via a harness abstraction.

---

<a id="sec-02-system-architecture-4-layer-model"></a>
## 2. System Architecture (The 4-Layer Model)

To avoid the massive maintenance burden of deeply forking VS Code's ~2.1M lines of TypeScript, Nexode
utilizes an "extension-first, fork-never (for now)" architecture. The system is organized into four cleanly
decoupled layers.

| Layer | Name | Description |
|---|---|---|
| Layer 0 | Substrate | The foundation: OS file system, Git repository internals, LLM provider APIs, and Model Context Protocol (MCP) servers. This layer exists independent of Nexode. |
| Layer 1 | Hypervisor Daemon | A standalone Rust binary using the tokio async runtime. Manages: OS process lifecycle, multi-repo Git worktree orchestration, vector memory (LanceDB), token accounting (SQLite), the DAG workflow engine, and the OrchestratorAgent control loop. This is the brain of Nexode. |
| Layer 2 | gRPC Bridge | High-throughput bidirectional IPC (Unix socket or named pipe). Streams agent output tokens, telemetry, and state mutations directly to the renderer, bypassing the VS Code Extension Host bottleneck. |
| Layer 3 | Rendering Shell | A VS Code Extension Pack containing modular UI surfaces. Renders the multi-monitor Synapse Telemetry Grid, DAG Kanban visualizer, and agent command chat using standard VS Code API endpoints. The TUI (Phase 2) is an alternative Layer 3 renderer. |

<a id="sec-02-key-architectural-property"></a>
### Key Architectural Property

The daemon (Layer 1) is completely decoupled from the UI (Layer 3). The daemon could run on a DGX
Spark in a server rack while the VS Code extension or TUI connects to it remotely via the gRPC socket.
This decoupling also means that multiple rendering shells (TUI, VS Code, a future web dashboard) can
connect simultaneously.

<a id="sec-02-layer-1-rust-daemon-core-architecture"></a>
### Layer 1: Rust Daemon Core Architecture

At the heart of the daemon is a central message-passing architecture using `tokio::sync::mpsc`
(Multi-Producer, Single-Consumer) channels. No shared mutable state. Three primary concurrent loops:

- **gRPC Server Loop (Tonic):** Listens on the Unix socket for `OperatorCommand` messages from any connected UI. Pushes commands to the Core Engine via an mpsc channel.
- **Agent Process Runners:** Up to 15 individual async tasks, one per active CLI agent. Read from the agent's stdout/stderr, parse output, and stream telemetry to the Core Engine.
- **Core Engine and Orchestrator Loop:** The master state machine. Receives UI commands, agent outputs, and cron triggers. Updates the SQLite database and broadcasts `HypervisorEvent` payloads to connected UIs.

<a id="sec-02-daemon-subsystem-breakdown"></a>
### Daemon Subsystem Breakdown

| Component | Responsibility | Implementation |
|---|---|---|
| Agent Process Manager | Spawns, monitors, and terminates fungible CLI agent processes. Handles heartbeats and watchdog timeouts. | OS process groups + Unix signals via `tokio::process`. |
| Git Worktree Orchestrator | Creates, assigns, merges, and garbage-collects isolated git worktrees across multiple repositories. | Native Git bindings (git2 or gix). |
| Scoped Context Compiler | Queries the Vector Memory Bus to build targeted, role-specific prompts for blank agents before dispatch. | LanceDB (vector search) + Tree-sitter (AST parsing). Phase 4. |
| Workflow DAG Engine | Manages Kanban columns, parses session.yaml task slots, and tracks task completion states. | In-memory graph resolving dependencies. |
| Token Accountant | Tracks per-agent, per-slot, per-project, and per-session token usage and cost estimates. | SQLite append-only log with project_id and slot_id columns. |
| Session Config Manager | Parses session.yaml v2 with defaults cascade, include directives, .nexode.yaml merging, and v1 fallback. | serde YAML deserialization with strict validation. |

### Conceptual Rust Skeleton

**Rust — main.rs (Conceptual)**

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Substrates
    let db_pool = initialize_sqlite_accounting().await;
    let vector_db = initialize_lancedb_memory().await;
    let session = parse_session_yaml("~/.nexode/session.yaml")?;
    // 2. Create the core communication channels
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<OperatorCommand>(100);
    let (event_tx, event_rx) = broadcast::channel::<HypervisorEvent>(100);
    // 3. Spawn the gRPC Server (Layer 2 Bridge)
    tokio::spawn(async move {
        run_grpc_server(cmd_tx, event_rx).await;
    });
    // 4. The Core Engine Loop
    let mut orchestrator = Orchestrator::new(db_pool, vector_db, session);
    let mut tick_interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                orchestrator.handle_command(cmd, &event_tx).await;
            }
            _ = tick_interval.tick() => {
                orchestrator.evaluate_slots_and_dispatch(&event_tx).await;
                orchestrator.run_observer_checks(&event_tx).await;
            }
        }
    }
}
```

---

<a id="sec-03-domain-types-state-management"></a>
## 3. Domain Types and State Management

<a id="sec-03-01-domain-model-hierarchy"></a>
### 3.1 Domain Model Hierarchy

> **New in v2 (E-003):** The domain model introduces two new types: Project and AgentSlot. The hierarchy
> is NexodeSession > Project > AgentSlot > Agent. AgentSlot is the stable identity that survives agent
> crashes, giving the UI, cost tracker, and Kanban Board a fixed reference point while allowing the
> underlying agent process to be recycled freely.

| Type | Lifecycle | Owns | Purpose |
|---|---|---|---|
| NexodeSession | Daemon start to stop | projects[], budget, defaults | Top-level container. One per daemon run. Parsed from session.yaml. |
| Project | Session lifetime | repo, slots[], budget, color, tags | A single codebase or task collection. Maps to one Git repository (or none for non-git tasks). |
| AgentSlot | Session lifetime | task, mode, branch, context, current_agent | The stable work unit. Represents 'what needs doing.' Survives agent crashes. Cost accrues to the slot. |
| Agent | Process spawn to exit | process handle, stdout/stderr streams | The ephemeral compute worker. A raw CLI process (Claude, Codex, etc.) bound to a slot. Replaceable. |
| TaskNode | Session lifetime | title, description, status, dependencies | A unit of work on the Kanban board. May map 1:1 with a slot or be subdivided. |
| Worktree | Slot lifetime | absolute_path, branch_name, conflict_risk | Isolated Git worktree directory managed by the daemon. One per active slot per git-backed project. |

<a id="sec-03-02-fungible-workers-agentslot-abstraction"></a>
### 3.2 Fungible Workers and the AgentSlot Abstraction

Nexode separates raw compute from context, allowing agents to act as interchangeable workers with
unique, task-specific identities.

- **Fungible Workers:** At the OS layer, agents are generic, spawned CLI processes. If an agent crashes or hangs, the daemon terminates it and spins up a blank replacement into the same AgentSlot. The slot ID, cost history, worktree, and task context are preserved.
- **Unique Identities (Scoped Context Compiler):** An agent gains identity upon slot assignment. The daemon uses Tree-sitter and vector search (Phase 4) to compile a role-appropriate context slice and injects it into the blank worker.
- **Crash Recovery:** When an agent process dies, the daemon detects the exit, spawns a replacement into the same slot, and emits a `SlotAgentSwapped` event. The UI sees slot continuity; the cost tracker continues accruing to the same `slot_id`.
- **O(1) Concurrency:** Each agent operates in a completely isolated Git worktree managed automatically by the daemon. No file-level locking needed.

<a id="sec-03-03-shared-memory-bus"></a>
### 3.3 Shared Memory Bus

While the EventBus (Layer 2) keeps the human in sync, the Vector Memory Bus keeps the fungible agents
in sync. Because we use isolated Git worktrees, agents cannot see what other agents are typing. The
memory architecture (fully implemented in Phase 4) is divided into three layers:

- **Codebase Graph (Code Intent):** Embeddings of AST nodes, dependency relationships, and import maps. The Scoped Context Compiler queries this graph to find structural signatures relevant to a task, reducing context window bloat by 20-40%.
- **Decision Log (Architectural Intent):** An append-only log of agent reasoning traces, architectural decisions, and rejected alternatives. When Agent B starts work after Agent A finishes, it queries LanceDB for past decisions and aligns accordingly.
- **Session State (Ephemeral):** Active tasks, agent assignments, and pending approvals. Stored in SQLite. Cleared when the session ends.

<a id="sec-03-04-hypervisor-proto-v2-contract"></a>
### 3.4 hypervisor.proto v2 Contract

The gRPC contract between the Rust Daemon (Layer 1) and any rendering shell (Layer 3). This is the
absolute source of truth. v2 adds Project, AgentSlot, ProjectBudgetAlert, and SlotAgentSwapped.

**Protocol Buffers — hypervisor.proto v2 (Service and Enums)**

```protobuf
syntax = "proto3";
package nexode.hypervisor.v2;
service Hypervisor {
    rpc SubscribeEvents(SubscribeRequest) returns (stream HypervisorEvent);
    rpc DispatchCommand(OperatorCommand) returns (CommandResponse);
    rpc GetFullState(StateRequest) returns (FullStateSnapshot);
}
enum AgentState {
    AGENT_STATE_UNSPECIFIED = 0;
    AGENT_STATE_INIT        = 1;  // Spawning process / provisioning worktree
    AGENT_STATE_IDLE        = 2;  // Waiting in the fungible worker pool
    AGENT_STATE_PLANNING    = 3;  // Compiling context / generating plan
    AGENT_STATE_EXECUTING   = 4;  // Writing code / running tools
    AGENT_STATE_REVIEW      = 5;  // Awaiting Orchestrator or Human approval
    AGENT_STATE_BLOCKED     = 6;  // Uncertainty flag (needs help)
    AGENT_STATE_TERMINATED  = 7;  // Process killed / worktree GC'd
}
enum TaskStatus {
    TASK_STATUS_UNSPECIFIED = 0;  TASK_STATUS_PENDING = 1;
    TASK_STATUS_WORKING = 2;  TASK_STATUS_REVIEW = 3;
    TASK_STATUS_DONE = 4;  TASK_STATUS_PAUSED = 5;  TASK_STATUS_ARCHIVED = 6;
}
enum AgentMode {
    AGENT_MODE_UNSPECIFIED = 0;  AGENT_MODE_NORMAL = 1;
    AGENT_MODE_PLAN = 2;  AGENT_MODE_FULL_AUTO = 3;
}
```

**Protocol Buffers — hypervisor.proto v2 (Entity Models)**

```protobuf
message Project {
    string id = 1;  string display_name = 2;  string repo_path = 3;
    string color = 4;  repeated string tags = 5;
    double budget_max_usd = 6;  double budget_warn_usd = 7;
    double current_cost_usd = 8;  repeated AgentSlot slots = 9;
}
message AgentSlot {
    string id = 1;  string project_id = 2;  string task = 3;
    AgentMode mode = 4;  string branch = 5;
    string current_agent_id = 6;  string worktree_id = 7;
    uint64 total_tokens = 8;  double total_cost_usd = 9;
}
message Agent {
    string id = 1;  string display_name = 2;  string current_role = 3;
    AgentState state = 4;  AgentMode mode = 5;
    string slot_id = 6;  string worktree_id = 7;
    uint64 tokens_consumed = 8;  double estimated_cost = 9;
    double tokens_per_sec = 10;
}
message TaskNode {
    string id = 1;  string title = 2;  string description = 3;
    TaskStatus status = 4;  string assigned_agent_id = 5;
    string project_id = 6;  // NEW in v2
    repeated string dependency_ids = 7;
}
message Worktree {
    string id = 1;  string absolute_path = 2;
    string branch_name = 3;  double conflict_risk = 4;
}
message SubscribeRequest { string client_version = 1; }
message StateRequest {}
message FullStateSnapshot {
    repeated Project projects = 1;  // v2: replaces flat agent list
    repeated TaskNode task_dag = 2;
    double total_session_cost = 3;  double session_budget_max_usd = 4;
}
```

**Protocol Buffers — hypervisor.proto v2 (Events and Commands)**

```protobuf
message HypervisorEvent {
    string event_id = 1;  uint64 timestamp_ms = 2;  string barrier_id = 3;
    oneof payload {
        AgentStateChanged agent_state_changed = 4;
        AgentTelemetryUpdated agent_telemetry_updated = 5;
        TaskStatusChanged task_status_changed = 6;
        UncertaintyFlagTriggered uncertainty_flag = 7;
        WorktreeStatusChanged worktree_status_changed = 8;
        ProjectBudgetAlert project_budget_alert = 9;   // NEW v2
        SlotAgentSwapped slot_agent_swapped = 10;       // NEW v2
    }
}
message AgentStateChanged  { string agent_id = 1; AgentState new_state = 2; }
message AgentTelemetryUpdated { string agent_id = 1; uint64 incr_tokens = 2; double tps = 3; }
message TaskStatusChanged  { string task_id = 1; TaskStatus new_status = 2; string agent_id = 3; }
message UncertaintyFlagTriggered { string agent_id = 1; string task_id = 2; string reason = 3; }
message WorktreeStatusChanged { string worktree_id = 1; double new_risk = 2; }
message ProjectBudgetAlert {
    string project_id = 1; double current_usd = 2; double limit_usd = 3;
    bool hard_kill = 4;  // true = at max. false = at warn.
}
message SlotAgentSwapped {
    string slot_id = 1; string old_agent_id = 2; string new_agent_id = 3;
    string reason = 4;  // "crash_recovery", "manual_reassign", "initial"
}
message OperatorCommand {
    string command_id = 1;
    oneof action {
        PauseAgent pause_agent = 2;  ResumeAgent resume_agent = 3;
        KillAgent kill_agent = 4;  MoveTask move_task = 5;
        AssignTask assign_task = 6;  SetAgentMode set_agent_mode = 7;
        ChatDispatch chat_dispatch = 8;  KillProject kill_project = 9;  // NEW v2
    }
}
message PauseAgent   { string agent_id = 1; }
message ResumeAgent  { string agent_id = 1; }
message KillAgent    { string agent_id = 1; }
message MoveTask     { string task_id = 1; TaskStatus target = 2; }
message AssignTask   { string task_id = 1; string agent_id = 2; }
message SetAgentMode { string agent_id = 1; AgentMode new_mode = 2; }
message ChatDispatch { string raw_nl = 1; }
message KillProject  { string project_id = 1; }  // Stops all agents
message CommandResponse { bool success = 1; string error_message = 2; }
```

**Design Highlights:** The `barrier_id` in `HypervisorEvent` ensures the TypeScript `EventBus` updates all React
Webviews simultaneously before rendering, preventing torn UI state. The `ChatDispatch` command wraps
natural language from the VS Code ChatBar (e.g., `@agent-3 pause`) and routes it to the `OrchestratorAgent`
for parsing.

---

<a id="sec-04-session-configuration-session-yaml-v2"></a>
## 4. Session Configuration (session.yaml v2)

> **New in v2 (E-004, as modified):** The session.yaml schema has been redesigned from scratch for the
> multi-project model. It adds: a defaults block for DRY configuration, include directives for splitting
> large sessions across files, per-project .nexode.yaml overrides, tags for filtering, and a model pricing
> table.

<a id="sec-04-01-design-principles"></a>
### 4.1 Design Principles

- **Convention over configuration:** A session with one project and one agent should require fewer than 10 lines of YAML. Defaults block eliminates repetition.
- **Layered configuration:** Global `~/.nexode/session.yaml` defines the portfolio. Per-project `.nexode.yaml` in repo roots defines slots. The daemon merges both, with per-project overriding global defaults. Mirrors `.gitconfig` / `.git/config` layering.
- **Include directives:** Large sessions can split project definitions into separate files for readability.
- **v1 backward compatibility:** A YAML file without a `projects[]` key is treated as a single implicit project. The version field disambiguates.
- **Human-friendly identifiers:** Project and slot IDs are short kebab-case strings chosen by the user, not UUIDs.

<a id="sec-04-02-complete-schema-annotations"></a>
### 4.2 Complete Schema with Annotations

**YAML — ~/.nexode/session.yaml v2 (Session and Model Config)**

```yaml
version: "2.0"                  # Schema version. "1.0" = legacy single-project.
session:
  name: "Thursday Sprint"       # Human label. Shown in TUI header and HUD.
  budget:
    max_usd: 50.00              # Hard kill: all agents stop when session hits this.
    warn_usd: 35.00             # Soft alert: TUI/Grid flashes warning.
defaults:
  model: "claude-code"        # Default agent CLI. Overridable per-slot.
  mode: "plan"                # Default autonomy tier: manual | plan | full_auto
  timeout_minutes: 120        # Kill agent if idle beyond this duration.
  provider_config:
    claude: "$ANTHROPIC_API_KEY"
    codex: "$OPENAI_API_KEY"
    gemini: "$GOOGLE_API_KEY"
models:                         # Per-model pricing (rates per 1M tokens)
  claude-code:
    input_per_1m: 3.00
    output_per_1m: 15.00
    cache_read_per_1m: 0.30
  codex:
    input_per_1m: 2.00
    output_per_1m: 8.00
  gemini-cli:
    input_per_1m: 1.25
    output_per_1m: 10.00
  local-llama:
    input_per_1m: 0.00
    output_per_1m: 0.00
```

**YAML — ~/.nexode/session.yaml v2 (Projects and Slots)**

```yaml
projects:
  - id: "saas-app"
    repo: "~/projects/my-saas"
    display_name: "SaaS Platform"
    color: "#20808D"
    tags: ["internal", "priority"]
    budget: { max_usd: 25.00, warn_usd: 18.00 }
    slots:
      - id: "auth-refactor"
        task: "Refactor JWT middleware to support token rotation"
        mode: "full_auto"
        branch: "agent/auth-refactor"
        context:
          include: ["src/auth/**", "tests/auth/**"]
          exclude: ["node_modules/**"]
      - id: "dashboard-ui"
        task: "Build analytics dashboard with React charts"
        model: "codex"
        mode: "plan"
  - include: "./projects/nexode-cli.yaml"     # External file include
  - id: "client-acme"
    repo: "~/projects/acme-api"
    display_name: "Acme Client API"
    color: "#6E522B"
    tags: ["client", "billable"]
    budget: { max_usd: 10.00 }
    slots:
      - id: "api-migration"
        task: "Migrate REST endpoints from v2 to v3 schema"
        mode: "plan"
  - id: "docs"
    repo: "~/projects/nexode-docs"
    display_name: "Nexode Docs"
    color: "#944454"
    slots:
      - id: "api-reference"
        task: "Generate API reference from proto files"
        mode: "full_auto"
  - id: "mcp-research"
    display_name: "MCP Protocol Research"
    color: "#7A39BB"
    tags: ["research"]
    slots:
      - id: "mcp-survey"
        task: "Survey MCP server ecosystem"
        mode: "full_auto"
```

<a id="sec-04-03-schema-field-reference"></a>
### 4.3 Schema Field Reference

#### Session-Level Fields

| Field | Type | Req | Default | Description |
|---|---|---|---|---|
| version | string | Yes | — | Schema version. '2.0' for multi-project. |
| session.name | string | Yes | — | Human label for the session. |
| session.budget.max_usd | float | No | unlimited | Hard kill ceiling for session. |
| session.budget.warn_usd | float | No | 80% of max | Soft alert threshold. |
| session.defaults.model | string | No | claude-code | Default model for all slots. |
| session.defaults.mode | string | No | plan | Default autonomy tier. |
| session.defaults.timeout_minutes | int | No | 120 | Idle timeout per agent. |

#### Project-Level Fields

| Field | Type | Req | Default | Description |
|---|---|---|---|---|
| id | string | Yes | — | Unique kebab-case identifier. |
| repo | string | No | — | Path to git repo. Omit for non-git tasks. |
| display_name | string | No | id | Human name for TUI/Grid headers. |
| color | string | No | auto | Hex color for project group. Auto-assigned if omitted. |
| tags[] | string[] | No | [] | Filterable labels (e.g., 'client', 'oss'). |
| budget.max_usd | float | No | unlimited | Per-project hard ceiling. |
| include | string | No | — | Path to external YAML file for this project. |

#### Slot-Level Fields

| Field | Type | Req | Default | Description |
|---|---|---|---|---|
| id | string | Yes | — | Unique within project. Kebab-case. |
| task | string | Yes | — | Natural language task description. |
| model | string | No | inherited | Overrides session default model. |
| mode | string | No | inherited | manual \| plan \| full_auto |
| branch | string | No | agent/{id} | Git worktree branch name. |
| context.include[] | string[] | No | all | Glob patterns for context focus. |
| context.exclude[] | string[] | No | none | Glob patterns to exclude. |

<a id="sec-04-04-per-project-nexode-yaml"></a>
### 4.4 Per-Project .nexode.yaml (Repo-Local Override)

A developer can place a `.nexode.yaml` in any repo root. When the daemon discovers a project pointing to
that repo, it merges the repo-local config. Slots defined locally are added; matching slot IDs take
precedence.

**YAML — ~/projects/my-saas/.nexode.yaml**

```yaml
# Repo-local overrides. Merged INTO the session's project entry.
defaults:
  model: "claude-code"
  mode: "plan"
  context:
    exclude: ["node_modules/**", "dist/**", ".next/**"]
slots:
  # Slots here are ADDED to session.yaml slots.
  # Matching slot IDs: repo-local takes precedence.
  - id: "lint-fix"
    task: "Fix all ESLint errors in src/"
    mode: "full_auto"
    branch: "agent/lint-fix"
```

<a id="sec-04-05-backward-compatibility-v1-schema"></a>
### 4.5 Backward Compatibility: v1 Schema

If the daemon encounters a YAML file without a `projects` key (or with `version: "1.0"`), it wraps the entire
file in a single implicit project named 'default'. Existing v1 files work without modification.

#### Minimal Quick-Start (8 lines)

**YAML — Minimal session.yaml**

```yaml
version: "2.0"
session:
  name: "Quick Fix"
projects:
  - id: "my-app"
    repo: "."
    slots:
      - id: "fix-bug"
        task: "Fix the null pointer in src/main.rs line 42"
```

Model defaults to `claude-code`, mode to `plan`, branch auto-generates as `agent/fix-bug`, and no budget
ceiling is enforced.

---

<a id="sec-05-ui-ux-surfaces-command-center"></a>
## 5. UI/UX Surfaces (The Command Center)

The user interface provides multi-monitor spatial orchestration using standard VS Code Webview panels
(Phase 3) and a rich terminal UI (Phase 2). Both rendering shells consume the same gRPC event stream
from the daemon.

> **v2 Changes (E-006, as modified):** UI surfaces now organize agents by project group. Three view
> modes: Project Groups (default), Flat View, Focus View. Monitor assignment is deferred to Phase 4+.
> Idle agent reallocation is manual in Phase 2, auto-suggested in Phase 3+.

<a id="sec-05-synapse-telemetry-grid"></a>
### Synapse Telemetry Grid

- **Project Groups (Default View):** Agents are grouped by project with color-coded headers. Each group shows the project name, agent count, and per-project cost. Groups can be collapsed.
- **Flat View:** All agents in a single grid, sorted by state (executing first, idle last). Useful when monitoring overall swarm health.
- **Focus View:** Expand a single project to fill the grid. Other projects collapse to a compact sidebar list.
- **Per-Cell Display:** Streaming terminal/diff output, token velocity, elapsed time, cost, and one-click actions (Pause, Resume, Kill, Reassign).
- **Sidebar Mode:** A compressed vertical list in the VS Code sidebar showing agent avatars, active task names, and status indicators (teal = executing, orange = blocked).
- **Maximized Mode:** Popped out via WebviewPanel, 1x1 to 3x3 grid. Can be dragged to a second monitor via standard VS Code panel dragging.

<a id="sec-05-macro-kanban-board-task-queue"></a>
### Macro Kanban Board and Task Queue

- Full-screen WebviewPanel with drag-and-drop DAG dependency map.
- Columns: Pending, Working, Done, Merged, Paused, Archived.
- Cards expand to show worktree paths, branch names, conflict risk scores, and token costs.
- Phase 2: Project filtering via dropdown selector. Phase 3+: Cross-project swim lanes.

<a id="sec-05-universal-command-chat"></a>
### Universal Command Chat

Registered as a Chat Participant in VS Code's native ChatBar. Users can route instructions to specific slots
or agents: `@nexode /pause agent-3`, `/assign Task-7 to @claude-2`.

<a id="sec-05-merge-choreography"></a>
### Merge Choreography

An AuxiliaryBar TreeView that visualizes the Git worktree merge queue per project. Shows structural
conflict risk scores (Phase 4: AST-based). Facilitates human-in-the-loop merge approvals.

<a id="sec-05-status-bar-hud"></a>
### Status Bar HUD

Global metrics anchored to the VS Code footer: active agent count, aggregate token velocity (tok/s), total
session cost, per-project cost breakdown (top 3 projects shown, click to expand), and a developer fatigue
meter.

---

<a id="sec-06-orchestration-autonomy-orchestratoragent"></a>
## 6. Orchestration and Autonomy (The OrchestratorAgent)

A central control-plane agent (the "Observer") runs as a continuous loop within the Rust Daemon to
prevent swarm chaos. In the multi-project model, the Orchestrator operates per-project: it manages slots
within each project independently and does not attempt cross-project intelligence until Phase 3+.

<a id="sec-06-auto-dispatch"></a>
### Auto-Dispatch

The Orchestrator monitors slot states. When a slot's agent completes its task or crashes, the Orchestrator
evaluates whether to auto-spawn a replacement (if mode is `full_auto`) or surface a prompt to the human
(if mode is `plan`). It checks task dependencies, budget remaining, and slot context before dispatch.

<a id="sec-06-human-in-the-loop-checkpoints"></a>
### Human-in-the-Loop Checkpoints

The Orchestrator uses `UncertaintyFlagTriggered` events to pause agents and prompt the human for a
decision. Triggers include: file modification thresholds exceeded, structural (AST) conflict risk above a
configurable threshold, agent uncertainty self-report (via a structured output marker), or budget
warning level crossed.

<a id="sec-06-observer-agent-three-monitoring-loops"></a>
### Observer Agent: Three Monitoring Loops

The OrchestratorAgent runs three concurrent monitoring loops:

1. **Heartbeat Loop (every 2s):** Checks process liveness for all active agent slots. Triggers crash recovery if a process has gone silent beyond the configured timeout.
2. **Budget Loop (every 30s):** Queries the Token Accountant for per-project costs. If a project hits `warn_usd`, it emits `ProjectBudgetAlert(hard_kill: false)`. If `max_usd` is reached, it emits `ProjectBudgetAlert(hard_kill: true)` and kills all project agents.
3. **Semantic Loop (continuous):** Background LLM calls to detect scope drift. Phase 4+.

<a id="sec-06-autonomy-tiers"></a>
### Autonomy Tiers

| Tier | YAML Value | Behavior |
|---|---|---|
| Manual | manual | Agent waits for explicit `AssignTask` before starting. No auto-respawn. |
| Plan | plan | Agent starts, generates plan, pauses for human approval before executing. Auto-respawn on crash. |
| Full Auto | full_auto | Agent starts, plans, and executes without checkpoints. Orchestrator monitors drift and budget. |

---

<a id="sec-07-phased-development-timeline-overview"></a>
## 7. Phased Development Timeline (Overview)

> **v2 Scope Change (E-007):** This section has been restructured to reflect the multi-project model.
> Agent Pools, the `.swarm/` protocol, and mutation zones are moved to Phase 3+ (Appendix A).
> Phases 0-2 focus exclusively on the daemon, headless orchestrator, and TUI. VS Code integration
> is Phase 3.

| Phase | Duration | Deliverable | Exit Criteria |
|---|---|---|---|
| 0 | 2 weeks | Spike: core runtime proof | Multi-repo worktrees. Session parsed. Budget enforced. Crash recovery. CLI output streaming. |
| 1 | 4-6 weeks | Headless orchestrator | Full gRPC + state machine. 5 agents across 3 projects, headless, 24h uptime. |
| 2 | 4-6 weeks | TUI command center | ratatui TUI. 10 agents, project groups, cost tracking, HITL checkpoints. |
| 3 | 6-8 weeks | VS Code integration | Extension pack. Synapse Grid, Kanban Board, Merge Choreography. 15 agents. |
| 4 | 6-8 weeks | Smart context + memory | LanceDB, AST indexing, Scoped Context Compiler. 20-40% token reduction. |
| 5 | Conditional | Deep Fork | Only if Phase 3 extension host bottleneck is unresolvable. |

---

<a id="sec-08-phase-0-spike-validate"></a>
## 8. Phase 0: Spike and Validate (2 Weeks)

<a id="sec-08-objective"></a>
### Objective

Prove the fundamental technical assumptions before building a production system. Phase 0 is explicitly
not production code. It is a controlled experiment to validate four core hypotheses.

<a id="sec-08-hypotheses"></a>
### Hypotheses to Test

1. **Multi-repo worktrees work:** Can the daemon programmatically create, assign, and manage git worktrees across multiple repositories without corrupting the working tree?
2. **Session config parses correctly:** Does the session.yaml v2 schema (with defaults cascade, include directives, and .nexode.yaml merging) parse into the correct domain objects without ambiguity?
3. **CLI output streams reliably:** Can the daemon capture stdout/stderr from a Claude Code or Codex process in real-time, parse token counts, and stream them via gRPC to a connected client without significant latency?
4. **Crash recovery is fast:** If an agent process is killed, can the daemon detect the exit, respawn a blank agent into the same AgentSlot, and emit a `SlotAgentSwapped` event within 2 seconds?

<a id="sec-08-week-1-foundation"></a>
### Week 1 Deliverables: Foundation

- **Session Config Manager:** Parse `session.yaml v2` with defaults cascade, include directives, and `.nexode.yaml` merging.
- **Git Worktree Orchestrator:** `create_worktree(project, slot_id, branch_name)`, `delete_worktree(worktree_id)`, `list_worktrees(project_id)` using git2 or gix.
- **Agent Process Manager:** `spawn_agent(slot_id, cli_type, worktree_path)` using `tokio::process`. Reads stdout line-by-line. Emits to an mpsc channel.
- **Token Accountant:** SQLite table: `(slot_id, project_id, timestamp, tokens_in, tokens_out, model, cost_usd)`. Three methods: `record()`, `get_project_total()`, `get_session_total()`.

<a id="sec-08-week-2-agent-lifecycle"></a>
### Week 2 Deliverables: Agent Lifecycle

- **Heartbeat Watchdog:** Poll agent process every 2s. Kill and respawn if no output in `timeout_minutes`.
- **gRPC Skeleton:** Implement `SubscribeEvents` and `DispatchCommand` stubs. Not full state machine — just connection + streaming proof.
- **Crash Recovery:** Kill an agent manually. Verify `SlotAgentSwapped` is emitted and new process is running within 2 seconds.
- **Multi-Repo Test Harness:** Session with 3 projects, 2 slots each. Spawn mock agents (shell scripts that echo fake token output). Run for 10 minutes. Verify: no worktree conflicts, correct cost accumulation per project, crash recovery fires on slot 3.

<a id="sec-08-kill-criteria"></a>
### Kill Criteria

Stop if:

- The worktree isolation consistently causes file system conflicts with the parent repo.
- The session.yaml parser cannot resolve ambiguous include directives reliably.
- The merge step consistently produces broken code (indicates worktree strategy is flawed).
- gRPC streaming latency for 15 simultaneous agent outputs exceeds 500ms end-to-end.

<a id="sec-08-exit-criteria"></a>
### Exit Criteria

- Session.yaml v2 parses into domain objects with all cascades applied correctly.
- Three repos, two worktrees each: create, use, delete without parent repo corruption.
- Mock agent stdout streams to a gRPC subscriber with < 200ms latency.
- Crash recovery: process kill → `SlotAgentSwapped` event in < 2 seconds.
- Automated worktree merge succeeds without manual conflict resolution for at least N test tasks.

---

<a id="sec-09-phase-1-headless-orchestrator"></a>
## 9. Phase 1: Headless Orchestrator (4-6 Weeks)

<a id="sec-09-objective"></a>
### Objective

Build the production daemon: full gRPC state machine, complete domain model, token accounting, and
crash-recovery loop. No UI. The deliverable is a headless Rust binary that runs 5 agents across 3
projects for 24 hours without human intervention.

<a id="sec-09-weeks-1-2-core-daemon"></a>
### Weeks 1-2: Core Daemon

- **Full domain objects:** Implement NexodeSession, Project, AgentSlot, Agent, TaskNode, Worktree as Rust structs matching the proto definitions.
- **Full gRPC service:** Implement all three RPC methods from `hypervisor.proto v2`: `SubscribeEvents`, `DispatchCommand`, `GetFullState`.
- **State mutation:** All state changes go through the Core Engine mpsc channel. No direct struct mutation from outside the engine loop.

<a id="sec-09-weeks-3-4-grpc-state"></a>
### Weeks 3-4: gRPC State Machine

- **FullStateSnapshot:** Implement `GetFullState` returning the complete project/slot/agent/task hierarchy. `FullStateSnapshot` includes `projects[]` (each containing `slots[]`), `task_dag[]`, and session cost totals.
- **OperatorCommand routing:** `PauseAgent`, `ResumeAgent`, `KillAgent`, `MoveTask`, `AssignTask`, `SetAgentMode`, `ChatDispatch`, `KillProject` — all handled.
- **Event sourcing:** Every state mutation emits a `HypervisorEvent`. A connected gRPC client receives a stream of events that, applied in order, recreates full state.

<a id="sec-09-weeks-5-6-orchestrator-loop"></a>
### Weeks 5-6: OrchestratorAgent Loop

- **Heartbeat Loop:** Every 2s. Detect crashes. Auto-spawn replacement into same slot (if `full_auto`). Emit `SlotAgentSwapped`.
- **Budget Loop:** Every 30s. Check project costs. Emit `ProjectBudgetAlert`. Kill project on `max_usd`.
- **HITL Checkpoint:** On `UncertaintyFlagTriggered`, pause agent, wait for operator `ResumeAgent` command.
- **24-hour soak test:** 5 agents, 3 projects, 24 hours. Target: < 3 human interventions, all budget alerts fire correctly, no daemon memory leaks.

<a id="sec-09-exit-criteria"></a>
### Exit Criteria

- Full gRPC service: all 3 RPC methods implemented and tested with a gRPC client (grpcurl or a Rust integration test).
- 5 agents across 3 projects: 24-hour headless run without daemon crash.
- Budget enforcement: per-project `warn_usd` and `max_usd` triggers verified.
- Crash recovery: < 2s measured, verified in integration test.
- All `OperatorCommand` types handled and smoke-tested.

---

<a id="sec-10-phase-2-tui-command-center"></a>
## 10. Phase 2: TUI Command Center (4-6 Weeks)

<a id="sec-10-objective"></a>
### Objective

Build the first human-facing interface: a rich terminal UI using ratatui. The TUI connects to the daemon
via gRPC, renders project groups, shows agent state and cost, and handles HITL checkpoints.

<a id="sec-10-week-1-tui-skeleton"></a>
### Week 1: TUI Skeleton

- **ratatui layout:** Three-pane layout: Project List (left), Agent Grid (center), Event Log (right).
- **gRPC subscriber:** Connect to daemon, receive `HypervisorEvent` stream, apply mutations to local state mirror.
- **Static rendering:** Display project names, slot IDs, agent states (using colored symbols: ● executing, ○ idle, ✕ blocked).

<a id="sec-10-weeks-2-3-live-data"></a>
### Weeks 2-3: Live Data

- **Project group headers:** Color-coded by project `color` field. Show agent count and per-project cost.
- **Live token velocity:** Spark-line charts (using ratatui Sparkline widget) showing tokens/sec per agent.
- **Keyboard controls:** `p` = pause focused agent, `r` = resume, `k` = kill, `?` = help overlay.
- **Event log:** Right pane shows last 50 `HypervisorEvent` entries with timestamps and event types.

<a id="sec-10-weeks-4-6-hitl-budget"></a>
### Weeks 4-6: HITL and Budget

- **HITL popup:** When `UncertaintyFlagTriggered` fires, show a modal overlay with agent reason, task description, and Resume/Kill options.
- **Budget warnings:** Flash project header orange at `warn_usd`. Red + confirm prompt at `max_usd`.
- **Project filter:** `tab` cycles through projects. `f` = focus mode (single project full-screen).
- **10-agent demo:** 10 agents, 5 projects, 1-hour live demo with real Claude Code or Codex agents.

<a id="sec-10-exit-criteria"></a>
### Exit Criteria

- TUI connects to running daemon, displays live agent states, costs, and token velocity.
- HITL modal works: agent pauses on flag, human responds, agent resumes.
- Budget warnings fire at correct thresholds.
- 10-agent demo: 1 hour, no TUI freeze, no missed events.

---

<a id="sec-11-phase-3-vs-code-integration"></a>
## 11. Phase 3: VS Code Integration (6-8 Weeks)

<a id="sec-11-objective"></a>
### Objective

Build the VS Code Extension Pack: Synapse Telemetry Grid, Macro Kanban Board, Merge Choreography
TreeView, Status Bar HUD, and Universal Command Chat. This is the Phase 3 user-facing deliverable and
the first release candidate for external users.

<a id="sec-11-week-1-extension-scaffold"></a>
### Week 1: Extension Scaffold

- **Extension pack:** `nexode-vscode` TypeScript project. Registers all commands, views, and chat participants.
- **gRPC client (TypeScript):** Connect to Unix socket. Receive `HypervisorEvent` stream. Maintain a local state mirror (EventBus pattern).
- **Status Bar HUD:** Always-visible footer item: agent count, total cost, aggregate tokens/sec.

<a id="sec-11-weeks-2-4-multi-monitor-react-webviews"></a>
### Weeks 2-4: Multi-Monitor React Webviews

- **Synapse Grid WebviewPanel:** React app rendered in VS Code WebviewPanel. Subscribes to EventBus. Renders project groups with per-cell agent cards. Supports Flat, Group, and Focus view modes.
- **Macro Kanban WebviewPanel:** React app. Full-screen DAG Kanban with drag-and-drop. Cards show worktree branch, conflict risk, and cost.
- **Merge Choreography TreeView:** VS Code native TreeView (AuxiliaryBar). Shows worktrees in REVIEW state, conflict risk score, approve/reject actions.

<a id="sec-11-weeks-5-8-chat-polish"></a>
### Weeks 5-8: Chat, Polish, and Release

- **Universal Command Chat:** Register `@nexode` as a VS Code Chat Participant. Handle structured commands: `/pause`, `/resume`, `/assign`, `/slot`.
- **Extension polish:** Settings page (session.yaml path, socket path, theme). Onboarding walkthrough. README.
- **Release candidate:** Publish to VS Code Marketplace. 15-agent demo across 5 projects. Documentation site.

<a id="sec-11-exit-criteria"></a>
### Exit Criteria

- Extension connects to daemon, all 3 Webviews render live data.
- 15 agents across 5 projects: 2-hour run, no VS Code freeze, no missed events.
- HITL modal works in VS Code: agent pauses, human responds via Chat or Kanban.
- Merge Choreography TreeView shows correct REVIEW state and conflict risk.
- Published to VS Code Marketplace with install instructions.

---

<a id="sec-12-phase-4-smart-context-semantic-memory"></a>
## 12. Phase 4: Smart Context and Semantic Memory (6-8 Weeks)

<a id="sec-12-objective"></a>
### Objective

Upgrade the daemon's context injection from static session.yaml glob patterns to dynamic, AST-aware
vector search. Target: 20-40% reduction in per-task token consumption without degrading output quality.

<a id="sec-12-step-1-real-time-ast-indexing"></a>
### Step 1: Real-Time AST Indexing

- **Tree-sitter integration:** On project load, parse all source files and extract structural signatures (function signatures, class names, import maps). Strip function bodies; retain structure.
- **Incremental updates:** File watcher (notify crate) triggers re-parse on file change. Delta updates to the Codebase Graph.

<a id="sec-12-step-2-lancedb-vector-memory-bus"></a>
### Step 2: LanceDB Vector Memory Bus

- **Embed AST signatures:** Use a local embedding model (e.g., `nomic-embed-text`) to embed structural signatures into LanceDB.
- **Decision Log:** Embed agent reasoning traces and architectural decisions into a separate LanceDB table.
- **Query interface:** `search_relevant_context(task: &str, project_id: &str) -> Vec<ContextChunk>`.

<a id="sec-12-step-3-scoped-context-compiler"></a>
### Step 3: Scoped Context Compiler

- **Three-input compilation:** Role persona + AST context chunks + Decision Log history → compiled context string.
- **Token budget enforcement:** Context compiler respects a `max_context_tokens` limit per slot. Truncates or summarizes if over budget.
- **Injection point:** Context is injected into the agent's initial prompt at slot assignment, not at every message.

<a id="sec-12-step-4-predictive-conflict-routing"></a>
### Step 4: Predictive Conflict Routing

- **AST mutation comparison:** Before merging two worktrees, compare their AST diffs for structural overlap. Compute a conflict risk score.
- **High-risk routing:** If structural conflict risk exceeds threshold, route to REVIEW before merge attempt.
- **UI indicator upgrade:** Phase 3 Merge Choreography TreeView shows AST-based conflict risk score (replaces Phase 3 placeholder).

<a id="sec-12-phase-4-success-kill-criteria"></a>
### Phase 4 Success and Kill Criteria

- **Success:** 20%+ token reduction on representative task set. Decision Log queries return relevant past decisions for >80% of test tasks. No latency regression > 100ms on slot dispatch.
- **Kill:** If LanceDB causes runtime stutter on the DGX Spark, offload vector DB to a separate process and connect via IPC.

---

<a id="sec-13-phase-5-deep-fork-conditional"></a>
## 13. Phase 5: Deep Fork (Conditional)

<a id="sec-13-trigger-condition"></a>
### Trigger Condition

Phase 5 is **conditional.** It is triggered only if Phase 3's extension-based approach fails due to
fundamental VS Code Extension Host limitations:

- Webview IPC overhead exceeds 200ms for 15-agent real-time streaming.
- Extension Host memory ceiling blocks the EventBus from scaling to 15 projects.
- VS Code API limitations prevent required UI customizations.

If Phase 3 extension approach succeeds, Phase 5 does not execute.

<a id="sec-13-architectural-shift-bypassing-extension-host"></a>
### Architectural Shift: Bypassing the Extension Host

Instead of communicating between the Rust Daemon and TypeScript through the Webview IPC bridge
(which routes through the Extension Host), Phase 5 moves critical communication components into VS
Code core itself:

- **HypervisorClient** and **EventBus** move from the Extension Host into VS Code core (workbench layer).
- **gRPC connection** is established at VS Code startup, not at extension activation.
- **Events** bypass the Extension Host entirely, flowing directly from the Rust socket to VS Code's internal event system.

<a id="sec-13-ui-overhaul-native-dom"></a>
### UI Overhaul: Native DOM

Replace Phase 3's React Webviews with native VS Code DOM primitives:

- **SynapseGridPart:** A custom VS Code Part (like the Sidebar or Panel) rendered with direct DOM manipulation. Target: 60 fps for 15 simultaneous streaming agents.
- **No React overhead:** Native DOM rendering eliminates the React reconciler and Webview IPC overhead.

<a id="sec-13-exploiting-sessions-layer"></a>
### Exploiting VS Code's Sessions Layer

Part of the VS Code fork strategy involves exploiting the Sessions layer for persistent agent contexts:

- **AgenticParts:** Custom Parts registered via the Sessions API that persist across workspace reloads.
- **`SynapseGridPart`:** Registers as a persistent Part showing the Synapse Telemetry Grid.
- **`MacroKanbanPart`:** Registers as a persistent Part showing the Kanban Board.
- **`ChatBar` takeover:** Extends the VS Code Chat API to register `@nexode` as a first-class participant with sidebar integration.

<a id="sec-13-native-worktree-scm-isolation"></a>
### Native Worktree SCM Isolation

VS Code's Source Control Managers (SCMs) are extended natively to distinguish between:

- **Human Main:** The operator's primary Git working directory.
- **Agent Sandboxes:** The isolated worktrees managed by Nexode agents.

The Explorer panel and Diff Editor are modified to show the agent sandboxes as distinct, visually
separated contexts within the same VS Code window.

<a id="sec-13-maintenance-strategy"></a>
### Maintenance Strategy

- **Minimal core mutation:** Only the components necessary for Nexode's HypervisorClient, EventBus, and AgenticParts are modified in VS Code core.
- **Patch tracking:** All VS Code core patches are tracked in a `patches/` directory in the Nexode repo.
- **Monthly rebases:** The fork is rebased onto VS Code `main` monthly. Patches are reviewed and reapplied.

---

<a id="app-a-agent-pools-swarm-protocol"></a>
## Appendix A: Agent Pools and .swarm/ Protocol (Phase 3+)

> This appendix captures the Agent Pool architecture, which was deferred from the core spec during
> the v2 multi-project reframing. It is preserved here for Phase 3+ implementation planning.

### Overview

For tasks that benefit from parallel sub-agent work within a single codebase (e.g., writing 50 unit tests
in parallel), Nexode supports Agent Pools. A pool is a group of 1-4 agents assigned to a single
Macro Kanban card. Pool agents communicate through a file-based message bus (the `.swarm/` directory)
within the worktree.

<a id="app-a-pool-structure-role-based-micro-swarms"></a>
### Pool Structure: Role-Based Micro-Swarms

| Role | Count | Responsibility |
|---|---|---|
| Builder | 1 | Implements the feature. Writes to assigned directories. |
| Tester | 1-2 | Writes and runs tests. Blocked until Builder commits. |
| Reviewer / Documenter | 0-1 | Reviews diffs, writes docstrings, updates API docs. |

<a id="app-a-swarm-file-based-message-bus"></a>
### .swarm/ File-Based Message Bus

```
<worktree-root>/.swarm/
├── status.json          # Current pool status: agent states, task phase
├── inbox/
│   ├── builder.md       # Instructions for the Builder agent
│   ├── tester.md        # Instructions for the Tester agent(s)
│   └── reviewer.md      # Instructions for the Reviewer
└── outbox/
    └── builder/
        └── progress.md  # Builder's progress updates for Tester to consume
```

<a id="app-a-coordination-constraints"></a>
### Coordination Constraints

- **Mutation Zones:** Each pool role is assigned a set of directories. The Builder owns `src/`, the Tester owns `tests/`, the Reviewer owns `docs/`. Agents must not write outside their mutation zone without explicit Orchestrator permission.
- **Sequencing:** Tester agents are blocked from running until the Builder has committed at least one working change. The Orchestrator enforces this via `TaskStatus` transitions.
- **Pool-wide stop condition:** If the Builder and Tester enter a loop (build fails → test re-runs → build fails), the Orchestrator flags the pool as `BLOCKED` and surfaces a HITL checkpoint.

---

<a id="app-b-licensing-distribution-strategy"></a>
## Appendix B: Licensing and Distribution Strategy

| Component | License | Rationale |
|---|---|---|
| Rust Hypervisor Daemon | MIT or Apache 2.0 (dual) | Core infrastructure. Community contribution welcome. Builds moat through adoption. |
| TUI (Phase 2) | MIT or Apache 2.0 | Open-source alternative UI. Low commercial risk. |
| VS Code Extension Pack (Phase 3) | Freemium (proprietary) | Free tier: 3 agents, 1 project. Pro tier: unlimited agents and projects. Enterprise: SSO, audit logs, team sync. |
| Hosted Session Dashboard (Future) | SaaS | Web-based session dashboard. Monthly subscription. |

---

<a id="app-c-karpathy-alignment-matrix"></a>
## Appendix C: Karpathy Alignment Matrix

| Karpathy Observation | Nexode Response | Spec Reference |
|---|---|---|
| Need for agent command center | Nexode is the command center | sec-01 |
| 10-15 agents across 10-15 projects | Multi-project primitive | sec-03-01 |
| Operator monitors and unblocks | HITL checkpoints | sec-06 |
| Agents get lost without context | Scoped Context Compiler | sec-03-02 |
| Need for swarm orchestration | OrchestratorAgent loop | sec-06 |
| File conflicts between agents | Git worktree isolation | sec-03-02 |
| Cost is a real concern | Token Accountant | sec-02 |
| Error recovery matters | Crash recovery + respawn | sec-03-02 |

---

<a id="app-d-competitive-differentiation"></a>
## Appendix D: Competitive Differentiation

| Dimension | Nexode | tmux/iTerm | VS Code Native | Cursor | OpenHands |
|---|---|---|---|---|---|
| Multi-project sessions | ✅ Native | ❌ Manual | ❌ | ❌ | ❌ |
| Agent lifecycle mgmt | ✅ Daemon | ❌ | ❌ | ⚡ | ✅ |
| Cost tracking | ✅ Per-project | ❌ | ❌ | ❌ | ⚡ |
| Worktree isolation | ✅ Auto | ❌ | ⚡ | ❌ | ❌ |
| Headless daemon | ✅ | ❌ | ❌ | ❌ | ✅ |
| TUI | ✅ Phase 2 | ✅ (manual) | ❌ | ❌ | ❌ |
| VS Code integration | ✅ Phase 3 | ❌ | ✅ built-in | ✅ | ❌ |
| Open source core | ✅ MIT/Apache | ✅ | ✅ | ❌ | ✅ |

---

<a id="app-e-errata-incorporation-log"></a>
## Appendix E: Errata Incorporation Log

| Errata ID | Topic | Disposition | Incorporated Into |
|---|---|---|---|
| E-001 | Karpathy Attribution | ACCEPT | sec-01 |
| E-002 | Core Reframing (Multi-Project) | ACCEPT | sec-01, sec-03-01 |
| E-003 | Domain Model (Project, AgentSlot) | ACCEPT | sec-03-01, sec-03-02, sec-03-04 |
| E-004 | session.yaml v2 | MODIFY | sec-04 (schema redesigned as specified) |
| E-005 | Token Accountant | ACCEPT | sec-02, sec-08-week-1-foundation |
| E-006 | UI/UX Multi-Project | MODIFY | sec-05 (monitor assignment deferred to Phase 4+) |
| E-007 | Phased Timeline | ACCEPT | sec-07 (pools deferred to Phase 3+, Appendix A) |
| E-008 | Competitive Analysis | ACCEPT | Appendix D |
| E-009 | Open Source Strategy | ACCEPT | Appendix B |
| E-010 | Karpathy Alignment Matrix | MODIFY | Appendix C. Added error recovery row. Placed as appendix (rationale, not spec). |
| E-011 | Competitive Differentiation | ACCEPT | Appendix D. Six-axis comparison table with positioning statement. |

---

End of Specification. The Nexode multi-project model is fully specified at the domain, configuration,
protocol, UI, orchestration, and implementation plan levels. The path from session.yaml v2 to a working
15-agent headless orchestrator is clear, scoped, and de-risked.
