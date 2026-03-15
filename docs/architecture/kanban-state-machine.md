# Kanban State Machine Architecture

> **Status:** PROPOSED
> **Date:** 2026-03-14
> **Author:** jwells + pc
> **Supersedes:** D-001 (TASK_STATUS_MERGED)
> **Spec refs:** `sec-03-04-service-enums`, `sec-05-macro-kanban-board-task-queue`, `sec-05-merge-choreography`, `sec-06-orchestration-autonomy-orchestratoragent`, `sec-12-step-4-predictive-conflict-routing`

---

## 1. Problem Statement

The spec's `TaskStatus` enum conflates an agent's execution state with a task's integration state. The Kanban Board lists "Merged" as a column, but `TaskStatus` has no corresponding enum value. D-001 proposed a simple `TASK_STATUS_MERGED = 7`, but this is insufficient for safe multi-agent orchestration.

When 10-15 autonomous agents operate in isolated Git worktrees, merging their output back into a shared codebase is a serialization bottleneck that introduces race conditions and potential main-branch corruption. The Kanban board requires a decoupled state machine that isolates worktree execution from codebase integration.

### The Fundamental Distinction

The question "Did the agent do the right thing?" (semantic correctness → **REVIEW**) is categorically different from "Does this code mathematically integrate with the current state of the repo?" (structural integration → **MERGE_QUEUE / RESOLVING**). These two concerns must not be collapsed into a single transition.

---

## 2. Proposed TaskStatus Enum

```protobuf
enum TaskStatus {
  TASK_STATUS_UNSPECIFIED = 0;
  TASK_STATUS_PENDING     = 1;  // Backlog. No worktree provisioned.
  TASK_STATUS_WORKING     = 2;  // Agent assigned. Worktree active.
  TASK_STATUS_REVIEW      = 3;  // Execution complete. Awaiting semantic approval.
  TASK_STATUS_MERGE_QUEUE = 4;  // Semantic approval granted. Awaiting structural integration.
  TASK_STATUS_RESOLVING   = 5;  // Blocked by AST/Git conflict. Human intervention required.
  TASK_STATUS_DONE        = 6;  // Merged to target branch. Worktree GC'd.
  TASK_STATUS_PAUSED      = 7;  // Operator halted via /pause.
  TASK_STATUS_ARCHIVED    = 8;  // Task killed or discarded.
}
```

### Changes from Spec Baseline

| Spec baseline | This proposal | Rationale |
|---|---|---|
| No `MERGED` state | `MERGE_QUEUE` (value 4) | Serialization buffer for concurrent merges |
| — | `RESOLVING` (value 5) | Explicit conflict resolution state |
| `DONE` = value 4 | `DONE` = value 6 | Renumbered. DONE now strictly means "code is in main, worktree GC'd" |
| `PAUSED` = value 5 | `PAUSED` = value 7 | Renumbered |
| `ARCHIVED` = value 6 | `ARCHIVED` = value 8 | Renumbered |

### Changes from D-001

D-001 proposed `TASK_STATUS_MERGED = 7` as a state between DONE and ARCHIVED. This proposal **supersedes** D-001 by replacing that single state with two states (`MERGE_QUEUE`, `RESOLVING`) that model the integration pipeline correctly. The "merged" concept is absorbed into DONE: a task is DONE when (and only when) its code is committed to the target branch and the worktree is dismantled.

---

## 3. State Transition Table

| Kanban Column | Entry Condition | Exit Condition | Worktree Status |
|---|---|---|---|
| **PENDING** | Task defined in session.yaml or UI | Orchestrator assigns AgentSlot | None |
| **WORKING** | AgentProcess spawned | Agent process exits successfully or task completed | Active, isolated branch |
| **REVIEW** | Agent outputs terminal completion signal | Human operator or Phase 4 Observer Agent approves semantics | Isolated. Read-only lock |
| **MERGE_QUEUE** | REVIEW approved via gRPC `MoveTask` command | Orchestrator initiates Git rebase/merge operation | Isolated. Queued for main |
| **RESOLVING** | Predictive conflict routing detects high structural conflict risk, or Git merge fails | Human operator resolves conflict in native VS Code diff editor and approves | Target branch checked out, merge markers present |
| **DONE** | Code successfully committed to target branch | N/A (terminal state) | Garbage collected |
| **PAUSED** | Operator issues `/pause` command | Operator issues `/resume` command | Frozen (worktree preserved) |
| **ARCHIVED** | Task killed or discarded | N/A (terminal state) | Garbage collected |

