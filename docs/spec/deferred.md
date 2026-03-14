# Deferred Requirements

> Requirements explicitly excluded from Phase 0–2 scope.
> Each entry preserves its original requirement ID and spec reference for traceability.
> Items move out of this file when their target phase begins decomposition.

---

## Phase 4: Smart Context and Semantic Memory

| Req ID | Spec Section | Summary | Target Phase | Rationale for Deferral |
|---|---|---|---|---|
| `REQ-ARCH-015` | `sec-02-daemon-subsystem-breakdown` | Scoped Context Compiler queries Vector Memory Bus for role-specific prompts | Phase 4 | Depends on AST indexing and LanceDB infrastructure |
| `REQ-DOM-009` | `sec-03-02-fungible-workers-agentslot-abstraction` | Agent gains identity via compiled context slice at slot assignment | Phase 4 | Requires Scoped Context Compiler |
| `REQ-DOM-012` | `sec-03-03-shared-memory-bus` | Shared Memory Bus: three-layer architecture (Codebase Graph, Decision Log, Session State) | Phase 4 | Full memory bus depends on vector DB and AST pipeline |
| `REQ-DOM-013` | `sec-03-03-shared-memory-bus` | Codebase Graph stores AST node embeddings, dependency relationships, import maps | Phase 4 | Requires Tree-sitter integration and embedding pipeline |
| `REQ-DOM-014` | `sec-03-03-shared-memory-bus` | Codebase Graph reduces context window bloat by 20-40% | Phase 4 | Performance target gated on Phase 4 infrastructure |
| `REQ-UX-005` | `sec-05-v2-changes-e-006` | Automatic monitor assignment (daemon-driven) | Phase 4+ | Manual drag works in Phase 3; automatic placement is research-risk (see D-005) |
| `REQ-UX-020` | `sec-05-merge-choreography` | Structural conflict risk scores upgraded to AST-based scoring | Phase 4 | Depends on Codebase Graph and Predictive Conflict Routing |
| `REQ-ORCH-014` | `sec-06-observer-agent-three-monitoring-loops` | Semantic Loop: background LLM calls to detect drift | Phase 4 | Requires vector memory for comparison baseline |
| `REQ-P4-001` | `sec-12-phase-4-smart-context-semantic-memory` | Transition to AST-aware semantic context injection | Phase 4 | Core Phase 4 capability |
| `REQ-P4-002` | `sec-12-phase-4-smart-context-semantic-memory` | 20-40% token reduction target | Phase 4 | Measurement requires full context compiler |
| `REQ-P4-003` | `sec-12-step-1-real-time-ast-indexing` | Full-codebase AST parse on init, incremental updates via file watchers | Phase 4 | Tree-sitter integration |
| `REQ-P4-004` | `sec-12-step-1-real-time-ast-indexing` | Signature extraction strips function bodies, retains structural signatures | Phase 4 | Part of AST indexing pipeline |
| `REQ-P4-005` | `sec-12-step-2-lancedb-vector-memory-bus` | Embed AST signatures and agent decisions into LanceDB | Phase 4 | Depends on embedding model selection and LanceDB integration |
| `REQ-P4-006` | `sec-12-step-3-scoped-context-compiler` | Dispatch compiles role persona + AST context + Decision Log history | Phase 4 | Three-input compiler is the Phase 4 deliverable |
| `REQ-P4-007` | `sec-12-step-4-predictive-conflict-routing` | AST mutation comparison before merging worktrees | Phase 4 | Requires AST diff infrastructure. **Linked to D-009/D-010:** upgrades RESOLVING trigger from Git-level to AST-level. |
| `REQ-P4-008` | `sec-12-step-4-predictive-conflict-routing` | High structural conflict risk turns merge node red in UI | Phase 4 | UI indicator depends on AST-based risk scoring. **Linked to D-009/D-010:** populates `Worktree.conflict_risk` with real AST scores. |
| `REQ-P4-009` | `sec-12-phase-4-success-kill-criteria` | Success: 20%+ token reduction, Decision Log sufficiency, no latency regression | Phase 4 | Exit criteria for Phase 4 |
| `REQ-P4-010` | `sec-12-phase-4-success-kill-criteria` | Kill: offload vector DB if it causes runtime stutter | Phase 4 | Mitigation path for Phase 4 |

## Phase 5: Deep Fork (Conditional)

| Req ID | Spec Section | Summary | Target Phase | Rationale for Deferral |
|---|---|---|---|---|
| `REQ-P5-001` | `sec-13-phase-5-deep-fork-conditional` | Phase 5 triggers only if Phase 3 extension approach fails | Phase 5 | Conditional on Phase 3 failure |
| `REQ-P5-002` | `sec-13-architectural-shift-bypassing-extension-host` | Move HypervisorClient/EventBus into VS Code core | Phase 5 | Requires VS Code fork |
| `REQ-P5-003` | `sec-13-ui-overhaul-native-dom` | Replace React Webviews with native DOM primitives (60fps target) | Phase 5 | Requires VS Code fork |
| `REQ-P5-004` | `sec-13-exploiting-sessions-layer` | AgenticParts: SynapseGridPart, MacroKanbanPart, ChatBar takeover | Phase 5 | Requires VS Code fork |
| `REQ-P5-005` | `sec-13-native-worktree-scm-isolation` | Explorer distinguishes Human Main from Agent Sandboxes | Phase 5 | Requires VS Code fork |
| `REQ-P5-006` | `sec-13-maintenance-strategy` | Maintenance: minimal core mutation, patch tracking, monthly rebases | Phase 5 | Fork maintenance strategy |

## Phase 3+: Pool Architecture

| Req ID | Spec Section | Summary | Target Phase | Rationale for Deferral |
|---|---|---|---|---|
| `REQ-PLAN-003` | `sec-07-v2-scope-change-e-007` | Agent Pools, .swarm/, mutation zones removed from Phase 0-1 | Phase 3+ | Explicitly deferred by E-007 |
| `REQ-POOL-001` | `app-a-agent-pools-swarm-protocol` | Pool architecture deferred from MVP | Phase 3+ | Appendix A is non-MVP |
| `REQ-POOL-002` | `app-a-agent-pools-swarm-protocol` | Sprint Pool: 1-4 agents per Macro Kanban card | Phase 3+ | Depends on pool infrastructure |
| `REQ-POOL-003` | `app-a-pool-structure-role-based-micro-swarms` | Pool roles: Builder, Tester, Reviewer/Documenter | Phase 3+ | Depends on pool infrastructure |
| `REQ-POOL-004` | `app-a-swarm-file-based-message-bus` | .swarm/ file-based message bus for pool coordination | Phase 3+ | Depends on pool infrastructure |
| `REQ-POOL-005` | `app-a-coordination-constraints` | Mutation zones: directory-level write locks per pool role | Phase 3+ | Depends on pool infrastructure |
| `REQ-POOL-006` | `app-a-coordination-constraints` | Pool sequencing: Tester blocked until Builder commits | Phase 3+ | Depends on pool infrastructure |
| `REQ-POOL-007` | `app-a-coordination-constraints` | Pool-wide stop on Builder/Tester loop | Phase 3+ | Depends on pool infrastructure |
