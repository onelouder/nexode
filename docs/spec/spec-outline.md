# Nexode Agent IDE Master Specification v2 Outline

Source: `Nexode-Master-Specification-v2.pdf`

Note: The stable IDs below are editorial reference IDs derived from the source headings. They are intended to make the specification easier to cite without changing the source wording or scope.

## 1. Hierarchical Outline

| ID | Parent | Level | Kind | Source heading |
|---|---|---:|---|---|
| `nexode-spec-v2` | — | 0 | root | Nexode Agent IDE Master Specification v2 |
| `sec-01-executive-summary-core-philosophy` | `nexode-spec-v2` | 1 | section | 1. Executive Summary and Core Philosophy |
| `sec-01-core-reframing-v2` | `sec-01-executive-summary-core-philosophy` | 2 | callout | Core Reframing (v2) |
| `sec-01-user-personas` | `sec-01-executive-summary-core-philosophy` | 2 | subsection | User Personas |
| `sec-01-design-principles` | `sec-01-executive-summary-core-philosophy` | 2 | subsection | Design Principles |
| `sec-02-system-architecture-4-layer-model` | `nexode-spec-v2` | 1 | section | 2. System Architecture (The 4-Layer Model) |
| `sec-02-key-architectural-property` | `sec-02-system-architecture-4-layer-model` | 2 | subsection | Key Architectural Property |
| `sec-02-layer-1-rust-daemon-core-architecture` | `sec-02-system-architecture-4-layer-model` | 2 | subsection | Layer 1: Rust Daemon Core Architecture |
| `sec-02-daemon-subsystem-breakdown` | `sec-02-system-architecture-4-layer-model` | 2 | subsection | Daemon Subsystem Breakdown |
| `sec-02-conceptual-rust-skeleton` | `sec-02-system-architecture-4-layer-model` | 2 | labeled block | Conceptual Rust Skeleton |
| `sec-03-domain-types-state-management` | `nexode-spec-v2` | 1 | section | 3. Domain Types and State Management |
| `sec-03-01-domain-model-hierarchy` | `sec-03-domain-types-state-management` | 2 | subsection | 3.1 Domain Model Hierarchy |
| `sec-03-01-new-in-v2-e-003` | `sec-03-01-domain-model-hierarchy` | 3 | callout | New in v2 (E-003) |
| `sec-03-02-fungible-workers-agentslot-abstraction` | `sec-03-domain-types-state-management` | 2 | subsection | 3.2 Fungible Workers and the AgentSlot Abstraction |
| `sec-03-03-shared-memory-bus` | `sec-03-domain-types-state-management` | 2 | subsection | 3.3 Shared Memory Bus |
| `sec-03-04-hypervisor-proto-v2-contract` | `sec-03-domain-types-state-management` | 2 | subsection | 3.4 hypervisor.proto v2 Contract |
| `sec-03-04-service-enums` | `sec-03-04-hypervisor-proto-v2-contract` | 3 | labeled block | Protocol Buffers — hypervisor.proto v2 (Service and Enums) |
| `sec-03-04-entity-models` | `sec-03-04-hypervisor-proto-v2-contract` | 3 | labeled block | Protocol Buffers — hypervisor.proto v2 (Entity Models) |
| `sec-03-04-events-commands` | `sec-03-04-hypervisor-proto-v2-contract` | 3 | labeled block | Protocol Buffers — hypervisor.proto v2 (Events and Commands) |
| `sec-03-04-design-highlights` | `sec-03-04-hypervisor-proto-v2-contract` | 3 | subsection | Design Highlights |
| `sec-04-session-configuration-session-yaml-v2` | `nexode-spec-v2` | 1 | section | 4. Session Configuration (session.yaml v2) |
| `sec-04-new-in-v2-e-004` | `sec-04-session-configuration-session-yaml-v2` | 2 | callout | New in v2 (E-004, as modified) |
| `sec-04-01-design-principles` | `sec-04-session-configuration-session-yaml-v2` | 2 | subsection | 4.1 Design Principles |
| `sec-04-02-complete-schema-annotations` | `sec-04-session-configuration-session-yaml-v2` | 2 | subsection | 4.2 Complete Schema with Annotations |
| `sec-04-02-session-model-config` | `sec-04-02-complete-schema-annotations` | 3 | labeled block | YAML — ~/.nexode/session.yaml v2 (Session and Model Config) |
| `sec-04-02-projects-slots` | `sec-04-02-complete-schema-annotations` | 3 | labeled block | YAML — ~/.nexode/session.yaml v2 (Projects and Slots) |
| `sec-04-03-schema-field-reference` | `sec-04-session-configuration-session-yaml-v2` | 2 | subsection | 4.3 Schema Field Reference |
| `sec-04-03-session-level-fields` | `sec-04-03-schema-field-reference` | 3 | subsection | Session-Level Fields |
| `sec-04-03-project-level-fields` | `sec-04-03-schema-field-reference` | 3 | subsection | Project-Level Fields |
| `sec-04-03-slot-level-fields` | `sec-04-03-schema-field-reference` | 3 | subsection | Slot-Level Fields |
| `sec-04-04-per-project-nexode-yaml` | `sec-04-session-configuration-session-yaml-v2` | 2 | subsection | 4.4 Per-Project .nexode.yaml (Repo-Local Override) |
| `sec-04-04-repo-local-yaml-example` | `sec-04-04-per-project-nexode-yaml` | 3 | labeled block | YAML — ~/projects/my-saas/.nexode.yaml |
| `sec-04-05-backward-compatibility-v1-schema` | `sec-04-session-configuration-session-yaml-v2` | 2 | subsection | 4.5 Backward Compatibility: v1 Schema |
| `sec-04-05-minimal-quick-start` | `sec-04-05-backward-compatibility-v1-schema` | 3 | subsection | Minimal Quick-Start (8 lines) |
| `sec-04-05-minimal-session-yaml` | `sec-04-05-backward-compatibility-v1-schema` | 3 | labeled block | YAML — Minimal session.yaml |
| `sec-05-ui-ux-surfaces-command-center` | `nexode-spec-v2` | 1 | section | 5. UI/UX Surfaces (The Command Center) |
| `sec-05-v2-changes-e-006` | `sec-05-ui-ux-surfaces-command-center` | 2 | callout | v2 Changes (E-006, as modified) |
| `sec-05-synapse-telemetry-grid` | `sec-05-ui-ux-surfaces-command-center` | 2 | subsection | Synapse Telemetry Grid |
| `sec-05-macro-kanban-board-task-queue` | `sec-05-ui-ux-surfaces-command-center` | 2 | subsection | Macro Kanban Board and Task Queue |
| `sec-05-universal-command-chat` | `sec-05-ui-ux-surfaces-command-center` | 2 | subsection | Universal Command Chat |
| `sec-05-merge-choreography` | `sec-05-ui-ux-surfaces-command-center` | 2 | subsection | Merge Choreography |
| `sec-05-status-bar-hud` | `sec-05-ui-ux-surfaces-command-center` | 2 | subsection | Status Bar HUD |
| `sec-06-orchestration-autonomy-orchestratoragent` | `nexode-spec-v2` | 1 | section | 6. Orchestration and Autonomy (The OrchestratorAgent) |
| `sec-06-auto-dispatch` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | Auto-Dispatch |
| `sec-06-autonomy-tiers` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | Autonomy Tiers |
| `sec-06-uncertainty-routing-loop-detection` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | Uncertainty Routing (Loop Detection) |
| `sec-06-observer-agent-three-monitoring-loops` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | Observer Agent: Three Monitoring Loops |
| `sec-06-per-project-cost-enforcement` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | Per-Project Cost Enforcement |
| `sec-06-new-in-v2-e-009` | `sec-06-per-project-cost-enforcement` | 3 | callout | New in v2 (E-009) |
| `sec-06-sqlite-schema-cost-tracking` | `sec-06-orchestration-autonomy-orchestratoragent` | 2 | subsection | SQLite Schema for Cost Tracking |
| `sec-06-token-accounting-schema` | `sec-06-sqlite-schema-cost-tracking` | 3 | labeled block | SQL — Token Accounting Schema |
| `sec-07-phased-development-timeline-overview` | `nexode-spec-v2` | 1 | section | 7. Phased Development Timeline (Overview) |
| `sec-07-v2-scope-change-e-007` | `sec-07-phased-development-timeline-overview` | 2 | callout | v2 Scope Change (E-007) |
| `sec-08-phase-0-spike-validate` | `nexode-spec-v2` | 1 | section | 8. Phase 0: Spike and Validate (2 Weeks) |
| `sec-08-week-1-foundation` | `sec-08-phase-0-spike-validate` | 2 | subsection | Week 1: Foundation |
| `sec-08-week-2-agent-lifecycle` | `sec-08-phase-0-spike-validate` | 2 | subsection | Week 2: Agent Lifecycle |
| `sec-08-phase-0-exit-criteria` | `sec-08-phase-0-spike-validate` | 2 | subsection | Phase 0 Exit Criteria |
| `sec-08-kill-criteria` | `sec-08-phase-0-spike-validate` | 2 | subsection | Kill Criteria |
| `sec-09-phase-1-headless-orchestrator` | `nexode-spec-v2` | 1 | section | 9. Phase 1: Headless Orchestrator (4-6 Weeks) |
| `sec-09-weeks-3-4-grpc-state` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Weeks 3-4: gRPC and State |
| `sec-09-weeks-5-6-orchestration` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Weeks 5-6: Orchestration |
| `sec-09-weeks-7-8-buffer-hardening` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Weeks 7-8 (Buffer / Hardening) |
| `sec-09-phase-1-exit-criteria` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Phase 1 Exit Criteria |
| `sec-09-phase-0-1-dependency-graph` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Phase 0-1 Dependency Graph |
| `sec-09-risk-register` | `sec-09-phase-1-headless-orchestrator` | 2 | subsection | Risk Register |
| `sec-10-phase-2-tui-command-center` | `nexode-spec-v2` | 1 | section | 10. Phase 2: TUI Command Center (4-6 Weeks) |
| `sec-10-v2-changes-e-008` | `sec-10-phase-2-tui-command-center` | 2 | callout | v2 Changes (E-008) |
| `sec-10-ux-ui-layout` | `sec-10-phase-2-tui-command-center` | 2 | subsection | UX/UI Layout |
| `sec-10-interactive-controls` | `sec-10-phase-2-tui-command-center` | 2 | subsection | Interactive Controls |
| `sec-10-agent-harness-abstraction` | `sec-10-phase-2-tui-command-center` | 2 | subsection | Agent Harness Abstraction |
| `sec-10-session-persistence` | `sec-10-phase-2-tui-command-center` | 2 | subsection | Session Persistence |
| `sec-10-phase-2-success-kill-criteria` | `sec-10-phase-2-tui-command-center` | 2 | subsection | Phase 2 Success and Kill Criteria |
| `sec-11-phase-3-vscode-integration` | `nexode-spec-v2` | 1 | section | 11. Phase 3: VS Code Integration (6-8 Weeks) |
| `sec-11-week-1-grpc-bridge-state-cache` | `sec-11-phase-3-vscode-integration` | 2 | subsection | Week 1: gRPC Bridge and State Cache |
| `sec-11-weeks-2-4-multi-monitor-react-webviews` | `sec-11-phase-3-vscode-integration` | 2 | subsection | Weeks 2-4: Multi-Monitor React Webviews |
| `sec-11-weeks-5-6-native-vscode-integrations` | `sec-11-phase-3-vscode-integration` | 2 | subsection | Weeks 5-6: Native VS Code Integrations |
| `sec-11-week-7-worktree-router` | `sec-11-phase-3-vscode-integration` | 2 | subsection | Week 7: Worktree Router |
| `sec-11-phase-3-success-kill-criteria` | `sec-11-phase-3-vscode-integration` | 2 | subsection | Phase 3 Success and Kill Criteria |
| `sec-12-phase-4-smart-context-semantic-memory` | `nexode-spec-v2` | 1 | section | 12. Phase 4: Smart Context and Semantic Memory (6-8 Weeks) |
| `sec-12-step-1-real-time-ast-indexing` | `sec-12-phase-4-smart-context-semantic-memory` | 2 | subsection | Step 1: Real-Time AST Indexing (Tree-sitter) |
| `sec-12-step-2-lancedb-vector-memory-bus` | `sec-12-phase-4-smart-context-semantic-memory` | 2 | subsection | Step 2: LanceDB Vector Memory Bus |
| `sec-12-step-3-scoped-context-compiler` | `sec-12-phase-4-smart-context-semantic-memory` | 2 | subsection | Step 3: Scoped Context Compiler |
| `sec-12-step-4-predictive-conflict-routing` | `sec-12-phase-4-smart-context-semantic-memory` | 2 | subsection | Step 4: Predictive Conflict Routing |
| `sec-12-phase-4-success-kill-criteria` | `sec-12-phase-4-smart-context-semantic-memory` | 2 | subsection | Phase 4 Success and Kill Criteria |
| `sec-13-phase-5-deep-fork-conditional` | `nexode-spec-v2` | 1 | section | 13. Phase 5: Deep Fork (Conditional) |
| `sec-13-architectural-shift-bypassing-extension-host` | `sec-13-phase-5-deep-fork-conditional` | 2 | subsection | The Architectural Shift: Bypassing the Extension Host |
| `sec-13-ui-overhaul-native-dom` | `sec-13-phase-5-deep-fork-conditional` | 2 | subsection | UI Overhaul: Native DOM |
| `sec-13-exploiting-sessions-layer` | `sec-13-phase-5-deep-fork-conditional` | 2 | subsection | Exploiting the Sessions Layer |
| `sec-13-native-worktree-scm-isolation` | `sec-13-phase-5-deep-fork-conditional` | 2 | subsection | Native Worktree and SCM Isolation |
| `sec-13-maintenance-strategy` | `sec-13-phase-5-deep-fork-conditional` | 2 | subsection | Maintenance Strategy |
| `appendices` | `nexode-spec-v2` | 1 | section group | Appendices |
| `app-a-agent-pools-swarm-protocol` | `appendices` | 2 | appendix | Appendix A: Agent Pools and .swarm/ Protocol |
| `app-a-deferral-note-e-007` | `app-a-agent-pools-swarm-protocol` | 3 | callout | Deferral Note (E-007) |
| `app-a-pool-structure-role-based-micro-swarms` | `app-a-agent-pools-swarm-protocol` | 3 | subsection | Pool Structure (Role-Based Micro-Swarms) |
| `app-a-swarm-file-based-message-bus` | `app-a-agent-pools-swarm-protocol` | 3 | subsection | The .swarm/ File-Based Message Bus |
| `app-a-coordination-constraints` | `app-a-agent-pools-swarm-protocol` | 3 | subsection | Coordination Constraints |
| `app-a-pool-persona-yaml-example` | `app-a-agent-pools-swarm-protocol` | 3 | subsection | Pool Persona YAML Example |
| `app-b-licensing-distribution-strategy` | `appendices` | 2 | appendix | Appendix B: Licensing and Distribution Strategy |
| `app-b-errata-reference-e-002` | `app-b-licensing-distribution-strategy` | 3 | callout | Errata Reference (E-002, modified) |
| `app-b-strategic-rationale` | `app-b-licensing-distribution-strategy` | 3 | subsection | Strategic Rationale |
| `app-c-karpathy-alignment-matrix` | `appendices` | 2 | appendix | Appendix C: Karpathy Alignment Matrix |
| `app-c-errata-reference-e-010` | `app-c-karpathy-alignment-matrix` | 3 | callout | Errata Reference (E-010, modified) |
| `app-d-competitive-differentiation` | `appendices` | 2 | appendix | Appendix D: Competitive Differentiation |
| `app-d-positioning-statement` | `app-d-competitive-differentiation` | 3 | subsection | Positioning Statement |
| `app-e-errata-incorporation-log` | `appendices` | 2 | appendix | Appendix E: Errata Incorporation Log |