### State Transition Diagram

```
                    ┌──────────┐
                    │ PENDING  │
                    └────┬─────┘
                         │ Orchestrator assigns slot
                         ▼
                    ┌──────────┐
              ┌─────│ WORKING  │──────────┐
              │     └────┬─────┘          │
              │          │ Agent completes │ /pause
              │          ▼                 ▼
              │     ┌──────────┐     ┌──────────┐
              │     │  REVIEW  │     │  PAUSED  │
              │     └────┬─────┘     └────┬─────┘
              │          │ Approved        │ /resume
              │          ▼                 │
              │     ┌────────────┐         │
              │     │MERGE_QUEUE │◄────────┘
              │     └────┬───┬───┘
              │          │   │
              │    Clean │   │ Conflict detected
              │    merge │   │
              │          │   ▼
              │          │  ┌───────────┐
              │          │  │ RESOLVING │
              │          │  └─────┬─────┘
              │          │        │ Human resolves
              │          │        │
              │          ▼        ▼
              │     ┌──────────────┐
              │     │     DONE     │
              │     │ (worktree GC)│
              │     └──────────────┘
              │
              │ Kill / discard
              ▼
         ┌──────────┐
         │ ARCHIVED │
         └──────────┘
```

### Valid Transitions (Exhaustive)

| From | To | Trigger |
|---|---|---|
| PENDING | WORKING | Orchestrator auto-dispatch or manual `AssignTask` |
| PENDING | ARCHIVED | Operator kills unstarted task |
| WORKING | REVIEW | Agent signals completion |
| WORKING | PAUSED | Operator `/pause` |
| WORKING | ARCHIVED | Operator kills or budget exceeded |
| REVIEW | MERGE_QUEUE | Semantic approval granted |
| REVIEW | WORKING | Reviewer rejects, sends back for rework |
| REVIEW | PAUSED | Operator `/pause` |
| MERGE_QUEUE | DONE | Clean fast-forward or three-way merge succeeds |
| MERGE_QUEUE | RESOLVING | Git conflict or AST conflict risk above threshold |
| RESOLVING | DONE | Human resolves conflict and approves merge |
| RESOLVING | ARCHIVED | Human decides to abandon the task |
| PAUSED | WORKING | Operator `/resume` (if was WORKING) |
| PAUSED | MERGE_QUEUE | Operator `/resume` (if was queued) |

### Invalid Transitions (Guards)

- DONE → anything (terminal)
- ARCHIVED → anything (terminal)
- PENDING → REVIEW (cannot skip execution)
- WORKING → MERGE_QUEUE (cannot skip review — see autonomy tier override below)
- RESOLVING → MERGE_QUEUE (resolved conflicts go to DONE, not back to queue)

---

## 4. System Dynamics and Implementation Rules

### 4.1 The Role of MERGE_QUEUE

In a multi-agent system, multiple agents may finish tasks on the same project simultaneously. If they all transition from REVIEW directly to a merge attempt, immediate locking collisions occur. **MERGE_QUEUE acts as a serialization buffer.** The Orchestrator daemon pulls tasks from this queue one at a time per project, rebases them against the latest target branch, and attempts a fast-forward merge.

Implementation requirements:
- The Orchestrator maintains a per-project FIFO merge queue
- Only one merge operation per project may be in-flight at any time
- After a successful merge, the next item in the queue must rebase against the now-updated target branch before its own merge attempt
- Queue ordering is configurable: FIFO (default), priority-weighted, or dependency-ordered

### 4.2 The Definition of DONE

DONE strictly means the lifecycle is complete: the code is in the main codebase and the isolated worktree has been dismantled by the Git Worktree Orchestrator. If a task is merely written but not integrated, it is **not DONE**.

This is a breaking change from D-001, which placed "MERGED" between DONE and ARCHIVED. Under this proposal, DONE absorbs the merged concept — there is no state where code is merged but the task is somehow not done.

