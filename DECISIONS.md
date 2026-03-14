# DECISIONS.md — Architecture & Design Decisions

> Numbered decisions for cross-agent reference. Append-only.
> Reference in commits and handoffs as `D-NNN`.

## Format

```
## D-NNN: Title
- **Date:** YYYY-MM-DD
- **By:** agent(s) involved
- **Status:** PROPOSED | ACCEPTED | SUPERSEDED by D-NNN
- **Context:** Why this decision was needed
- **Decision:** What we decided
- **Rationale:** Why this option over alternatives
- **Consequences:** What changes as a result
```

---

## D-001: TaskStatus shall include TASK_STATUS_MERGED

- **Date:** 2026-03-14
- **By:** pc
- **Status:** SUPERSEDED by D-009
- **Context:** The `TaskStatus` enum (`sec-03-04-service-enums`) defines `PENDING`, `WORKING`, `REVIEW`, `DONE`, `PAUSED`, and `ARCHIVED` — but no `MERGED`. The Macro Kanban Board (`sec-05-macro-kanban-board-task-queue`) lists `Merged` as a first-class column. The spec never clarifies whether `Merged` is UI-only, a missing enum value, or a synonym for `Done`.
- **Decision:** Add `TASK_STATUS_MERGED = 7` to the `TaskStatus` enum in `hypervisor.proto v2`. `Merged` is a distinct lifecycle state between `DONE` and `ARCHIVED`: the task's worktree has been successfully merged into the target branch. The Kanban column maps 1:1 to this enum value.
- **Rationale:** Treating `Merged` as UI-only would force Layer 3 renderers to infer merge state from worktree metadata, violating the principle that the daemon is the single source of truth for task lifecycle. A dedicated enum value keeps the Kanban columns isomorphic with the protocol.
- **Consequences:** Proto schema gains one enum value. The daemon's merge choreography flow transitions a `TaskNode` from `REVIEW → MERGED` on successful worktree merge, and from `MERGED → ARCHIVED` on GC. All Kanban renderers (TUI and VS Code) map the column directly.

---

## D-002: FullStateSnapshot shall include top-level agent_slots

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** The proto definition of `FullStateSnapshot` (`sec-03-04-entity-models`) contains `repeated Project projects`, `repeated TaskNode task_dag`, and session cost fields. Phase 1 (`sec-09-weeks-3-4-grpc-state`) later says "FullStateSnapshot includes projects[], agent_slots[]." The `agent_slots[]` field is not present in the proto definition. However, `Project` already contains `repeated AgentSlot slots`.
- **Decision:** Do NOT add a top-level `agent_slots` field to `FullStateSnapshot`. The Phase 1 text's "agent_slots[]" is a shorthand reference to the slots nested inside each `Project`. The canonical access path is `FullStateSnapshot.projects[].slots[]`. If a flat list is needed for UI rendering (e.g., Flat View mode), the client constructs it by iterating projects.
- **Rationale:** Adding a redundant top-level list creates a normalization hazard: the daemon would need to keep two representations in sync. The nested structure already provides full slot access. The Phase 1 text was a prose summary, not a schema amendment.
- **Consequences:** The `FullStateSnapshot` proto remains as defined in `sec-03-04-entity-models`. Phase 1 implementation docs and any references to "top-level agent_slots" shall be read as "projects[].slots[]". Layer 3 renderers that need a flat agent list derive it client-side.

---

## D-003: YAML mode "manual" maps to proto AGENT_MODE_NORMAL

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** The YAML schema and autonomy section (`sec-04-02-session-model-config`, `sec-06-autonomy-tiers`) use `manual | plan | full_auto`. The proto enum (`sec-03-04-service-enums`) uses `AGENT_MODE_NORMAL | AGENT_MODE_PLAN | AGENT_MODE_FULL_AUTO`. The mapping from `manual` to `NORMAL` is implied but never stated.
- **Decision:** The canonical mapping is: YAML `manual` ↔ proto `AGENT_MODE_NORMAL` (value 1), YAML `plan` ↔ proto `AGENT_MODE_PLAN` (value 2), YAML `full_auto` ↔ proto `AGENT_MODE_FULL_AUTO` (value 3). The Session Config Manager shall perform this translation at parse time. Proto-facing code always uses `AGENT_MODE_*` names; user-facing YAML and UI always display `manual | plan | full_auto`.
- **Rationale:** The proto name `NORMAL` is an implementation artifact (proto enums conventionally avoid domain-specific terms like `manual`). Renaming the proto value would be gratuitous churn; the session parser is the natural boundary for name translation.
- **Consequences:** Session Config Manager implementation must include a validated mapping table. Any future autonomy tiers must be added to both the YAML schema and the proto enum with an explicit mapping entry in this decision or a superseding one.