## 2. Glossary of Domain Terms

| Term | Definition | Primary IDs |
|---|---|---|
| `.nexode.yaml` | Repo-local configuration file merged into a matching project entry when the daemon discovers that repo. Repo-local slots are added, and matching slot IDs take precedence. | `sec-04-04-per-project-nexode-yaml`, `sec-09-weeks-3-4-grpc-state` |
| `.swarm/` | File-based message bus directory used by the deferred Appendix A pool architecture so pooled agents can coordinate through watched files. | `app-a-swarm-file-based-message-bus` |
| `Agent` | The ephemeral compute worker: a raw CLI process bound to a slot. It can crash, exit, or be replaced while the slot remains stable. | `sec-03-01-domain-model-hierarchy`, `sec-03-02-fungible-workers-agentslot-abstraction` |
| `Agent CLI` | A provider-facing CLI runtime such as Claude Code, Codex CLI, Gemini CLI, or a local model, treated by Nexode as interchangeable compute through a harness abstraction. | `sec-01-design-principles`, `sec-10-agent-harness-abstraction` |
| `Agent Harness abstraction` | The adapter layer that normalizes multiple AI providers so one session can mix Claude Code, Codex CLI, Gemini CLI, and local models. | `sec-01-design-principles`, `sec-10-agent-harness-abstraction`, `sec-09-risk-register` |
| `AgentMode` | The protocol enum used to represent an agent's autonomy mode in the gRPC contract. | `sec-03-04-service-enums`, `sec-03-04-entity-models` |
| `Agent Pool` | Deferred Appendix A architecture in which a small set of agents collaborates on one macro task inside a single project. | `app-a-agent-pools-swarm-protocol`, `app-a-pool-structure-role-based-micro-swarms` |
| `Agent Process Manager` | Daemon subsystem that spawns, monitors, terminates, and watchdogs fungible CLI agent processes. | `sec-02-daemon-subsystem-breakdown` |
| `Agent Process Runner` | One of the daemon's async tasks that reads an agent's stdout and stderr, parses output, and streams telemetry to the Core Engine. | `sec-02-layer-1-rust-daemon-core-architecture` |
| `Agent Sandboxes` | Phase 5 term for agent-specific worktree contexts that the forked editor would distinguish from the human main codebase. | `sec-13-native-worktree-scm-isolation` |
| `AgentSlot` | The stable work unit between Project and Agent. It owns task identity, cost continuity, and worktree continuity while the underlying agent process remains replaceable. | `sec-03-01-domain-model-hierarchy`, `sec-03-02-fungible-workers-agentslot-abstraction` |
| `AgentState` | The protocol enum that describes agent lifecycle states such as init, idle, executing, review, blocked, and terminated. | `sec-03-04-service-enums` |
| `Autonomy tier` | Per-slot operating mode that governs how much the agent can do without approval. The document names `manual`, `plan`, and `full_auto`. | `sec-06-autonomy-tiers`, `sec-04-02-session-model-config` |
| `barrier_id` | Event field used to coordinate synchronized UI updates so all webviews render the same state boundary together. | `sec-03-04-events-commands`, `sec-03-04-design-highlights`, `sec-11-week-1-grpc-bridge-state-cache` |
| `blank worker` | A fungible agent process before role/context injection, or the replacement worker spawned back into an existing slot after a crash or hang. | `sec-03-02-fungible-workers-agentslot-abstraction` |
| `BYOK` | "Bring your own API key." Appendix B clarifies this behavior belongs to Tier 0. | `app-b-errata-reference-e-002`, `app-b-licensing-distribution-strategy` |
| `ChatDispatch` | Natural-language operator command sent from the VS Code ChatBar to the OrchestratorAgent for parsing and action routing. | `sec-03-04-events-commands`, `sec-03-04-design-highlights`, `sec-05-universal-command-chat` |
| `ChatParticipant` | VS Code native chat participant registered by the extension so `@nexode` commands can be routed through the daemon. | `sec-05-universal-command-chat`, `sec-11-weeks-5-6-native-vscode-integrations` |
| `Code Intent` | The structural meaning captured from AST signatures, dependency relationships, and import maps rather than full file bodies. | `sec-03-03-shared-memory-bus`, `sec-12-step-1-real-time-ast-indexing` |
| `Codebase Graph` | The AST-backed structural index of a codebase queried by the Scoped Context Compiler to reduce context window bloat. | `sec-03-03-shared-memory-bus`, `sec-12-step-1-real-time-ast-indexing` |
| `Command Center` | The spec's framing for Nexode as an agent-centric control surface for many concurrent coding agents across many projects. | `sec-01-executive-summary-core-philosophy`, `sec-05-ui-ux-surfaces-command-center`, `sec-10-phase-2-tui-command-center` |
| `Context compiler (basic)` | Phase 1 context builder that combines task description, include/exclude globs, and recent git diff without AST analysis. | `sec-09-weeks-5-6-orchestration` |
| `Core Engine` | The daemon's master state machine that consumes UI commands, agent outputs, and timed triggers before broadcasting resulting events. | `sec-02-layer-1-rust-daemon-core-architecture`, `sec-02-conceptual-rust-skeleton` |
| `DAG` | The dependency graph used for task sequencing and Kanban visualization. | `sec-02-daemon-subsystem-breakdown`, `sec-05-macro-kanban-board-task-queue`, `sec-09-phase-0-1-dependency-graph` |
| `Decision Log` | Append-only record of reasoning traces, architectural decisions, and rejected alternatives used to align later agents. | `sec-03-03-shared-memory-bus`, `sec-12-step-2-lancedb-vector-memory-bus`, `sec-12-step-3-scoped-context-compiler` |
| `Deep Fork` | Conditional Phase 5 plan to fork VS Code only if the extension-based path fails on streaming telemetry scale or native UI limits. | `sec-01-design-principles`, `sec-07-phased-development-timeline-overview`, `sec-13-phase-5-deep-fork-conditional` |
| `defaults block` | Session-level YAML block that removes repetition by setting default model, mode, timeout, and provider configuration. | `sec-04-new-in-v2-e-004`, `sec-04-01-design-principles`, `sec-04-02-session-model-config` |
| `developer fatigue meter` | Status Bar HUD metric intended to summarize operator load alongside active agents, token velocity, and cost. | `sec-05-status-bar-hud` |
| `DGX Spark` | Example target machine used to illustrate remote daemon deployment and Phase 1 scale testing. | `sec-02-key-architectural-property`, `sec-09-weeks-7-8-buffer-hardening`, `app-d-competitive-differentiation` |
| `Dynamic Loop` | Observer monitoring loop that watches tool and terminal behavior for repeated calls and anomalous diff sizes. | `sec-06-observer-agent-three-monitoring-loops` |
| `EventBus` | Phase 3 UI-side event collector that batches incoming events by `barrier_id`, updates `StateCache`, and fans out unified view updates. | `sec-03-04-design-highlights`, `sec-11-week-1-grpc-bridge-state-cache`, `sec-13-architectural-shift-bypassing-extension-host` |
| `Extension Host` | The VS Code execution boundary whose IPC and serialization overhead motivate the extension-first design and, if necessary, the Deep Fork trigger. | `sec-02-system-architecture-4-layer-model`, `sec-11-phase-3-success-kill-criteria`, `sec-13-phase-5-deep-fork-conditional` |
| `Extension-first, fork-never (for now)` | The architecture principle that prioritizes a standalone daemon plus extension pack over an immediate VS Code fork. | `sec-01-design-principles`, `sec-02-system-architecture-4-layer-model` |
| `Flat View` | UI mode that places all agents in a single grid sorted by state. | `sec-05-synapse-telemetry-grid` |
| `Focus View` | UI mode that expands one project and compresses the rest into a sidebar list. | `sec-05-synapse-telemetry-grid` |
| `FullStateSnapshot` | gRPC response carrying the current global daemon state for connected clients. | `sec-03-04-entity-models`, `sec-09-weeks-3-4-grpc-state`, `sec-11-week-1-grpc-bridge-state-cache` |
| `fungible worker` | A raw agent process treated as interchangeable compute until the daemon binds it to a slot and injects task-specific context. | `sec-01-design-principles`, `sec-03-02-fungible-workers-agentslot-abstraction` |
| `Git Worktree Orchestrator` | Daemon subsystem that creates, assigns, merges, and garbage-collects isolated git worktrees across repositories. | `sec-02-daemon-subsystem-breakdown` |
| `gRPC Bridge` | Layer 2 IPC channel between the daemon and rendering shells, using a Unix socket or named pipe for high-throughput state/event streaming. | `sec-02-system-architecture-4-layer-model`, `sec-11-week-1-grpc-bridge-state-cache` |
| `hard kill` | Budget-enforcement action that terminates all relevant agents once a hard ceiling is reached. | `sec-04-02-session-model-config`, `sec-06-per-project-cost-enforcement` |
| `Hypervisor` | The daemon control plane and the name of the `hypervisor.proto v2` gRPC service. | `sec-02-system-architecture-4-layer-model`, `sec-03-04-hypervisor-proto-v2-contract` |
| `HypervisorEvent` | Streamed event envelope carrying state changes, telemetry, budget alerts, uncertainty flags, and slot swap events. | `sec-03-04-events-commands`, `sec-02-layer-1-rust-daemon-core-architecture` |
| `include directive` | YAML mechanism that lets a large multi-project session be split across files. | `sec-04-new-in-v2-e-004`, `sec-04-01-design-principles`, `sec-04-03-project-level-fields` |
| `Karpathy Alignment Matrix` | Appendix C mapping Karpathy's "agent command center" requests to concrete spec features and phases. | `app-c-karpathy-alignment-matrix` |
| `KillProject` | Operator command that stops all agents in one project. | `sec-03-04-events-commands` |
| `LanceDB` | Embedded vector search store used for code signatures and decision memory in the Shared Memory Bus and Scoped Context Compiler. | `sec-02-daemon-subsystem-breakdown`, `sec-03-03-shared-memory-bus`, `sec-12-step-2-lancedb-vector-memory-bus` |
| `Macro Kanban Board` | Full-screen task and dependency surface with project filtering and, later, cross-project swim lanes. | `sec-05-macro-kanban-board-task-queue` |
| `Merge Choreography` | Auxiliary UI surface for per-project merge queues, conflict risk visibility, and human approvals. | `sec-05-merge-choreography` |
| `Model Context Protocol (MCP)` | External protocol/server ecosystem listed as part of the substrate beneath Nexode. | `sec-02-system-architecture-4-layer-model`, `sec-04-02-projects-slots` |
| `mutation zone` | Appendix A directory-level write boundary used to prevent pooled agents from colliding in the same files. | `app-a-coordination-constraints`, `app-a-pool-persona-yaml-example` |
| `NexodeSession` | Top-level runtime container parsed from `session.yaml`; it owns projects, defaults, and budget for one daemon run. | `sec-03-01-domain-model-hierarchy`, `sec-08-week-1-foundation` |
| `nexode-ctl` | Simple Rust gRPC client used to inspect state and issue headless commands during Phase 1 validation. | `sec-09-weeks-3-4-grpc-state`, `sec-09-phase-1-exit-criteria` |
| `Observer` | Alternate name for the OrchestratorAgent when it is described as enforcing sandbox, tool, and semantic monitoring loops. | `sec-06-orchestration-autonomy-orchestratoragent`, `sec-06-observer-agent-three-monitoring-loops` |
| `OperatorCommand` | Command envelope used to send pause, resume, kill, move, assign, chat, mode, and project control actions to the daemon. | `sec-03-04-events-commands` |
| `OrchestratorAgent` | The central control-plane agent inside the daemon that manages slots, auto-dispatch, autonomy, uncertainty routing, and project-local coordination. | `sec-02-daemon-subsystem-breakdown`, `sec-06-orchestration-autonomy-orchestratoragent`, `sec-09-weeks-5-6-orchestration` |
| `per-project cost enforcement` | Budget logic that measures and acts on a single project's spend independently of the whole session. | `sec-06-per-project-cost-enforcement`, `sec-08-week-2-agent-lifecycle` |
| `portfolio view` | The unfocused TUI state that shows multiple projects before a single project is expanded. | `sec-10-interactive-controls` |
| `Predictive Conflict Routing` | Phase 4 merge-risk analysis that compares AST mutations to catch semantically dangerous merges Git may accept. | `sec-12-step-4-predictive-conflict-routing` |
| `Project` | A codebase or task collection within a session, typically mapped to one Git repository but allowed to be non-git. | `sec-03-01-domain-model-hierarchy`, `sec-04-02-projects-slots` |
| `Project Groups` | Default UI grouping mode in which agents are organized by project with color-coded headers and project-level cost display. | `sec-05-v2-changes-e-006`, `sec-05-synapse-telemetry-grid` |
| `ProjectBudgetAlert` | Event emitted when a project's warning or hard budget threshold is crossed. | `sec-03-04-hypervisor-proto-v2-contract`, `sec-03-04-events-commands`, `sec-06-per-project-cost-enforcement` |
| `provider_config` | YAML mapping from provider names to environment-backed credentials. | `sec-04-02-session-model-config` |
| `Rendering Shell` | Any Layer 3 client that renders daemon state, including the TUI and the VS Code extension pack. | `sec-02-system-architecture-4-layer-model`, `sec-02-key-architectural-property` |
| `Scoped Context Compiler` | Phase 4 compiler that assembles persona, structural context, and historical intent before dispatching work into a slot. | `sec-02-daemon-subsystem-breakdown`, `sec-03-02-fungible-workers-agentslot-abstraction`, `sec-12-step-3-scoped-context-compiler` |
| `Semantic Loop` | Observer monitoring loop that uses background LLM calls to detect persona drift or task misalignment. | `sec-06-observer-agent-three-monitoring-loops` |
| `semantic drift` | Condition where an agent's output stops matching the slot's task description or intended role. | `sec-06-observer-agent-three-monitoring-loops` |
| `Session Config Manager` | Daemon subsystem that parses `session.yaml v2`, handles defaults cascade and includes, merges `.nexode.yaml`, and supports v1 fallback. | `sec-02-daemon-subsystem-breakdown` |
| `Session State` | Ephemeral part of shared memory containing active tasks, assignments, and approvals, stored in SQLite and cleared at session end. | `sec-03-03-shared-memory-bus` |
| `session.yaml v2` | The multi-project session schema that defines the session, defaults, models, projects, budgets, and slots. | `sec-04-session-configuration-session-yaml-v2`, `sec-08-phase-0-spike-validate` |
| `Shared Memory Bus` | The memory architecture that keeps isolated agents aligned through Codebase Graph, Decision Log, and Session State. | `sec-03-03-shared-memory-bus` |
| `SlotAgentSwapped` | Event emitted when an existing slot gets a replacement agent because of crash recovery, manual reassignment, or initial assignment. | `sec-03-02-fungible-workers-agentslot-abstraction`, `sec-03-04-events-commands` |
| `soft alert` | Warning-level budget threshold that triggers an alert before the hard ceiling kills agents. | `sec-04-02-session-model-config`, `sec-06-per-project-cost-enforcement` |
| `Sprint Pool` | Appendix A example of a small agent pool assigned to one macro Kanban card. | `app-a-agent-pools-swarm-protocol` |
| `StateCache` | In-memory copy of `FullStateSnapshot` held by the VS Code integration layer. | `sec-11-week-1-grpc-bridge-state-cache` |
| `Static Enforcement Loop` | Observer monitoring loop that uses OS-level file watchers to enforce per-slot sandbox boundaries and mock terminal errors. | `sec-06-observer-agent-three-monitoring-loops` |
| `Status Bar HUD` | Footer status surface showing active count, token velocity, cost, project breakdown, and fatigue meter. | `sec-05-status-bar-hud`, `sec-11-weeks-5-6-native-vscode-integrations` |
| `Stop the Line` | Intervention rule that pauses work when repeated failure loops or anomalous behavior cross a threshold. | `sec-06-observer-agent-three-monitoring-loops`, `app-a-coordination-constraints` |
| `structural conflict risk` | Estimate that AST-level changes may break behavior even when Git can merge text successfully. | `sec-05-merge-choreography`, `sec-12-step-4-predictive-conflict-routing` |
| `structural moat` | Appendix B framing that open source distribution creates defensibility through trust, adoption, and ecosystem gravity. | `app-b-licensing-distribution-strategy` |
| `Substrate` | Layer 0 foundation: OS filesystem, Git internals, provider APIs, and MCP servers that exist independently of Nexode. | `sec-02-system-architecture-4-layer-model` |
| `Synapse Telemetry Grid` | Primary observability surface for live agent output, state, cost, token velocity, and one-click controls. | `sec-05-synapse-telemetry-grid`, `sec-11-weeks-2-4-multi-monitor-react-webviews` |
| `TaskNode` | DAG-tracked unit of work that may map 1:1 to a slot or be subdivided further. | `sec-03-01-domain-model-hierarchy`, `sec-03-04-entity-models` |
| `TaskStatus` | The protocol enum that describes task lifecycle states such as pending, working, review, done, paused, and archived. | `sec-03-04-service-enums` |
| `Tier 0 / Tier 0.5 / Tier 1 / Tier 2` | Appendix B licensing and distribution ladder spanning open daemon core, open TUI plugin system, freemium VS Code extension, and paid enterprise/cloud features. | `app-b-licensing-distribution-strategy`, `app-b-strategic-rationale` |
| `Token Accountant` | Daemon subsystem and SQL logging model that tracks cost at the agent, slot, project, and session levels. | `sec-02-daemon-subsystem-breakdown`, `sec-06-sqlite-schema-cost-tracking`, `sec-08-week-1-foundation` |
| `TUI` | Terminal-based Layer 3 renderer introduced in Phase 2 as an alternative to the VS Code shell. | `sec-02-system-architecture-4-layer-model`, `sec-10-phase-2-tui-command-center` |
| `uncertainty routing` | Orchestrator behavior that detects repeated loops or uncertainty, pauses or blocks the affected agent, and requests intervention. | `sec-06-uncertainty-routing-loop-detection`, `sec-09-weeks-5-6-orchestration` |
| `UncertaintyFlagTriggered` | Event emitted when uncertainty routing identifies a blocked or looping condition. | `sec-03-04-events-commands`, `sec-06-uncertainty-routing-loop-detection` |
| `Vector Memory Bus` | Vector-backed memory layer that stores structural and decision context for later retrieval by fungible workers. | `sec-02-daemon-subsystem-breakdown`, `sec-03-03-shared-memory-bus`, `sec-12-step-2-lancedb-vector-memory-bus` |
| `ViewContainerLocation.ChatBar` | Internal VS Code chat location targeted by the Deep Fork plan. | `sec-13-exploiting-sessions-layer` |
| `WAL` | Write-ahead log used for daemon crash recovery and session reattachment. | `sec-09-weeks-3-4-grpc-state`, `sec-10-session-persistence` |
| `Universal Command Chat` | Named VS Code chat surface that routes natural-language commands toward specific agents or slots through the orchestrator. | `sec-05-universal-command-chat` |
| `Workflow DAG Engine` | Daemon subsystem that manages Kanban columns, parses task slots, and tracks dependency/completion state. | `sec-02-daemon-subsystem-breakdown` |
| `Worktree` | Isolated Git worktree directory managed by the daemon for an active slot. | `sec-03-01-domain-model-hierarchy`, `sec-03-04-entity-models`, `sec-08-week-2-agent-lifecycle` |
| `Worktree Diff Viewer` | TUI pane mode that flips from stdout stream to inline git diff review. | `sec-10-interactive-controls` |
| `Worktree Router` | Phase 3 workflow that mounts a blocked agent's hidden worktree into the user's active VS Code workspace so the human can take over. | `sec-11-week-7-worktree-router` |

