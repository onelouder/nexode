# Observer Subsystem Design

> **Status:** PROPOSED
> **Date:** 2026-03-15
> **Author:** pc
> **Addresses:** Sprint 3 (Observer Loops + Safety), R-005 (event sequence gaps), R-009 (semantic drift)
> **Spec refs:** `sec-06-autonomy-tiers`, `sec-09-phase-1-headless-orchestrator`

---

## 1. Problem Statement

Once agents run unattended (full-auto mode), the daemon needs safety mechanisms to detect and intervene when agents:

1. **Loop** — produce identical output patterns or consume tokens without making worktree progress
2. **Escape sandbox** — write files outside their assigned worktree
3. **Stall** — stop producing output without exiting
4. **Signal uncertainty** — emit markers indicating they need human guidance

Sprint 3 delivers the concrete implementations for these four cases. This document provides the architectural context and design rationale for the `observer.rs` module that Codex will build, plus forward-looking design for Phase 4+ Observer capabilities (coherence modeling, LLM-backed evaluation).

---

## 2. Sprint 3 Scope: What to Build Now

### 2.1 Module Structure

A single new file: `crates/nexode-daemon/src/observer.rs`

Contains:
- `LoopDetector` — tracks per-slot output patterns and progress signals
- `SandboxGuard` — validates file paths against worktree boundaries
- `ObserverAlert` — event type for detected problems
- Configuration structs integrated into `DaemonConfig`

### 2.2 LoopDetector

```
Engine tick (2s)
    │
    ▼
loop_detector.check(slot_id)
    │
    ├── check_repeated_outputs(slot_id)
    │     └── sliding window of last N output hashes per slot
    │         if 3+ identical consecutive hashes → alert
    │
    ├── check_stuck_timeout(slot_id)
    │     └── compare now() - last_worktree_change_at
    │         if > stuck_timeout_seconds → alert
    │
    └── check_budget_velocity(slot_id)
          └── if tokens_consumed > 50% budget AND worktree_changes == 0 → alert
```

**Key design decisions for Codex:**

- The `LoopDetector` is NOT a separate task/thread. It's a struct called synchronously from the engine's tick handler. This keeps the control flow simple and avoids shared mutable state.
- Output patterns should be hashed (e.g., `blake3` or even a simple `std::hash`) rather than stored verbatim, to keep memory bounded.
- The sliding window size should be configurable but defaulting to the last 10 outputs per slot.
- `check_stuck_timeout` needs access to the worktree's last `git diff` state. The simplest approach is to cache the last `git diff --stat` hash per slot and update it on each tick (or less frequently — every 5th tick to avoid I/O pressure).

### 2.3 SandboxGuard

```
Agent spawn
    │
    ▼
sandbox_guard.register(slot_id, canonical_worktree_path)
    │
    ▼
During execution: monitor output lines for path patterns
    │
    ▼
Post-completion: git diff --name-only
    │
    ├── All paths under worktree root → allow merge
    └── Any path escapes → block merge, emit ObserverAlert
```

**Key design decisions for Codex:**

- `canonical_worktree_path` must be resolved via `std::fs::canonicalize()` BEFORE agent spawn, not after. Symlink races are a real attack surface.
- Output monitoring during execution is best-effort (advisory). The authoritative check is the post-completion `git diff --name-only` — this catches anything that actually landed on disk.
- The guard should normalize paths before comparison: resolve `../`, strip trailing slashes, handle case sensitivity per platform.

### 2.4 ObserverAlert Event

```rust
// Proposed shape — Codex has design freedom here
enum ObserverAlert {
    LoopDetected {
        slot_id: String,
        detection_type: LoopType, // RepeatedOutput | StuckTimeout | BudgetVelocity
        details: String,
    },
    SandboxViolation {
        slot_id: String,
        offending_path: String,
        worktree_root: String,
    },
    UncertaintySignal {
        slot_id: String,
        agent_id: String,
        reason: String,          // The matched uncertainty marker text
    },
}

enum LoopType {
    RepeatedOutput,
    StuckTimeout,
    BudgetVelocity,
}
```

### 2.5 Uncertainty Routing

Parse agent stdout for these markers (from AGENTS.md convention):
- `"DECISION:"` — agent is asking the operator to make a call
- `"I'm not sure"` / `"I need clarification"` — hedging language
- `"BLOCKED:"` — explicit block signal

On detection: transition slot to `PAUSED`, emit `ObserverAlert::UncertaintySignal`.