### 4.3 Autonomy Tier Overrides

| Mode | Behavior at REVIEW boundary |
|---|---|
| `manual` | Hard-stop at REVIEW. Human must approve. |
| `plan` | Hard-stop at REVIEW. Human must approve. |
| `full_auto` | Bypass REVIEW → push directly to MERGE_QUEUE. The Orchestrator automatically attempts the merge. If the Phase 4 AST parser detects a signature collision, the system kicks the task to RESOLVING and pulses the UI orange, preserving safety even under full automation. |

### 4.4 RESOLVING and Phase 4 Dependency

> **CRITICAL DEPENDENCY:** The RESOLVING state has two trigger paths with different phase availability.
>
> - **Phase 0-3 (available immediately):** Git merge failure triggers RESOLVING. The daemon attempts `git merge`, catches the failure, and transitions the task. This is basic and testable in Phase 0.
> - **Phase 4+ (deferred):** Predictive Conflict Routing (`sec-12-step-4-predictive-conflict-routing`, `REQ-P4-007`) uses AST mutation comparison to flag RESOLVING **before** Git attempts the merge. This is the proactive path — catching semantically dangerous merges that Git would silently accept (e.g., two agents modify different files but introduce incompatible type signatures).
>
> Phase 0-3 implementations MUST support RESOLVING via Git-level conflict detection. Phase 4 adds the AST-level proactive trigger. The state machine is the same in both phases; only the trigger sophistication changes.

---

## 5. Barrier Synchronization for Merge Transitions

### 5.1 The Synchronization Race Condition

When the Orchestrator executes a clean fast-forward merge, the transition sequence (`MERGE_QUEUE → DONE → Worktree GC'd`) occurs in single-digit milliseconds inside the Rust daemon. If the daemon streams these as isolated gRPC events, the Extension Host — a single-threaded Node.js process — will interleave these updates with telemetry streams from 14 other active agents. The React Kanban board will attempt to render a task in DONE while simultaneously trying to fetch path metrics for a Worktree that has already been destroyed, causing a null reference in the UI renderer.

### 5.2 Implementation: The Barrier Vector

The solution relies on the `barrier_id` string in the `HypervisorEvent` message. A barrier acts as a transactional boundary for UI state.

#### Rust Daemon (Layer 1): Payload Grouping

The Core Engine wraps the sequence of merge-related events in a single barrier before transmitting over the gRPC Unix socket.

```rust
// Rust Daemon Core Engine
pub async fn execute_fast_forward_merge(
    &mut self,
    slot_id: &str,
    event_tx: &broadcast::Sender<HypervisorEvent>,
) {
    let barrier_id = uuid::Uuid::new_v4().to_string();
    let timestamp_ms = current_time_ms();

    // 1. Transition Task Status
    let _ = event_tx.send(HypervisorEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        timestamp_ms,
        barrier_id: barrier_id.clone(),
        payload: Some(Payload::TaskStatusChanged(TaskStatusChanged {
            task_id: slot_id.to_string(),
            new_status: TaskStatus::Done,
            agent_id: self.get_agent_for_slot(slot_id),
        })),
    });

    // 2. Teardown Worktree
    self.worktree_orchestrator.garbage_collect(slot_id).await;
    let _ = event_tx.send(HypervisorEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        timestamp_ms,
        barrier_id, // Identical barrier_id binds the events
        payload: Some(Payload::WorktreeStatusChanged(WorktreeStatusChanged {
            worktree_id: slot_id.to_string(),
            new_risk: 0.0, // Indicates deletion/resolution
        })),
    });
}
```

#### Extension Host (Layer 3): Buffer & Flush

The TypeScript EventBus caches incoming events by `barrier_id` in a `Map<string, HypervisorEvent[]>`. It uses `queueMicrotask` to schedule the flush, ensuring all events received in the current event loop tick are batched and sent via a single `postMessage` to all React Webviews simultaneously.