## 3. Overloaded or Ambiguous Terms

| Term | Distinct usages in the spec | Why the term is overloaded or ambiguous |
|---|---|---|
| `agent` | Ephemeral process, slot-bound worker identity, UI cell, and participant in pooled collaboration. | The spec uses one word for both raw compute and the contextualized worker the user sees. |
| `slot` | Stable work unit, assignment target, cost bucket, and chat-routing target by implication. | The formal command model is more agent-centric than slot-centric, but UX text talks about slot routing. |
| `task` | Natural-language slot task, `TaskNode` in the DAG, and Kanban card. | The spec says `TaskNode` may map 1:1 to a slot or be subdivided, so the boundary is intentionally loose. |
| `project` | Git-backed codebase, task collection, budget/accounting unit, and non-git research effort. | The spec explicitly allows projects with no repo, which broadens the term beyond codebase. |
| `session` | Daemon run, YAML portfolio definition, persisted state, and shared-memory lifetime. | Runtime, config, and persistence scopes are discussed under the same word. |
| `mode` | Autonomy tier (`manual`, `plan`, `full_auto`) and UI view mode (`Project Groups`, `Flat View`, `Focus View`). | The document uses "mode" for both control policy and visual presentation. |
| `review` | `TaskStatus.REVIEW`, `AgentState.REVIEW`, merge-approval queue language, and "needs approval" UI color semantics. | The exact entity that is "in review" is not always explicit. |
| `context` | File-glob scope, AST structural context, historical intent, system prompt payload, and repo-local defaults. | The word spans source selection, semantic retrieval, and prompt assembly. |
| `defaults` | Session defaults block, repo-local defaults block, and implicit auto-generated defaults such as branch naming. | The hierarchy is described in more places than it is fully formalized. |
| `pool` | Main-spec fungible worker pool and Appendix A micro-swarm collaboration pool. | One phrase describes interchangeable workers; another describes coordinated collaborator groups. |
| `command center` | Overall product framing, TUI surface, and VS Code surface family. | The spec sometimes uses it as the product concept and sometimes as a concrete UI surface. |
| `renderer` / `rendering shell` / `UI` | TUI, VS Code extension, future web dashboard, and multi-monitor webviews. | The document uses near-synonyms across layers and phases. |
| `merge` / `merged` | Merge choreography queue, merge review, merge conflict risk, and Kanban column state. | The status model for "merged" is not fully aligned with the proto status enum. |
| `Observer` / `OrchestratorAgent` | Same control-plane actor described from a monitoring perspective and a dispatch/control perspective. | The naming shift suggests one component, but the boundary is not restated each time. |
| `bus` | `gRPC Bridge`, `EventBus`, `Shared Memory Bus`, and `Vector Memory Bus`. | "Bus" names transport and memory mechanisms that operate at different layers. |