---

## D-004: Project-level defaults block shall be added to session.yaml v2

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** Section 4.1 and Phase 0 (`sec-04-01-design-principles`, `sec-08-week-1-foundation`) state the defaults cascade is `session > project > slot`. The `session.yaml v2` schema (`sec-04-02-complete-schema-annotations`) defines a session-level `defaults` block but not a project-level one. Project-local defaults appear only in the `.nexode.yaml` example (`sec-04-04-per-project-nexode-yaml`), which is repo-local, not session-level.
- **Decision:** Add an optional `defaults` block at project level inside `session.yaml v2`, with the same shape as the session-level defaults (model, mode, timeout, provider). The cascade becomes: session defaults → project defaults (in session.yaml) → `.nexode.yaml` repo-local overrides → slot-level fields. When a project entry in session.yaml has a `defaults` block, those values override session defaults for all slots in that project. `.nexode.yaml` overrides are then merged on top.
- **Rationale:** Without a project-level defaults block in the session YAML, the only way to set per-project defaults is through `.nexode.yaml` in each repo. This breaks for non-git projects and forces the human to maintain defaults outside the session file. Adding the block fulfills the documented cascade promise.
- **Consequences:** The YAML schema field reference (`sec-04-03-project-level-fields`) gains a `defaults` entry. Session Config Manager merge order becomes: session.defaults → projects[].defaults → .nexode.yaml → slots[].{field}. A spec amendment note shall be appended to the normalized spec.

---

## D-005: Multi-monitor: manual placement in Phase 3, automatic assignment deferred to Phase 4+

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** The UI section (`sec-05-ui-ux-surfaces-command-center`) promises "multi-monitor spatial orchestration" in Phase 3. The v2 change note (`sec-05-v2-changes-e-006`) says "Monitor assignment is deferred to Phase 4+." `REQ-UX-012` explicitly states maximized grid mode is "draggable to a second monitor." These are not contradictory if "manual drag" and "automatic assignment" are distinguished, but the spec never states this distinction.
- **Decision:** Phase 3 shall support manual multi-monitor use: the Synapse Grid `WebviewPanel` can be popped out and dragged to any monitor by the operator using standard VS Code panel mechanics. No daemon logic, session.yaml configuration, or automatic monitor routing is required in Phase 3. Automatic monitor assignment (daemon-driven placement of panels to specific monitors based on session config) is Phase 4+ scope.
- **Rationale:** Manual drag is a zero-cost consequence of using `WebviewPanel` (VS Code already supports it). Automatic assignment requires daemon awareness of display topology and VS Code extension APIs for programmatic panel placement, which is research-risk work better deferred.
- **Consequences:** Phase 3 requirements and exit criteria do not include automatic monitor assignment. Phase 4 requirements shall include `REQ-UX-005` (monitor assignment). No schema changes needed.

---

