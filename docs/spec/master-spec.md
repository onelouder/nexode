

SPEC v2
Nexode Agent IDE
Master Specification v2
Architecture and Implementation Blueprint
Supersedes: Master Specification v1 (March 2026)
Incorporates: Errata 001 (E-001 through E-011), Triage Dispositions
Author: jwells@gmail.com / Xcognis + Perplexity Computer
## Date: March 13, 2026
This document is the authoritative engineering specification for the Nexode Agent IDE. It
incorporates all accepted errata items (7 ACCEPT, 4 MODIFY) from the multi-project
orchestration review, the revised session.yaml v2 schema, and the updated Phase 0-1
implementation plan. Where the original specification described single-codebase swarm
orchestration, this revision reframes Nexode as a multi-project agent command center: 10-15
agents working across 10-15 separate codebases and tasks.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 2
## Contents
- Executive Summary and Core Philosophy
- System Architecture (4-Layer Model)
- Domain Types and State Management
## 3.1 Domain Model Hierarchy
3.2 Fungible Workers and AgentSlot Abstraction
## 3.3 Shared Memory Bus
3.4 hypervisor.proto v2 Contract
- Session Configuration (session.yaml v2)
## 4.1 Design Principles
## 4.2 Complete Schema
## 4.3 Field Reference
4.4 Per-Project .nexode.yaml
## 4.5 Backward Compatibility
- UI/UX Surfaces (The Command Center)
- Orchestration and Autonomy
- Phased Development Timeline (Overview)
- Phase 0: Spike and Validate (2 Weeks)
## 9. Phase 1: Headless Orchestrator (4-6 Weeks)
- Phase 2: TUI Command Center (4-6 Weeks)
- Phase 3: VS Code Integration (6-8 Weeks)
- Phase 4: Smart Context and Semantic Memory (6-8 Weeks)
- Phase 5: Deep Fork (Conditional)
## Appendices
A Agent Pools and .swarm/ Protocol (Phase 3+)
B Licensing and Distribution Strategy
## C Karpathy Alignment Matrix
## D Competitive Differentiation
## E Errata Incorporation Log

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 3
- Executive Summary and Core Philosophy
The Nexode Agent IDE shifts the paradigm of developer tooling from file-centric editing to agent-centric
multi-project orchestration. Designed to fulfill the vision of an "agent command center" (Karpathy, 2025),
Nexode  enables  a  single  developer  to  effectively  manage  10  to  15  parallel  autonomous  coding  agents
(Claude Code, Codex, Gemini CLI, and local models) across 10 to 15 separate codebases and tasks.
Core Reframing (v2): The primary use case is not 15 agents on one codebase. It is 10-15 agents
distributed across 10-15 separate projects: a SaaS backend, a CLI tool, a client API, documentation,
and a research spike, all running simultaneously under a single orchestration session. This is the
pattern described by Karpathy, practiced by power users and consultants, and unserved by any
existing tool.
Nexode  is  optimized  for  horizontal  task  distribution.  Rather  than  forcing  agents  to  compete  for  real-time
semantic locks within the same file — which introduces high research risk and code quality degradation —
agents are isolated in Git worktrees to execute separate, loosely coupled tasks. Nexode serves as "tmux
with  a  brain,"  providing  rich  visual  orchestration,  shared  memory,  and  a  central  control  plane  that  spans
multiple repositories.
## User Personas
PersonaAgentsProjectsKey Need
Solo Developer2-51-3Run agents across front-end, back-end, and tests in parallel.
Single dashboard.
## Power User / Indie
## Hacker
5-103-8Manage multiple personal projects simultaneously. Per-project
cost tracking.
## Consultant /
## Freelancer
10-158-15Run agents on separate client codebases. Budget isolation per
client project.
Team Lead (Future)15+5-10Orchestrate agents across team repos. Shared decision log.
Phase 3+ scope.
## Design Principles
- Extension-first, fork-never (for now): Build as a VS Code extension pack, not a fork. The 2.1M-line
VS Code codebase is a maintenance liability. Phase 5 (Deep Fork) is conditional.
- Daemon-first, UI-second: The Rust hypervisor daemon is the product. The TUI and VS Code
extension are rendering shells. If the daemon works headlessly, every UI is optional.
- Multi-project as the primitive: The session schema, domain model, cost tracking, and UI grouping all
assume multiple projects. Single-project is a degenerate case that works automatically.
- Open source as structural moat: The core daemon and TUI are MIT/Apache 2.0. The VS Code
extension is freemium. See Appendix B.
- Model-agnostic orchestration: Nexode treats agent CLIs as fungible compute. Claude Code, Codex,
Gemini CLI, and local models are interchangeable via a harness abstraction.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 4
- System Architecture (The 4-Layer Model)
To avoid the massive maintenance burden of deeply forking VS Code's ~2.1M lines of TypeScript, Nexode
utilizes  an  "extension-first,  fork-never  (for  now)"  architecture.  The  system  is  organized  into  four  cleanly
decoupled layers.
LayerNameDescription
Layer 0SubstrateThe foundation: OS file system, Git repository internals, LLM provider APIs, and
Model Context Protocol (MCP) servers. This layer exists independent of Nexode.
Layer 1Hypervisor
## Daemon
A standalone Rust binary using the tokio async runtime. Manages: OS process
lifecycle, multi-repo Git worktree orchestration, vector memory (LanceDB), token
accounting (SQLite), the DAG workflow engine, and the OrchestratorAgent
control loop. This is the brain of Nexode.
Layer 2gRPC BridgeHigh-throughput bidirectional IPC (Unix socket or named pipe). Streams agent
output tokens, telemetry, and state mutations directly to the renderer, bypassing
the VS Code Extension Host bottleneck.
Layer 3Rendering ShellA VS Code Extension Pack containing modular UI surfaces. Renders the
multi-monitor Synapse Telemetry Grid, DAG Kanban visualizer, and agent
command chat using standard VS Code API endpoints. The TUI (Phase 2) is an
alternative Layer 3 renderer.
## Key Architectural Property
The  daemon  (Layer  1)  is  completely  decoupled  from  the  UI  (Layer  3).  The  daemon  could  run  on  a  DGX
Spark  in  a  server  rack  while  the  VS  Code  extension  or  TUI  connects  to  it  remotely  via  the  gRPC  socket.
This  decoupling  also  means  that  multiple  rendering  shells  (TUI,  VS  Code,  a  future  web  dashboard)  can
connect simultaneously.
## Layer 1: Rust Daemon Core Architecture
At   the   heart   of   the   daemon   is   a   central   message-passing   architecture   using   tokio::sync::mpsc
(Multi-Producer, Single-Consumer) channels. No shared mutable state. Three primary concurrent loops:
- gRPC Server Loop (Tonic): Listens on the Unix socket for OperatorCommand messages from any
connected UI. Pushes commands to the Core Engine via an mpsc channel.
- Agent Process Runners: Up to 15 individual async tasks, one per active CLI agent. Read from the
agent's stdout/stderr, parse output, and stream telemetry to the Core Engine.
- Core Engine and Orchestrator Loop: The master state machine. Receives UI commands, agent
outputs, and cron triggers. Updates the SQLite database and broadcasts HypervisorEvent payloads to
connected UIs.
## Daemon Subsystem Breakdown

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 5
ComponentResponsibilityImplementation
## Agent Process
## Manager
Spawns, monitors, and terminates
fungible CLI agent processes. Handles
heartbeats and watchdog timeouts.
OS process groups + Unix signals via
tokio::process.
## Git Worktree
## Orchestrator
Creates, assigns, merges, and
garbage-collects isolated git worktrees
across multiple repositories.
Native Git bindings (git2 or gix).
## Scoped Context
## Compiler
Queries the Vector Memory Bus to build
targeted, role-specific prompts for blank
agents before dispatch.
LanceDB (vector search) + Tree-sitter (AST
parsing). Phase 4.
Workflow DAG
## Engine
Manages Kanban columns, parses
session.yaml task slots, and tracks task
completion states.
In-memory graph resolving dependencies.
Token AccountantTracks per-agent, per-slot, per-project,
and per-session token usage and cost
estimates.
SQLite append-only log with project_id and
slot_id columns.
## Session Config
## Manager
Parses session.yaml v2 with defaults
cascade, include directives,
.nexode.yaml merging, and v1 fallback.
serde YAML deserialization with strict
validation.
## Conceptual Rust Skeleton
Rust — main.rs (Conceptual)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 6
## #[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
## // 1. Initialize Substrates
let db_pool = initialize_sqlite_accounting().await;
let vector_db = initialize_lancedb_memory().await;
let session = parse_session_yaml("~/.nexode/session.yaml")?;
// 2. Create the core communication channels
let (cmd_tx, mut cmd_rx) = mpsc::channel::<OperatorCommand>(100);
let (event_tx, event_rx) = broadcast::channel::<HypervisorEvent>(100);
// 3. Spawn the gRPC Server (Layer 2 Bridge)
tokio::spawn(async move {
run_grpc_server(cmd_tx, event_rx).await;
## });
## // 4. The Core Engine Loop
let mut orchestrator = Orchestrator::new(db_pool, vector_db, session);
let mut tick_interval = tokio::time::interval(Duration::from_secs(2));
loop {
tokio::select! {
Some(cmd) = cmd_rx.recv() => {
orchestrator.handle_command(cmd, &event_tx).await;
## }
_ = tick_interval.tick() => {
orchestrator.evaluate_slots_and_dispatch(&event_tx).await;
orchestrator.run_observer_checks(&event_tx).await;
## }
## }
## }
## }

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 7
- Domain Types and State Management
## 3.1 Domain Model Hierarchy
New in v2 (E-003): The domain model introduces two new types: Project and AgentSlot. The hierarchy
is NexodeSession > Project > AgentSlot > Agent. AgentSlot is the stable identity that survives agent
crashes, giving the UI, cost tracker, and Kanban Board a fixed reference point while allowing the
underlying agent process to be recycled freely.
TypeLifecycleOwnsPurpose
NexodeSessionDaemon
start to stop
projects[], budget,
defaults
Top-level container. One per daemon run. Parsed
from session.yaml.
ProjectSession
lifetime
repo, slots[], budget,
color, tags
A single codebase or task collection. Maps to one
Git repository (or none for non-git tasks).
AgentSlotSession
lifetime
task, mode, branch,
context, current_agent
The stable work unit. Represents 'what needs
doing.' Survives agent crashes. Cost accrues to
the slot.
AgentProcess
spawn to
exit
process handle,
stdout/stderr streams
The ephemeral compute worker. A raw CLI
process (Claude, Codex, etc.) bound to a slot.
## Replaceable.
TaskNodeSession
lifetime
title, description,
status, dependencies
A unit of work on the Kanban board. May map 1:1
with a slot or be subdivided.
WorktreeSlot lifetimeabsolute_path,
branch_name,
conflict_risk
Isolated Git worktree directory managed by the
daemon. One per active slot per git-backed
project.
3.2 Fungible Workers and the AgentSlot Abstraction
Nexode  separates  raw  compute  from  context,  allowing  agents  to  act  as  interchangeable  workers  with
unique, task-specific identities.
- Fungible Workers: At the OS layer, agents are generic, spawned CLI processes. If an agent crashes
or hangs, the daemon terminates it and spins up a blank replacement into the same AgentSlot. The slot
ID, cost history, worktree, and task context are preserved.
- Unique Identities (Scoped Context Compiler): An agent gains identity upon slot assignment. The
daemon uses Tree-sitter and vector search (Phase 4) to compile a role-appropriate context slice and
injects it into the blank worker.
- Crash Recovery: When an agent process dies, the daemon detects the exit, spawns a replacement
into the same slot, and emits a SlotAgentSwapped event. The UI sees slot continuity; the cost tracker
continues accruing to the same slot_id.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 8
- O(1) Concurrency: Each agent operates in a completely isolated Git worktree managed automatically
by the daemon. No file-level locking needed.
## 3.3 Shared Memory Bus
While the EventBus (Layer 2) keeps the human in sync, the Vector Memory Bus keeps the fungible agents
in  sync.  Because  we  use  isolated  Git  worktrees,  agents  cannot  see  what  other  agents  are  typing.  The
memory architecture (fully implemented in Phase 4) is divided into three layers:
- Codebase Graph (Code Intent): Embeddings of AST nodes, dependency relationships, and import
maps. The Scoped Context Compiler queries this graph to find structural signatures relevant to a task,
reducing context window bloat by 20-40%.
- Decision Log (Architectural Intent): An append-only log of agent reasoning traces, architectural
decisions, and rejected alternatives. When Agent B starts work after Agent A finishes, it queries
LanceDB for past decisions and aligns accordingly.
- Session State (Ephemeral): Active tasks, agent assignments, and pending approvals. Stored in
SQLite. Cleared when the session ends.
3.4 hypervisor.proto v2 Contract
The  gRPC  contract  between  the  Rust  Daemon  (Layer  1)  and  any  rendering  shell  (Layer  3).  This  is  the
absolute source of truth. v2 adds Project, AgentSlot, ProjectBudgetAlert, and SlotAgentSwapped.
Protocol Buffers — hypervisor.proto v2 (Service and Enums)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 9
syntax = "proto3";
package nexode.hypervisor.v2;
service Hypervisor {
rpc SubscribeEvents(SubscribeRequest) returns (stream HypervisorEvent);
rpc DispatchCommand(OperatorCommand) returns (CommandResponse);
rpc GetFullState(StateRequest) returns (FullStateSnapshot);
## }
enum AgentState {
## AGENT_STATE_UNSPECIFIED = 0;
AGENT_STATE_INIT        = 1;  // Spawning process / provisioning worktree
AGENT_STATE_IDLE        = 2;  // Waiting in the fungible worker pool
AGENT_STATE_PLANNING    = 3;  // Compiling context / generating plan
AGENT_STATE_EXECUTING   = 4;  // Writing code / running tools
AGENT_STATE_REVIEW      = 5;  // Awaiting Orchestrator or Human approval
AGENT_STATE_BLOCKED     = 6;  // Uncertainty flag (needs help)
AGENT_STATE_TERMINATED  = 7;  // Process killed / worktree GC'd
## }
enum TaskStatus {
## TASK_STATUS_UNSPECIFIED = 0;  TASK_STATUS_PENDING = 1;
## TASK_STATUS_WORKING = 2;  TASK_STATUS_REVIEW = 3;
## TASK_STATUS_DONE = 4;  TASK_STATUS_PAUSED = 5;  TASK_STATUS_ARCHIVED = 6;
## }
enum AgentMode {
## AGENT_MODE_UNSPECIFIED = 0;  AGENT_MODE_NORMAL = 1;
## AGENT_MODE_PLAN = 2;  AGENT_MODE_FULL_AUTO = 3;
## }
Protocol Buffers — hypervisor.proto v2 (Entity Models)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 10
message Project {
string id = 1;  string display_name = 2;  string repo_path = 3;
string color = 4;  repeated string tags = 5;
double budget_max_usd = 6;  double budget_warn_usd = 7;
double current_cost_usd = 8;  repeated AgentSlot slots = 9;
## }
message AgentSlot {
string id = 1;  string project_id = 2;  string task = 3;
AgentMode mode = 4;  string branch = 5;
string current_agent_id = 6;  string worktree_id = 7;
uint64 total_tokens = 8;  double total_cost_usd = 9;
## }
message Agent {
string id = 1;  string display_name = 2;  string current_role = 3;
AgentState state = 4;  AgentMode mode = 5;
string slot_id = 6;  string worktree_id = 7;
uint64 tokens_consumed = 8;  double estimated_cost = 9;
double tokens_per_sec = 10;
## }
message TaskNode {
string id = 1;  string title = 2;  string description = 3;
TaskStatus status = 4;  string assigned_agent_id = 5;
string project_id = 6;  // NEW in v2
repeated string dependency_ids = 7;
## }
message Worktree {
string id = 1;  string absolute_path = 2;
string branch_name = 3;  double conflict_risk = 4;
## }
message SubscribeRequest { string client_version = 1; }
message StateRequest {}
message FullStateSnapshot {
repeated Project projects = 1;  // v2: replaces flat agent list
repeated TaskNode task_dag = 2;
double total_session_cost = 3;  double session_budget_max_usd = 4;
## }
Protocol Buffers — hypervisor.proto v2 (Events and Commands)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 11
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
## }
## }
message AgentStateChanged  { string agent_id = 1; AgentState new_state = 2; }
message AgentTelemetryUpdated { string agent_id = 1; uint64 incr_tokens = 2; double tps = 3; }
message TaskStatusChanged  { string task_id = 1; TaskStatus new_status = 2; string agent_id = 3; }
message UncertaintyFlagTriggered { string agent_id = 1; string task_id = 2; string reason = 3; }
message WorktreeStatusChanged { string worktree_id = 1; double new_risk = 2; }
message ProjectBudgetAlert {
string project_id = 1; double current_usd = 2; double limit_usd = 3;
bool hard_kill = 4;  // true = at max. false = at warn.
## }
message SlotAgentSwapped {
string slot_id = 1; string old_agent_id = 2; string new_agent_id = 3;
string reason = 4;  // "crash_recovery", "manual_reassign", "initial"
## }
message OperatorCommand {
string command_id = 1;
oneof action {
PauseAgent pause_agent = 2;  ResumeAgent resume_agent = 3;
KillAgent kill_agent = 4;  MoveTask move_task = 5;
AssignTask assign_task = 6;  SetAgentMode set_agent_mode = 7;
ChatDispatch chat_dispatch = 8;  KillProject kill_project = 9;  // NEW v2
## }
## }
message PauseAgent   { string agent_id = 1; }
message ResumeAgent  { string agent_id = 1; }
message KillAgent    { string agent_id = 1; }
message MoveTask     { string task_id = 1; TaskStatus target = 2; }
message AssignTask   { string task_id = 1; string agent_id = 2; }
message SetAgentMode { string agent_id = 1; AgentMode new_mode = 2; }
message ChatDispatch { string raw_nl = 1; }
message KillProject  { string project_id = 1; }  // Stops all agents
message CommandResponse { bool success = 1; string error_message = 2; }
Design Highlights: The barrier_id in HypervisorEvent ensures the TypeScript EventBus updates all React
Webviews simultaneously before rendering, preventing torn UI state. The ChatDispatch command wraps
natural language from the VS Code ChatBar (e.g., @agent-3 pause) and routes it to the OrchestratorAgent
for parsing.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 12
- Session Configuration (session.yaml v2)
New in v2 (E-004, as modified): The session.yaml schema has been redesigned from scratch for the
multi-project model. It adds: a defaults block for DRY configuration, include directives for splitting
large sessions across files, per-project .nexode.yaml overrides, tags for filtering, and a model pricing
table.
## 4.1 Design Principles
- Convention over configuration: A session with one project and one agent should require fewer than
10 lines of YAML. Defaults block eliminates repetition.
- Layered configuration: Global ~/.nexode/session.yaml defines the portfolio. Per-project .nexode.yaml
in repo roots defines slots. The daemon merges both, with per-project overriding global defaults.
Mirrors .gitconfig / .git/config layering.
- Include directives: Large sessions can split project definitions into separate files for readability.
- v1 backward compatibility: A YAML file without a projects[] key is treated as a single implicit project.
The version field disambiguates.
- Human-friendly identifiers: Project and slot IDs are short kebab-case strings chosen by the user, not
UUIDs.
4.2 Complete Schema with Annotations
YAML — ~/.nexode/session.yaml v2 (Session and Model Config)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 13
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
YAML — ~/.nexode/session.yaml v2 (Projects and Slots)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 14
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

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 15
## 4.3 Schema Field Reference
Session-Level Fields
FieldTypeRe
q
DefaultDescription
versionstrin
g
## Ye
s
—Schema version. '2.0' for multi-project.
session.namestrin
g
## Ye
s
—Human label for the session.
session.budget.max_usdfloatNounlimitedHard kill ceiling for session.
session.budget.warn_usdfloatNo80% of
max
Soft alert threshold.
session.defaults.modelstrin
g
## Noclaude-c
ode
Default model for all slots.
session.defaults.modestrin
g
NoplanDefault autonomy tier.
session.defaults.timeout_minut
es
intNo120Idle timeout per agent.
Project-Level Fields
FieldTypeRe
q
DefaultDescription
idstringYe
s
—Unique kebab-case identifier.
repostringNo—Path to git repo. Omit for non-git tasks.
display_namestringNoidHuman name for TUI/Grid headers.
colorstringNoautoHex color for project group. Auto-assigned if omitted.
tags[]string[
## ]
No[]Filterable labels (e.g., 'client', 'oss').
budget.max_usdfloatNounlimite
d
Per-project hard ceiling.
includestringNo—Path to external YAML file for this project.
Slot-Level Fields

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 16
FieldTypeRe
q
DefaultDescription
idstringYe
s
—Unique within project. Kebab-case.
taskstringYe
s
—Natural language task description.
modelstringNoinherite
d
Overrides session default model.
modestringNoinherite
d
manual | plan | full_auto
branchstringNoagent/{i
d}
Git worktree branch name.
context.include[]string[
## ]
NoallGlob patterns for context focus.
context.exclude[]string[
## ]
NononeGlob patterns to exclude.
4.4 Per-Project .nexode.yaml (Repo-Local Override)
A developer can place a .nexode.yaml in any repo root. When the daemon discovers a project pointing to
that  repo,  it  merges  the  repo-local  config.  Slots  defined  locally  are  added;  matching  slot  IDs  take
precedence.
YAML — ~/projects/my-saas/.nexode.yaml
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
4.5 Backward Compatibility: v1 Schema
If the daemon encounters a YAML file without a projects key (or with version: "1.0"), it wraps the entire
file in a single implicit project named 'default'. Existing v1 files work without modification.
Minimal Quick-Start (8 lines)

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 17
YAML — Minimal session.yaml
version: "2.0"
session:
name: "Quick Fix"
projects:
- id: "my-app"
repo: "."
slots:
- id: "fix-bug"
task: "Fix the null pointer in src/main.rs line 42"
Model  defaults  to  claude-code,  mode  to  plan,  branch  auto-generates  as  agent/fix-bug,  and  no  budget
ceiling is enforced.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 18
- UI/UX Surfaces (The Command Center)
The user interface provides multi-monitor spatial orchestration using standard VS Code Webview panels
(Phase 3) and a rich terminal UI (Phase 2). Both rendering shells consume the same gRPC event stream
from the daemon.
v2 Changes (E-006, as modified): UI surfaces now organize agents by project group. Three view
modes: Project Groups (default), Flat View, Focus View. Monitor assignment is deferred to Phase 4+.
Idle agent reallocation is manual in Phase 2, auto-suggested in Phase 3+.
## Synapse Telemetry Grid
- Project Groups (Default View): Agents are grouped by project with color-coded headers. Each group
shows the project name, agent count, and per-project cost. Groups can be collapsed.
- Flat View: All agents in a single grid, sorted by state (executing first, idle last). Useful when
monitoring overall swarm health.
- Focus View: Expand a single project to fill the grid. Other projects collapse to a compact sidebar list.
- Per-Cell Display: Streaming terminal/diff output, token velocity, elapsed time, cost, and one-click
actions (Pause, Resume, Kill, Reassign).
- Sidebar Mode: A compressed vertical list in the VS Code sidebar showing agent avatars, active task
names, and status indicators (teal = executing, orange = blocked).
- Maximized Mode: Popped out via WebviewPanel, 1x1 to 3x3 grid. Can be dragged to a second
monitor via standard VS Code panel dragging.
Macro Kanban Board and Task Queue
- Full-screen WebviewPanel with drag-and-drop DAG dependency map.
## • Columns: Pending, Working, Done, Merged, Paused, Archived.
- Cards expand to show worktree paths, branch names, conflict risk scores, and token costs.
- Phase 2: Project filtering via dropdown selector. Phase 3+: Cross-project swim lanes.
## Universal Command Chat
Registered as a Chat Participant in VS Code's native ChatBar. Users can route instructions to specific slots
or agents: @nexode /pause agent-3, /assign Task-7 to @claude-2.
## Merge Choreography
An  AuxiliaryBar  TreeView  that  visualizes  the  Git  worktree  merge  queue  per  project.  Shows  structural
conflict risk scores (Phase 4: AST-based). Facilitates human-in-the-loop merge approvals.
Status Bar HUD

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 19
Global metrics anchored to the VS Code footer: active agent count, aggregate token velocity (tok/s), total
session cost, per-project cost breakdown (top 3 projects shown, click to expand), and a developer fatigue
meter.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 20
- Orchestration and Autonomy (The
OrchestratorAgent)
A  central  control-plane  agent  (the  "Observer")  runs  as  a  continuous  loop  within  the  Rust  Daemon  to
prevent swarm chaos. In the multi-project model, the Orchestrator operates per-project: it manages slots
within each project independently and does not attempt cross-project intelligence until Phase 3+.
Auto-Dispatch
The Orchestrator monitors slot states. When a slot's agent completes its task or crashes, the Orchestrator
evaluates  whether  to  auto-spawn  a  replacement  (crash  recovery)  or  mark  the  slot  as  idle.  In  full_auto
mode, completed slots with remaining sub-tasks are automatically re-dispatched.
## Autonomy Tiers
ModeBehaviorUse Case
manualAgent asks permission before every write and
terminal command.
High-risk tasks, production codebases,
learning/auditing.
planAgent proposes a plan. Daemon pauses for operator
approval via gRPC. Agent then executes the approved
plan autonomously.
Default mode. Balanced safety and
speed.
full_autoAgent runs continuously until task completion, budget
limit, or error. No human gates.
Low-risk tasks, documentation, linting,
research, well-tested codebases.
Uncertainty Routing (Loop Detection)
The Orchestrator monitors execution traces of all active agents. If an agent runs npm test, fails, writes the
exact  same  code,  and  runs  npm  test  again  3  times  in  a  row,  it  halts  that  agent's  process,  emits  an
UncertaintyFlagTriggered  event,  and  pulses  that  agent's  Synapse  Grid  cell  orange,  requesting  human
intervention without halting the rest of the swarm.
## Observer Agent: Three Monitoring Loops
- Static Enforcement Loop (Sandbox Boundaries): OS-level file watchers enforce per-slot context
rules. If a slot's context.exclude pattern covers a directory and the agent writes there, the Observer
blocks the write and injects a mock terminal error.
- Dynamic Loop (Tool and Terminal Monitoring): Monitors stdout/stderr for repeated identical tool
calls and anomalous diff sizes. Triggers 'Stop the Line' after configurable threshold.
- Semantic Loop (Persona and Context Coherence, Phase 4): Periodic background LLM calls evaluate
worker output against the slot's task description. Detects semantic drift (agent working on wrong
thing) and triggers correction or pause.
Per-Project Cost Enforcement

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 21
New in v2 (E-009): Four-level cost accounting: per-agent, per-slot, per-project, per-session. When a
project's cost exceeds budget.max_usd, the daemon sends SIGTERM to all agents in that project and
emits a ProjectBudgetAlert event with hard_kill = true. The warning threshold emits the same event
with hard_kill = false.
SQLite Schema for Cost Tracking
SQL — Token Accounting Schema
CREATE TABLE token_log (
id          INTEGER PRIMARY KEY AUTOINCREMENT,
timestamp   TEXT    NOT NULL DEFAULT (datetime('now')),
session_id  TEXT    NOT NULL,
project_id  TEXT    NOT NULL,
slot_id     TEXT    NOT NULL,
agent_id    TEXT    NOT NULL,
model       TEXT    NOT NULL,
tokens_in   INTEGER NOT NULL DEFAULT 0,
tokens_out  INTEGER NOT NULL DEFAULT 0,
cache_read  INTEGER NOT NULL DEFAULT 0,
cost_usd    REAL    NOT NULL DEFAULT 0.0
## );
CREATE VIEW project_costs AS
SELECT project_id,
SUM(tokens_in)  AS total_in,
SUM(tokens_out) AS total_out,
SUM(cost_usd)   AS total_cost
FROM token_log
GROUP BY project_id;
CREATE VIEW slot_costs AS
SELECT project_id, slot_id,
SUM(cost_usd) AS total_cost
FROM token_log
GROUP BY project_id, slot_id;

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 22
- Phased Development Timeline (Overview)
The  engineering  roadmap  mitigates  risk  by  proving  the  daemon  and  headless  orchestration  before
investing in complex UI. Each phase has explicit success and kill criteria.
## Pha
se
NameDurationKey Deliverable
0Spike and Validate2 weeksMinimal Rust daemon: parse session.yaml v2, spawn agents
across multiple repos, track per-project costs, enforce budget
ceilings. Headless only.
1Headless
## Orchestrator
4-6 weeks
(+2 buffer)
Full daemon: gRPC streaming, WAL crash recovery,
OrchestratorAgent loop, autonomy tiers, uncertainty routing.
10-15 agents across 10+ projects.
2TUI Command
## Center
4-6 weeksRich terminal interface (ratatui): multi-project navigation, agent
grid, vim-style controls, inline diff viewer.
3VS Code Integration6-8 weeksTypeScript extension pack: Synapse Grid, Kanban, ChatBar,
Merge Choreography. gRPC bridge with barrier synchronization.
4Smart Context6-8 weeksTree-sitter AST indexing, LanceDB vector memory, Scoped
Context Compiler, predictive conflict routing. 20-40% token
reduction.
5Deep Fork
(Conditional)
8-12 weeksFork VS Code only if Extension Host IPC bottlenecks at scale.
Native core integration, native DOM rendering. Break-glass plan.
v2 Scope Change (E-007): Agent Pools, .swarm/ protocol, mutation zones, pool persona YAML, and
intra-pool DAG are removed from Phase 0-1 and deferred to Phase 3+. Replaced by: multi-project
session.yaml v2 parser, AgentSlot abstraction, per-project cost tracking, and budget enforcement. Net
savings: ~1 week, with significantly reduced risk.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 23
- Phase 0: Spike and Validate (2 Weeks)
Goal: Prove the daemon can parse a multi-project session.yaml, spawn agents across separate git repos,
track per-project costs, and enforce budget ceilings. Headless only — no TUI, no VS Code.
## Week 1: Foundation
DayDeliverableAcceptance Criteria
1-2session.yaml v2 parserSerde deserializes the full schema. Defaults cascade: session > project
> slot. Include directives resolve. v1 YAML auto-wraps into single
project.
2-3Domain types:
NexodeSession, Project,
AgentSlot
Rust structs with builder pattern. Unit tests for state transitions:
slot_empty > slot_assigned > agent_crashed > slot_reassigned.
3-4SQLite schema v2 + Token
## Accountant
token_log table with project_id, slot_id columns. project_costs view.
Insert mock token events and verify aggregation queries.
4-5Model pricing loaderParse models: block from session.yaml. Feed pricing to Token
Accountant. Test: estimate_cost(model, tokens_in, tokens_out) returns
correct USD.
## Week 2: Agent Lifecycle
DayDeliverableAcceptance Criteria
6-7Agent spawner: multi-repo
worktree creation
Given a session with 3 projects across 3 repos, daemon creates git
worktrees for each slot. Verify: each worktree is on the correct branch,
in the correct repo.
7-8Agent process lifecycle
(spawn, monitor, kill)
Spawn a real claude-code / codex process in a worktree. Capture
stdout/stderr. Detect process exit. Log token events to SQLite.
8-9Budget enforcementPer-project: when cost exceeds budget.max_usd, daemon SIGTERMs all
project agents. Per-session: global hard kill. Test with mock pricing that
triggers limits quickly.
9-10AgentSlot crash recoveryKill an agent (SIGKILL). Daemon detects exit, spawns replacement into
same slot. Slot ID unchanged. Cost continues accruing to same slot_id.
Integration test: 3 projects, kill agents, verify recovery.
## Phase 0 Exit Criteria
- Demo: spawn 5 agents across 3 repos from a single session.yaml
- Demo: kill an agent, observe slot recovery, verify cost continuity
- Demo: hit a project budget ceiling, observe project-level agent shutdown

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 24
- Demo: all token events in SQLite with correct project_id and slot_id
- All acceptance criteria pass in CI (cargo test)
## Kill Criteria
If  git  worktree  creation  is  flaky  across  multiple  repos,  if  tokio  fails  to  gracefully  kill  runaway  agent
processes across OS environments, or if the merge step consistently produces broken code, we stop. We
do not proceed until the isolation and lifecycle strategy is rock solid.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 25
## 9. Phase 1: Headless Orchestrator (4-6 Weeks)
Goal: Expand the daemon to manage 10-15 agents across 10-15 projects with full headless orchestration.
Add  gRPC  streaming,  WAL  for  crash  recovery,  and  the  OrchestratorAgent's  core  loop.  Still  no  TUI  —
validate via gRPC client and CLI.
Weeks 3-4: gRPC and State
WeekDeliverableDetails
3hypervisor.proto v2
implementation
Implement SubscribeEvents, DispatchCommand, GetFullState RPCs.
FullStateSnapshot includes projects[], agent_slots[]. Event stream
includes ProjectBudgetAlert, SlotAgentSwapped.
3-4WAL (Write-Ahead Log) for
daemon state
Persist NexodeSession state to disk. On daemon crash/restart, reload
session, re-attach to running agent processes (if alive), or respawn into
slots. Test: kill daemon, restart, verify recovery.
4.nexode.yaml repo-local
merge
Daemon scans each project's repo root for .nexode.yaml. Merge:
repo-local slots added to session slots. Matching IDs: repo-local wins.
4gRPC test client (nexode-ctl)Simple Rust CLI that connects to daemon, displays state, sends
commands: nexode-ctl status, nexode-ctl kill , nexode-ctl reassign .
## Weeks 5-6: Orchestration
WeekDeliverableDetails
5OrchestratorAgent core loopRuns as a tokio task. Monitors slot states: when a slot's agent
completes, mark idle and emit event. When a slot has no agent,
auto-spawn. Phase 1: per-project orchestration only, no cross-project
intelligence.
5-6Autonomy tier enforcementImplement manual, plan, and full_auto modes per-slot. plan mode:
agent proposes plan, daemon pauses for approval via gRPC. full_auto:
runs continuously until complete or budget exceeded.
6Uncertainty routing (loop
detection)
Monitor stdout for repeated identical tool calls (> 3x). On detection:
pause agent, emit AGENT_STATE_BLOCKED. Human resolves via gRPC
command.
6Context compiler (basic)Generate context payload: task description + file list from
context.include/exclude globs + recent git diff. No AST analysis (Phase
4). Inject as system prompt or CLAUDE.md.
Weeks 7-8 (Buffer / Hardening)
- Scale testing: 15 agents across 10 projects on DGX Spark target

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 26
- Memory profiling: daemon RSS under 200MB with 15 agent processes
- gRPC stress test: 15 agent event streams multiplexed to 3 concurrent clients
- Documentation: README, session.yaml reference, nexode-ctl man page
- CI: cargo test + integration suite with mock agents
## Phase 1 Exit Criteria
- Demo: 10 agents across 8 projects, managed headlessly via nexode-ctl
- Demo: daemon crash + restart with full state recovery from WAL
- Demo: plan-mode agent proposes plan, operator approves via gRPC, agent executes
- Demo: loop detection triggers pause and BLOCKED event
- All acceptance criteria pass in CI

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 27
## Phase 0-1 Dependency Graph
Items on the same row can be developed in parallel. Items on subsequent rows depend on the row above.
LayerPhase 0 (Weeks 1-2)Phase 1 (Weeks 3-8)
## Config
session.yaml v2 parser models.yaml pricing.nexode.yaml merge Context compiler (basic)
## Domain
NexodeSession, Project, AgentSlot types State
machine transitions
WAL persistence OrchestratorAgent core loop
## Runtime
Agent spawner (multi-repo) Process lifecycle
## (spawn/monitor/kill)
Autonomy tiers (manual/plan/full_auto)
Uncertainty routing
## Accountin
g
SQLite schema v2 (project_id, slot_id) Token
Accountant + pricing
Per-project budget enforcement Slot cost
continuity across crashes
## IPC
—hypervisor.proto v2 gRPC server + nexode-ctl
client
## Risk Register
RiskLIMitigation
Agent CLI incompatibility (stdout
format changes)
MHBuild AgentHarness abstraction that normalizes output. Test
against 2+ real CLIs in Phase 0.
Git worktree limits (too many in
one repo)
LMMulti-project model mitigates: typically 1-3 worktrees per repo.
Daemon can reuse worktrees with branch switching.
session.yaml v2 parser edge
cases
MLserde strict mode. JSON Schema validation before parse.
Extensive tests for includes, defaults cascade, v1 fallback.
Budget estimation accuracy
(pricing changes)
HMmodels.yaml is user-editable. Warn if > 30 days old. Future:
auto-fetch from provider APIs.
Daemon memory under 15
agents
LHPhase 1 buffer includes memory profiling. Daemon is lightweight
(Rust); risk is in agent process memory (separate OS processes).
DGX Spark compatibilityLMRust cross-compiles to aarch64. Test early on ARM. tokio and
tonic are fully ARM-compatible.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 28
- Phase 2: TUI Command Center (4-6 Weeks)
Primary  Stack:  Rust,  ratatui  (terminal  rendering),  tokio  (async  runtime).  Purpose:  Build  a  rich  terminal
interface  that  gives  a  single  developer  real-time  visibility  and  control  over  10+  concurrent  agents  across
multiple projects, before investing in the VS Code Extension Host.
v2 Changes (E-008): TUI now supports multi-project navigation with project-level grouping,
Tab/Shift+Tab project cycling, and per-project views. The '/' fuzzy search operates across all agents
and projects as the 'command palette' of the TUI.
UX/UI Layout
- Agent Grid: Dynamic ratatui layout that auto-tiles based on terminal width and active agent count.
Agents grouped by project with color-coded project headers.
- Per-Agent Panes: Real-time streaming output of the assigned worker process. Shows agent name,
worktree path, state, and last lines of stdout.
- Status Colors: Gray = idle, Teal = executing, Orange = blocked, Red = needs approval, Blue =
merging.
- Project Sidebar: Collapsible list of all projects with agent counts and cost summaries.
## Interactive Controls
- Project Navigation: Tab / Shift+Tab to cycle projects. 1-9 for direct project jump. Enter to focus a
project, Esc to return to portfolio view.
- Vim-Style Agent Navigation: h, j, k, l to jump between agent panes within a project.
- Agent Controls: c opens command overlay: pause, resume, kill, reassign task, promote to lead role.
- Worktree Diff Viewer: d flips pane from stdout stream to inline Git diff viewer for merge review.
- Global Fuzzy Search: / searches across all active agent streams and projects.
## Agent Harness Abstraction
Phase 2 proves the daemon can juggle multiple AI providers simultaneously. Trait-based harness adapters
for Claude Code, Codex CLI, and Gemini CLI allow mixing models within a single session. Configuration via
.nexode.yaml or session-level defaults.
## Session Persistence
If the terminal is closed, the TUI recovers by reading the SQLite state database and WAL, re-attaching to
the running daemon processes. Orchestration resumes exactly where it left off.
Phase 2 Success and Kill Criteria
- Success: A single developer manages 10+ agents across multiple projects from one terminal. At
least 3 different agent types operate through the harness abstraction.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 29
- Kill: If rendering 10+ active streams causes ratatui to lag or consume excessive CPU, optimize local
state caching before attempting gRPC bridge to VS Code.
- Phase 3: VS Code Integration (6-8 Weeks)
Primary Stack: TypeScript, React (Webviews), @grpc/grpc-js, VS Code Extension API. Purpose: Connect
the Rust Daemon to VS Code using an extension-first approach, delivering the Synapse Grid, Kanban DAG,
and native chat routing.
Week 1: gRPC Bridge and State Cache
- Connection: @grpc/grpc-js over local Unix domain socket (named pipe on Windows).
- StateCache: In-memory replica of FullStateSnapshot.
- EventBus (Barrier Sync): Collects incoming events by barrier_id. Updates StateCache. Fires single
unified postMessage to all React Webviews. Holds new events in queue until ACK received from all
active Webviews.
Weeks 2-4: Multi-Monitor React Webviews
- Synapse Grid: vscode.window.createWebviewPanel. Virtualized react-grid-layout. Uses React
useRef + direct DOM mutation for 60fps terminal streams.
- Kanban and DAG: reactflow node-based renderer. Drag-and-drop task assignment triggers
AssignTask gRPC command. Cross-project swim lanes (Phase 3+).
Weeks 5-6: Native VS Code Integrations
- ChatParticipant: vscode.chat.createChatParticipant('nexode.orchestrator', handler). Routes
@nexode commands via gRPC.
- Sidebar View: Compressed Synapse Grid via WebviewViewProvider. 'Pop-out to Full Screen' button.
- Merge Choreography: TreeView showing worktrees in REVIEW state. Click opens VS Code native
diff editor.
- HUD StatusBar: Updates every 1000ms: active count, tok/s, cost, per-project breakdown.
## Week 7: Worktree Router
"Take Over" command: when user clicks on a blocked agent, the extension dynamically adds that agent's
hidden  worktree  to  the  VS  Code  workspace  via  vscode.workspace.updateWorkspaceFolders().  Developer
fixes the issue, saves, clicks 'Resume Agent.'
Phase 3 Success and Kill Criteria
- Success: EventBus processes 1,000 token updates/sec across 15 agents without editor freeze.
React webview panels sync perfectly across monitors. Orchestrator routes UI events natively.
- Kill (Deep Fork Trigger): If the Extension Host IPC channel cannot handle streaming telemetry
volume despite optimized useRef renders and barrier sync, halt Phase 3 and trigger Phase 5.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 30
- Phase 4: Smart Context and Semantic Memory
## (6-8 Weeks)
Primary   Stack:   Rust,   Tree-sitter   (incremental   AST   parsing),   LanceDB   (embedded   vector   search),
embedding  models  (OpenAI  or  local).  Purpose:  Transition  from  file-centric  prompting  to  AST-aware
semantic context injection. 20-40% token reduction.
Step 1: Real-Time AST Indexing (Tree-sitter)
- Codebase Graph: Parse entire codebase into AST on init. Incremental updates via file watchers as
agents mutate code.
- Signature Extraction: Strip function bodies. Extract only structural signatures (interfaces, types, class
declarations, method headers). Creates lightweight 'Code Intent' map.
Step 2: LanceDB Vector Memory Bus
- Embed AST signatures and store in LanceDB (embedded, inside daemon).
- Decision Log: Every agent's reasoning and architectural decisions are embedded and persisted. A
new agent queries: 'What decisions were made about the database schema?' and gets Agent A's
reasoning.
## Step 3: Scoped Context Compiler
Three-part compilation for each slot dispatch:
- Fetch Persona: Load system instructions for the assigned role.
- Fetch Structural Context (AST): Query LanceDB for code signatures relevant to the task. No file
bloat.
- Fetch Historical Intent: Query Decision Log for past decisions related to the task domain.
## Step 4: Predictive Conflict Routing
Before merging worktrees, the daemon compares AST mutations. If Agent A modified a function body and
Agent B added a parameter to the same function's signature, Git might allow the merge but the code will
break. Tree-sitter flags this as High Structural Conflict Risk. The merge node in the DAG turns red in the UI.
Phase 4 Success and Kill Criteria
- Success: Token consumption drops 20%+ due to AST-stripping. Fungible agent completes
dependent task using only LanceDB Decision Log. AST parser adds no noticeable latency to
Orchestrator tick loop.
- Kill: If embeddings and LanceDB updates cause the daemon's async runtime to stutter and miss UI
telemetry, offload vector DB to a separate microservice or throttle update frequency.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 31
- Phase 5: Deep Fork (Conditional)
Break glass in case of emergency. Phase 5 is only triggered if Phase 3's extension-based approach
definitively fails — specifically, if the Extension Host IPC chokes on serialization overhead of 15 agents
streaming tokens simultaneously, or if native UI capabilities hit an insurmountable wall.
The Architectural Shift: Bypassing the Extension Host
Move  HypervisorClient  and  EventBus  directly  into  VS  Code's  core  dependency  injection  framework  as
IHypervisorService.  Zero  extension  host  serialization  overhead.  Direct,  synchronous  access  to  every
internal VS Code API.
UI Overhaul: Native DOM
Strip  React  Webviews.  Rewrite  Synapse  Grid  and  Kanban  using  VS  Code's  native  FastDomNode,
SplitView, and Grid toolkit. 60fps with near-zero memory overhead.
Exploiting the Sessions Layer
VS  Code  has  a  hidden  src/vs/sessions/  layer  (~26K  lines)  built  for  agentic  workflows.  Fork  and  expand
AgenticParts       to       include       SynapseGridPart       and       MacroKanbanPart.       Take       control       of
ViewContainerLocation.ChatBar.
Native Worktree and SCM Isolation
Rewrite the core workspace and SCM logic: Explorer natively distinguishes 'Human Main Codebase' from
'Agent Sandboxes.' Clicking an agent in the Synapse Grid context-switches the entire VS Code window to
that agent's git worktree.
## Maintenance Strategy
- Strict Boundary Rules: No core VS Code files heavily mutated. All Hypervisor logic in
src/vs/hypervisor/.
- Aspect-Oriented Patches: Patch-management system (quilt or git patch tracking) for minimal core
hooks. Monthly rebases automated.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 32
Appendix A: Agent Pools and .swarm/ Protocol
Deferred to Phase 3+
Deferral Note (E-007): This section documents the Agent Pool architecture as designed in Spec v1. It is
fully specified but deferred from the MVP (Phase 0-1) because the multi-project model (E-001) makes
pool coordination unnecessary for the primary use case. When the need arises for multiple agents
collaborating within a single project, this architecture is ready for implementation.
Agent  Pools  evolve  the  architecture  from  purely  horizontal  task  distribution  (15  agents  doing  unrelated
things)  to  horizontal  pools  of  vertical  collaborators.  A  'Sprint  Pool'  of  1-4  agents  is  assigned  to  a  single
Macro Kanban card.
Pool Structure (Role-Based Micro-Swarms)
- Agent 1 (The Builder): Writes core implementation logic in src/.
- Agent 2 (The Tester): Observes Builder's interfaces, writes tests in tests/.
- Agent 3 (The Reviewer/Documenter): Reviews code, writes docstrings, updates README.
The .swarm/ File-Based Message Bus
Agents  in  a  pool  share  a  .hypervisor/worktrees/pool-A/.swarm/  directory.  The  Builder  writes  status  to
.swarm/builder_status.md. The daemon watches this directory and alerts the Tester when updates appear.
## Coordination Constraints
- Mutually Exclusive Mutation Zones: Directory-level write locks. Builder owns /src, Tester owns
/tests, Documenter owns .md files. Zero file-level merge conflicts.
- Pipeline DAG (Strict Ordering): Tester cannot start until Builder commits the interface. If tests fail,
Tester assigns a 'Bug' sub-task back to the Builder via .swarm/.
- "Stop the Line" Rule: If Builder and Tester loop (write > fail > same write > fail), the
OrchestratorAgent pauses the entire pool and flags the UI.
Pool Persona YAML Example
YAML — Scenario A: Sequential Feature Pod

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 33
version: "1.0"
task_id: "auth-middleware-update"
description: "Implement JWT rotation and ensure full test coverage."
pool_config:
shared_branch: "feature/jwt-rotation"
agents:
- id: "builder-1"
model: "claude-code"
persona: "backend-engineer"
context_focus: ["src/middleware/auth.ts", "DecisionLog:JWT_Strategy"]
mutation_zones:
allow: ["src/middleware/**"]
deny: ["tests/**", "docs/**"]
- id: "tester-1"
model: "claude-code"
persona: "qa-automation-expert"
context_focus: ["src/middleware/auth.ts"]
mutation_zones:
allow: ["tests/middleware/**"]
deny: ["src/**", "docs/**"]
- id: "doc-1"
model: "gemini-cli"
persona: "technical-writer"
context_focus: ["src/middleware/auth.ts", "tests/middleware/**"]
mutation_zones:
allow: ["docs/auth.md", "README.md"]
deny: ["src/**", "tests/**"]
workflow_dag:
- step: "implement_logic"
assignee: "builder-1"
prompt: "Update auth.ts to handle JWT refresh token rotation."
on_success: trigger_tests
- step: "trigger_tests"
assignee: "tester-1"
prompt: "Write unit tests. Read .swarm/builder_status.md for context."
depends_on: ["implement_logic"]
on_failure: loop_back_to_builder
on_success: trigger_docs
- step: "trigger_docs"
assignee: "doc-1"
depends_on: ["trigger_tests"]

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 34
Appendix B: Licensing and Distribution Strategy
Errata Reference (E-002, modified): Placed as appendix rather than inline section to maintain the
spec's engineering focus. Includes the addition of Tier 0.5 (TUI plugin system) and clarification that
'bring your own API key' is Tier 0 behavior.
Open  source  is  not  charity  —  it  is  a  structural  moat.  Augment  charges  $200/month  for  intensive  usage.
Gas  Town  is  MIT  but  Claude-only.  Nexode's  strategy:  make  the  universal  primitive  (daemon  +  TUI)  free
and open, then build premium value on top.
TierComponentLicenseRevenue Model
0 (Free)Rust Daemon + Core
## Libraries
MIT / Apache 2.0Community adoption. Ecosystem gravity. Bring your
own API key.
## 0.5
(Free)
TUI + TUI Plugin
## System
MIT / Apache 2.0Third-party TUI plugins (dashboards, provider
integrations) create ecosystem.
## Community-contributed.
1 (Freemi
um)
VS Code Extension
## Pack
## Source-available /
## BSL
Free tier: 3 agents, 1 project. Paid tier: unlimited
agents/projects, advanced features.
2 (Paid)Enterprise / Cloud
## Features
ProprietaryTeam orchestration, managed API key pooling
(convenience, not requirement), audit logs, SSO.
## Strategic Rationale
- Tier 0 (daemon) is the foundation that every layer depends on. MIT ensures trust and adoption.
- Tier 0.5 (TUI plugins) creates ecosystem gravity without requiring VS Code. Power users can extend
the TUI for custom dashboards and provider integrations.
- Tier 1 (VS Code) is the natural upgrade path. The extension adds visual value that justifies payment.
- Tier 2 (Enterprise) is the long-term revenue engine. Team features are where SaaS margins live.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 35
## Appendix C: Karpathy Alignment Matrix
Errata Reference (E-010, modified): Placed as appendix (design rationale artifact, not specification).
Added error recovery row per triage modification.
Karpathy's  tweet:  "tmux  grids  are  awesome,  but  i  feel  a  need  to  have  a  proper  'agent  command  center'
IDE for teams of them." This matrix maps his specific requirements to Nexode spec features.
Karpathy RequestSpec v1 Featurev2 EnhancementPhase
Agent command
center IDE
VS Code Extension
Pack (Layer 3)
Multi-project grouped grid
## (E-006)
## Phase
## 3
tmux gridsSynapse Telemetry
## Grid (ratatui +
## Webview)
Project-grouped TUI navigation
## (E-008)
## Phase
## 2
Teams of agents15 parallel agents,
OrchestratorAgent
10-15 across 10-15 projects
## (E-001)
## Phase 1
## Agent
observability
Token velocity, cost
HUD, status colors
Per-project cost tracking
## (E-009)
## Phase
## 0
## Agent
coordination
## Shared Memory Bus,
## Decision Log
AgentSlot abstraction (E-003)Phase
## 4
Error recovery /
restart
Uncertainty routing
(orange pulse)
AgentSlot crash recovery: new
agent spawns into same slot
## Phase 1

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 36
## Appendix D: Competitive Differentiation
Nexode occupies a unique position: the free, open-source agent command center for developers who run
multiple AI coding agents across multiple projects.
AxisNexodeAugmentGas TownVS Code
## Native
## Antigravity
PriceFree (Tier
## 0/0.5)
$200/moFree (MIT)FreeUnknown
Multi-ProjectYes (primary)Single
workspace
Single repoSingle
workspace
## Unknown
Model-AgnosticYes (harness)ProprietaryClaude
only
Copilot/GP
## T
## Gemini
only
ObservabilityFull telemetry
grid
## Basictmux
output
MinimalUnknown
Local-FirstYes (DGX
## Spark)
CloudLocalCloudCloud
TUI-FirstYes (Phase 2)NoYes (tmux)NoNo
Open SourceMIT daemon +
## TUI
NoMITNoNo
## Positioning Statement
Nexode is the free, open-source agent command center for developers who run multiple AI coding agents
across multiple projects. It is tmux with a brain: a Rust daemon that orchestrates any CLI agent (Claude,
Codex,  Gemini,  local)  across  any  number  of  codebases,  with  per-project  cost  tracking,  crash  recovery,
and a TUI/VS Code command center for real-time observability.

Nexode Agent IDE — Master Specification v2March 2026
Perplexity Computer | XcognisPage 37
## Appendix E: Errata Incorporation Log
This  log  tracks  how  each  errata  item  from  Errata  001  was  incorporated  into  this  Master  Spec  v2.  All  11
items are incorporated: 7 as-is (ACCEPT), 4 with modifications (MODIFY).
IDTitleVerdictWhere Incorporated
E-001Core Reframing:
Multi-Project
ACCEPTSection 1 rewritten. User persona table added. Multi-project is
now the primary framing throughout.
E-002Open Source as
## Structural Moat
MODIFYAppendix B. Added Tier 0.5 (TUI plugins). Clarified BYOK = Tier
## 0.
E-003New Domain ModelACCEPTSection 3.1. NexodeSession > Project > AgentSlot > Agent
hierarchy throughout.
E-004YAML/TOML Schema
## Revision
MODIFYSection 4. Added defaults block, include directives,
.nexode.yaml merging, tags.
E-005Protobuf Schema
## Additions
ACCEPTSection 3.4. Project, AgentSlot, ProjectBudgetAlert,
SlotAgentSwapped added.
E-006UI/UX RevisionsMODIFYSection 5. Project-grouped views. Monitor assignment deferred
to Phase 4+. Simplified idle reallocation.
E-007Phase 0-1 Scope
## Reduction
ACCEPTSections 7-9. Pools/.swarm/ moved to Appendix A. New Phase
0-1 plan with multi-project features.
E-008TUI Multi-Project
## Navigation
ACCEPTSection 10. Tab/Shift+Tab, 1-9, hjkl, fuzzy search across all
projects.
E-009Per-Project Cost
## Accounting
ACCEPTSection 6. Four-level model. SQLite schema v2. project_costs
view.
E-010Karpathy Alignment
## Matrix
MODIFYAppendix C. Added error recovery row. Placed as appendix
(rationale, not spec).
E-011Competitive
## Differentiation
ACCEPTAppendix D. Six-axis comparison table with positioning
statement.
End  of  Specification.  The  Nexode  multi-project  model  is  fully  specified  at  the  domain,  configuration,
protocol,  UI,  orchestration,  and  implementation  plan  levels.  The  path  from  session.yaml  v2  to  a  working
15-agent headless orchestrator is clear, scoped, and de-risked.