## 4. Explicit Contradictions and Specification Ambiguities

1. `TaskStatus` does not define `MERGED`, but the Macro Kanban Board lists `Merged` as a first-class column. The spec does not say whether `Merged` is UI-only, a missing enum value, or a synonym for `Done` or `Archived`.
   References: `sec-03-04-service-enums`, `sec-05-macro-kanban-board-task-queue`

2. `FullStateSnapshot` is defined with `projects`, `task_dag`, and session cost fields, but Phase 1 later says `FullStateSnapshot includes projects[], agent_slots[]`. The earlier message definition does not include `agent_slots[]`.
   References: `sec-03-04-entity-models`, `sec-09-weeks-3-4-grpc-state`

3. The YAML and autonomy sections use `manual | plan | full_auto`, while the protobuf enum uses `AGENT_MODE_NORMAL | PLAN | FULL_AUTO`. The mapping from `manual` to `NORMAL` is implied rather than stated.
   References: `sec-04-02-session-model-config`, `sec-06-autonomy-tiers`, `sec-03-04-service-enums`

4. Section 4.1 and Phase 0 say defaults cascade `session > project > slot`, but the `session.yaml v2` schema and field reference do not define a project-level `defaults` block. Project-local defaults appear in the `.nexode.yaml` example instead.
   References: `sec-04-01-design-principles`, `sec-04-02-session-model-config`, `sec-04-03-project-level-fields`, `sec-04-04-per-project-nexode-yaml`, `sec-08-week-1-foundation`