```typescript
// VS Code Extension Host (EventBus)
private barrierCache = new Map<string, nexode.hypervisor.v2.HypervisorEvent[]>();

public onGrpcEventReceived(event: nexode.hypervisor.v2.HypervisorEvent) {
    if (event.barrierId) {
        if (!this.barrierCache.has(event.barrierId)) {
            this.barrierCache.set(event.barrierId, []);
            // Defer dispatch until the end of the current microtask queue
            queueMicrotask(() => this.flushBarrier(event.barrierId));
        }
        this.barrierCache.get(event.barrierId)!.push(event);
    } else {
        // Unbarriered events (e.g., standard telemetry) pass through immediately
        this.dispatchToWebviews([event]);
    }
}

private flushBarrier(barrierId: string) {
    const events = this.barrierCache.get(barrierId);
    if (events && events.length > 0) {
        this.dispatchToWebviews(events); // Single postMessage containing the array
    }
    this.barrierCache.delete(barrierId);
}
```

### 5.3 Risk Assessment

| Scenario | Probability | Mitigation |
|---|---|---|
| UI tearing without barriers during concurrent multi-agent merges | >95% | Barrier synchronization (this document) |
| Extension Host IPC choking at 15 agents × 1,000 tok/sec | ~15% | Optimized `useRef` rendering for terminal streams; Phase 5 Deep Fork contingency remains on roadmap to bypass Extension Host serialization |

---

## 6. Kanban Column Mapping

The Kanban Board columns map 1:1 to `TaskStatus` enum values. No UI-only columns exist.

| Kanban Column | TaskStatus Value | Color | Icon |
|---|---|---|---|
| Backlog | `PENDING` | Gray | ○ |
| Working | `WORKING` | Teal | ◉ |
| Review | `REVIEW` | Amber | ◎ |
| Merge Queue | `MERGE_QUEUE` | Blue | ⟳ |
| Resolving | `RESOLVING` | Red | ⚠ |
| Done | `DONE` | Green | ✓ |
| Paused | `PAUSED` | Gray | ‖ |
| Archived | `ARCHIVED` | Dim | ✗ |

---

## 7. Phase Dependencies

| Capability | Phase | Notes |
|---|---|---|
| TaskStatus enum with MERGE_QUEUE and RESOLVING | Phase 0 | Proto schema. Testable immediately. |
| Per-project FIFO merge queue in Orchestrator | Phase 1 | Core serialization logic. |
| RESOLVING via Git merge failure | Phase 0 | Basic conflict detection. |
| RESOLVING via Predictive Conflict Routing (AST) | Phase 4 | Depends on `REQ-P4-007`, `REQ-P4-008`. Proactive detection before Git attempts merge. |
| Barrier synchronization in EventBus | Phase 3 | Required when React Webviews exist. TUI can apply barriers directly. |
| Autonomy tier bypass (full_auto → MERGE_QUEUE) | Phase 1 | Tied to autonomy tier enforcement. |

---

## 8. Impact on Existing Decisions

| Decision | Impact |
|---|---|
| **D-001** | **SUPERSEDED** by D-009. `TASK_STATUS_MERGED` is replaced by `MERGE_QUEUE` + `RESOLVING`. The "merged" concept is absorbed into `DONE`. |
| **D-007** | **Reinforced.** REVIEW still belongs to TaskNode. The Merge Choreography TreeView now also shows `MERGE_QUEUE` and `RESOLVING` tasks via the same task→slot→worktree join path. |
| **D-008** | **Reinforced.** Phase 0 "merge step" now maps precisely to the `MERGE_QUEUE → DONE` transition using programmatic `git merge`. RESOLVING via Git failure is also testable in Phase 0. |

---

## 9. Open Questions

1. **Queue ordering policy:** Should the per-project merge queue be strict FIFO, or should the Orchestrator support priority-weighted ordering (e.g., critical-path tasks merge first)? Recommend: FIFO default with optional `priority` field on TaskNode for future override.
2. **RESOLVING timeout:** Should there be an automatic escalation if a task sits in RESOLVING for >N hours without human action? Recommend: configurable per-project, default 24h, escalation emits `UncertaintyFlagTriggered`.
3. **Cross-project merge coordination:** When two projects share dependencies (monorepo or linked repos), should the Orchestrator coordinate merge ordering across projects? Recommend: defer to Phase 3+ per `sec-06-orchestration-autonomy-orchestratoragent` ("per-project intelligence, no cross-project until Phase 3+").