## D-006: Slot-targeted commands shall be formalized as SlotDispatch

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** Universal Command Chat (`sec-05-universal-command-chat`) describes users routing instructions "to specific slots or agents." The formal command contract (`sec-03-04-events-commands`) is agent-centric: `PauseAgent`, `ResumeAgent`, `ChatDispatch(raw_nl)`. The free-form `ChatDispatch` could carry slot-targeted intent, but the structured command set has no slot-addressed variant.
- **Decision:** Add a `SlotDispatch` command message to the `OperatorCommand` oneof: `message SlotDispatch { string slot_id = 1; string raw_nl = 2; }`. This is the structured counterpart to `ChatDispatch` that routes to a specific slot rather than requiring the OrchestratorAgent to parse natural language for intent routing. `ChatDispatch` remains for unrouted/ambient commands. The OrchestratorAgent resolves the slot's current agent and forwards.
- **Rationale:** Without a structured slot-targeted command, the only path from the UI to a specific slot is through natural-language parsing in `ChatDispatch`, which is fragile and non-deterministic. Slot IDs are stable; agent IDs are ephemeral. The UX text's promise of "route to a specific slot" deserves a first-class command.
- **Consequences:** Proto gains one message and one oneof arm. Phase 1 implementation includes `SlotDispatch` alongside `ChatDispatch`. UI chat input can provide a structured `/slot <slot-id> <instruction>` prefix that maps directly to `SlotDispatch`.

---

## D-007: Merge Choreography REVIEW state belongs to TaskNode, surfaced via Worktree

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** Merge Choreography (`sec-05-merge-choreography`) describes "worktrees in REVIEW state." The formal state model defines `REVIEW` for `AgentState` (the process) and `TaskStatus` (the task), but not for `Worktree` (which only carries `id`, `path`, `branch`, and `conflict_risk`). Phase 3 text (`sec-11-weeks-2-4-multi-monitor-react-webviews`) says "TreeView showing worktrees in REVIEW state."
- **Decision:** A worktree is "in REVIEW" when its owning `TaskNode` has `TaskStatus = TASK_STATUS_REVIEW`. The Worktree message does not gain a status field. The Merge Choreography UI queries the task DAG for tasks in REVIEW status and joins on `Worktree` via the task's assigned slot's `worktree_id` to display branch, diff, and conflict risk. The prose "worktrees in REVIEW" is shorthand for "worktrees whose task is in review."
- **Decision rule:** If a future need arises for worktree-specific lifecycle states independent of task status (e.g., worktree-level GC state), a `WorktreeStatus` enum may be added in a superseding decision.
- **Rationale:** Adding a `status` field to `Worktree` would create a second source of truth for review state. The join path (task → slot → worktree) is already implicit in the data model. Keeping the Worktree message lean preserves its role as a filesystem/git artifact, not a workflow entity.
- **Consequences:** Merge Choreography implementation queries `TaskNode` status, not `Worktree` status. Phase 3 TreeView data provider filters by `TASK_STATUS_REVIEW` and resolves worktree metadata through the slot association.

---

## D-008: Phase 0 kill criteria "merge step" refers to worktree merge validation, not full Merge Choreography

- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Context:** Phase 0 kill criteria (`sec-08-kill-criteria`) say to stop "if the merge step consistently produces broken code." Phase 0 deliverables (`sec-08-week-1-foundation`, `sec-08-week-2-agent-lifecycle`) cover session parsing, lifecycle, budgeting, multi-repo worktree creation, and crash recovery — but do not list Merge Choreography or a UI merge queue.
- **Decision:** In Phase 0 context, "the merge step" means the daemon's programmatic `git merge` of a slot's worktree branch back into the project's main branch after the task is marked `DONE`. This is a basic git operation in the Agent Process Manager / Git Worktree Orchestrator, not the Phase 3 Merge Choreography UI. The kill criterion tests whether isolated worktree branches can be cleanly merged after agent work completes. If automated merges consistently break (compile failures, test regressions), the worktree isolation strategy is invalid and the project should stop.
- **Rationale:** Phase 0 is a 2-week spike. The kill criterion must be testable within that scope. Programmatic `git merge` + test verification is achievable. Full Merge Choreography with UI, conflict-risk scoring, and human approvals is Phase 3 scope.
- **Consequences:** Phase 0 exit criteria include: "automated worktree merge succeeds without manual conflict resolution for at least N test tasks." Phase 0 does NOT require Merge Choreography UI, structural conflict scoring, or human approval queues.

---

## D-009: Kanban state machine with MERGE_QUEUE and RESOLVING (supersedes D-001)