The operator resumes via `nexode-ctl dispatch resume-slot <slot-id> --instruction "..."`.

### 2.6 Configuration

```yaml
# In DaemonConfig
loop_detection:
  enabled: true
  max_identical_outputs: 3    # consecutive identical output hashes
  stuck_timeout_seconds: 300  # 5 minutes with no worktree diff change
  budget_velocity_threshold: 0.5  # >50% tokens with 0 changes
  on_loop: alert              # alert | kill | pause

sandbox_enforcement: true
```

---

## 3. Phase 4+ Extensions: Coherence Monitoring (Future Scope)

> **This section is a design reference, not Sprint 3 scope.** It documents concepts from the master spec's Phase 4 (Smart Context and Semantic Memory) that will eventually extend the Observer. Sprint 3 should NOT implement any of this — but the `observer.rs` module structure should not preclude it.

### 3.1 Concept: LLM-Based Coherence Evaluation

Beyond heuristic loop detection, a mature Observer would periodically assess whether an agent's output semantically aligns with its task directive. This requires:

- **An evaluation LLM** (cheap, fast model like claude-haiku or gpt-4o-mini) that scores agent output against the task
- **Composite coherence signals**: velocity health, output diversity, task alignment, state freshness, semantic coherence, cost efficiency
- **An intervention policy** with configurable authority (advisory-only vs. autonomous pause/kill)

### 3.2 Concept: Sawtooth Coherence Model

See **D-011** in `DECISIONS.md` for the formal specification. The key insight is that agent coherence degrades between context checkpoints following an exponential decay weighted by swarm state drift, and is only partially restored at each checkpoint (gist compression is lossy). The model predicts optimal checkpoint frequency.

### 3.3 Concept: Relevance-Filtered Briefings

When an agent receives a checkpoint briefing (context reset), only shared observations whose relevance tags overlap with the agent's task domain should be included. This is a `BriefingFilter` concept that reduces token waste. Implementation depends on the Shared Memory Bus (Phase 4).

### 3.4 Design Implication for Sprint 3

The `observer.rs` module should be structured so that the `LoopDetector` and `SandboxGuard` are separate structs, not monolithic. Future phases will add an `EvaluationEngine` struct alongside them. Suggested module layout:

```
observer.rs (Sprint 3)
├── LoopDetector
├── SandboxGuard
└── ObserverAlert

observer.rs (Phase 4+ extension)
├── LoopDetector       (unchanged)
├── SandboxGuard       (unchanged)
├── EvaluationEngine   (new — LLM-backed coherence scoring)
├── CoherenceTracker   (new — P(S,t) sawtooth model)
└── ObserverAlert      (extended with EvaluationComplete variant)
```

This is advisory, not prescriptive. Codex should optimize for Sprint 3 clarity.

---

## 4. Integration Points

### 4.1 Engine Loop

The Observer is invoked from the engine's tick handler:

```
tick_interval.tick() => {
    orchestrator.evaluate_slots_and_dispatch(&event_tx).await;
    orchestrator.run_observer_checks(&event_tx).await;  // ← Sprint 3
}
```

### 4.2 Event Stream

`ObserverAlert` events flow through the existing `broadcast::channel` to connected clients. Sprint 3 Part 3 adds sequence numbers to this stream (R-005), which the Observer benefits from automatically.

### 4.3 Proto Changes

Sprint 3 needs proto additions:
- `ObserverAlert` message (or extend existing `UncertaintyFlagTriggered`)
- `event_sequence: uint64` in `HypervisorEvent` (Part 3)
- `last_event_sequence: uint64` in `FullStateSnapshot` (Part 3)

### 4.4 nexode-ctl

- `nexode-ctl watch` prints warnings on sequence gaps
- `nexode-ctl dispatch resume-slot <slot-id>` supports `--instruction` flag

---

## 5. Open Questions for Codex

These are design choices Codex should make during implementation and document in HANDOFF.md:

1. Should `LoopDetector` output hashing use a fast hash (FxHash) or a content-addressable hash (blake3)? FxHash is faster but has higher collision risk.
2. Should `SandboxGuard` path monitoring during execution be opt-in (disabled by default) or always-on? The post-completion check is authoritative regardless.
3. Should uncertainty marker parsing be regex-based or simple string containment? The set of markers is small and known.
4. For event sequence numbers, should the counter live on `HypervisorEvent` proto or in a wrapper `EventEnvelope`? The wrapper is cleaner but requires more proto surgery.