5. The UI section promises multi-monitor spatial orchestration in Phase 3, while the v2 change note says monitor assignment is deferred to Phase 4+. The document does not explicitly distinguish manual panel placement from automatic monitor assignment.
   References: `sec-05-ui-ux-surfaces-command-center`, `sec-05-v2-changes-e-006`, `sec-11-weeks-2-4-multi-monitor-react-webviews`

6. Universal Command Chat says users can route instructions to specific slots or agents, but the formal command contract is agent-centric except for free-form `ChatDispatch`. Slot-targeted command semantics are described in UX language but not formalized in the structured API.
   References: `sec-05-universal-command-chat`, `sec-03-04-events-commands`

7. Merge Choreography refers to worktrees in `REVIEW` state, but the formal state types define `REVIEW` for `AgentState` and `TaskStatus`, not for `Worktree`. The spec leaves the review-bearing entity implicit.
   References: `sec-05-merge-choreography`, `sec-03-04-service-enums`, `sec-03-04-entity-models`

8. Phase 0 kill criteria say to stop if "the merge step consistently produces broken code," but Phase 0 deliverables focus on parsing, lifecycle, budgeting, and crash recovery. The merge step is not otherwise defined as a Phase 0 deliverable.
   References: `sec-08-week-2-agent-lifecycle`, `sec-08-kill-criteria`