- **Date:** 2026-03-14
- **By:** jwells + pc
- **Status:** PROPOSED
- **Context:** D-001 proposed adding a single `TASK_STATUS_MERGED = 7` to bridge the gap between the spec's Kanban "Merged" column and the proto enum. However, this treats merge as an atomic event rather than a pipeline. When 10-15 agents finish tasks on the same project simultaneously, direct `REVIEW → merge` transitions cause locking collisions. The spec conflates agent execution state with task integration state. A proper resolution must decouple "Did the agent do the right thing?" (REVIEW) from "Does this code integrate with the current repo state?" (merge pipeline).
- **Decision:** Replace `TASK_STATUS_MERGED` with two new states: `TASK_STATUS_MERGE_QUEUE = 4` (semantic approval granted, awaiting structural integration) and `TASK_STATUS_RESOLVING = 5` (blocked by AST/Git conflict, human intervention required). `DONE` (renumbered to 6) now strictly means "code committed to target branch, worktree GC'd." The "merged" concept is absorbed into DONE — there is no state where code is merged but the task is not done. Full schema, state transitions, barrier synchronization, and autonomy tier overrides are specified in `docs/architecture/kanban-state-machine.md`.
- **Rationale:** MERGE_QUEUE acts as a per-project serialization buffer: the Orchestrator pulls tasks from the queue one at a time, rebases against the latest target branch, and attempts a fast-forward merge. This prevents race conditions and main-branch corruption. RESOLVING makes conflict state explicit rather than hiding it inside a generic "blocked" status. Together they give the Kanban board 1:1 isomorphism with the proto enum — no UI-only columns.
- **Consequences:** (1) `TaskStatus` enum renumbered: MERGE_QUEUE=4, RESOLVING=5, DONE=6, PAUSED=7, ARCHIVED=8. (2) Orchestrator must implement a per-project FIFO merge queue. (3) Barrier synchronization required for `MERGE_QUEUE → DONE → Worktree GC` transitions to prevent UI tearing. (4) Autonomy tier override: `full_auto` bypasses REVIEW and pushes directly to MERGE_QUEUE. (5) D-001 is superseded.

---

## D-010: RESOLVING state requires Phase 4 Predictive Conflict Routing for full capability

- **Date:** 2026-03-14
- **By:** jwells + pc
- **Status:** PROPOSED
- **Context:** D-009 introduces `TASK_STATUS_RESOLVING` with two trigger paths of different sophistication. The basic trigger (Git merge failure) is available immediately. The advanced trigger (AST mutation comparison that catches semantically dangerous merges Git would silently accept) depends on Phase 4 infrastructure: Tree-sitter AST indexing (`REQ-P4-003`, `REQ-P4-004`), LanceDB vector storage (`REQ-P4-005`), and Predictive Conflict Routing (`REQ-P4-007`, `REQ-P4-008`).
- **Decision:** The `RESOLVING` state and its associated Kanban column SHALL be implemented in Phase 0 with Git-level conflict detection as the sole trigger. In Phase 4, the trigger is upgraded: before the Orchestrator attempts `git merge`, the Predictive Conflict Routing subsystem compares AST mutations across the worktree branch and the target branch. If structural conflict risk exceeds a configurable threshold, the task transitions to `RESOLVING` proactively — before Git sees it. This catches the dangerous case where Git can merge text cleanly but the result has incompatible type signatures, broken imports, or semantic regressions.
- **Rationale:** The state machine must be stable from Phase 0 onward — agents and UI should never need to learn a new workflow. Only the trigger sophistication improves across phases. Deferring the state itself to Phase 4 would leave Phase 0-3 without a conflict resolution workflow.
- **Consequences:** (1) Phase 0-3: RESOLVING triggered only by `git merge` failure. Adequate for textual conflicts. (2) Phase 4+: RESOLVING additionally triggered by AST conflict risk scoring before merge attempt. (3) The `conflict_risk` field on the `Worktree` message gains operational significance in Phase 4 when the AST parser populates it with real scores. (4) Deferred requirements `REQ-P4-007` and `REQ-P4-008` in `docs/spec/deferred.md` are formally linked to D-009/D-010 as Phase 4 upgrade dependencies.
