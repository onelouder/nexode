# Phase 0 Sprint: Spike and Validate

> **Agent:** gpt (OpenAI Codex)
> **Branch:** `agent/gpt/phase-0-spike`
> **Duration:** 2 weeks (target)
> **Spec version:** v2.0.1-locked
> **Prerequisite reads:** `AGENTS.md`, `DECISIONS.md`, `docs/spec/master-spec.md` (Sections 2, 3, 4, 8), `docs/architecture/kanban-state-machine.md`

---

## Objective

Validate the core Nexode architecture: a Rust daemon that parses session.yaml, spawns agent CLI processes into isolated Git worktrees, tracks cost, and merges work back into a target branch. This is a spike — build the minimum to prove the approach works or kill it.

## What to Build

### Week 1: Foundation

#### 1. Cargo Workspace Setup
```
nexode/
├── Cargo.toml              # workspace root
├── crates/
│   ├── nexode-daemon/      # main binary
│   ├── nexode-proto/       # protobuf definitions + generated code
│   └── nexode-ctl/         # simple CLI client for testing
```

#### 2. Proto File: `hypervisor.proto v2`
Implement the proto schema from `sec-03-04-hypervisor-proto-v2-contract` in the spec, with the following amendments from accepted decisions:

**D-009 (ACCEPTED):** Use this TaskStatus enum, not the one in the spec:
```protobuf
enum TaskStatus {
  TASK_STATUS_UNSPECIFIED = 0;
  TASK_STATUS_PENDING     = 1;
  TASK_STATUS_WORKING     = 2;
  TASK_STATUS_REVIEW      = 3;
  TASK_STATUS_MERGE_QUEUE = 4;  // D-009: serialization buffer
  TASK_STATUS_RESOLVING   = 5;  // D-009: conflict state
  TASK_STATUS_DONE        = 6;  // D-009: renumbered
  TASK_STATUS_PAUSED      = 7;  // D-009: renumbered
  TASK_STATUS_ARCHIVED    = 8;  // D-009: renumbered
}
```

**D-003 (ACCEPTED):** `AGENT_MODE_NORMAL` maps to YAML `manual`. The Session Config Manager translates at parse time.

**D-006 (ACCEPTED):** Add `SlotDispatch` to the `OperatorCommand` oneof:
```protobuf
message SlotDispatch { string slot_id = 1; string raw_nl = 2; }
```
Add it as arm 10 in the `OperatorCommand.action` oneof.

Use `tonic` for gRPC, `prost` for code generation.

#### 3. session.yaml v2 Parser
Implement the `Session Config Manager` using `serde` with strict validation.

**D-004 (ACCEPTED):** The cascade is: session.defaults → projects[].defaults → .nexode.yaml → slots[].{field}. Use type-aware merging:
- Scalars (`model`, `mode`, `timeout`): deep override, most specific wins
- Arrays (`tags`, `context.include`, `context.exclude`): unique union merge, combine and deduplicate
- Maps (`provider_config`): shallow merge by key
- Empty array `[]` is an explicit clear, not a no-op

**D-004 also adds** an optional `defaults` block at the project level inside session.yaml (same shape as session-level defaults).

Must handle:
- v2 multi-project schema
- `include` directives (split session across files)
- v1 backward compat (no `projects` key → wrap in single implicit project)
- `.nexode.yaml` repo-local merge

Write extensive tests for the merge logic — especially array union behavior for `context.exclude`.

#### 4. Token Accountant (SQLite)
Implement the SQL schema from `sec-06-token-accounting-schema`. Append-only log with `project_id` and `slot_id` columns. Support per-project budget enforcement:
- `warn_usd` threshold → emit `ProjectBudgetAlert` (soft)
- `max_usd` threshold → emit `ProjectBudgetAlert` (hard) + kill agents

### Week 2: Agent Lifecycle

#### 5. Agent Process Manager
- Spawn CLI agent processes using `tokio::process`
- Monitor stdout/stderr, parse output, stream telemetry
- Watchdog timeout (configurable per-slot via `timeout_minutes`)
- Crash detection → spawn replacement into same slot → emit `SlotAgentSwapped`
- For Phase 0, the "agent" can be a mock: a shell script that writes to files in the worktree and exits

#### 6. Git Worktree Orchestrator
- Given a session with N projects across M repos, create isolated git worktrees for each active slot
- Branch naming: `agent/{slot-id}` (default) or slot-level `branch` field override
- Worktree lifecycle: create on slot assignment, GC on task DONE
- **D-008 (ACCEPTED):** After merging a worktree branch back to the target, run post-merge verification:
  - Execute project-level build command (if configured)
  - Execute project-level test command (if configured)
  - A merge is only "successful" if git merge AND build/test pass
  - Add a `verify` field at project level: `verify: { build: "cargo build", test: "cargo test" }`

#### 7. Per-Project Merge Queue
- **D-009 (ACCEPTED):** Implement a per-project FIFO merge queue
- Only one merge operation in-flight per project at any time
- After successful merge, next queued item must rebase against updated target before its own merge attempt
- On git conflict → transition task to `TASK_STATUS_RESOLVING`
- On clean merge + verification pass → transition to `TASK_STATUS_DONE`, GC worktree

#### 8. Core Engine Loop
The `tokio::select!` loop from the conceptual skeleton in `sec-02-conceptual-rust-skeleton`:
- Receive `OperatorCommand` from gRPC → dispatch
- Tick interval (2s) → evaluate slots, run observer checks
- Agent output → update telemetry, check for completion signals

#### 9. nexode-ctl
Simple Rust gRPC client that can:
- `nexode-ctl status` → call `GetFullState`, pretty-print
- `nexode-ctl dispatch <command>` → send `OperatorCommand`
- `nexode-ctl watch` → subscribe to event stream, print events

## Exit Criteria (Kill/Continue)

The spike succeeds if ALL of the following pass:

1. **Parse:** session.yaml v2 with 3 projects, 5 slots, include directives, and .nexode.yaml merges correctly. Array union merge for context.exclude works.
2. **Lifecycle:** Daemon spawns mock agents into worktrees, detects completion, detects crash, respawns into same slot.
3. **Budget:** Token accountant tracks cost. Soft alert fires at warn threshold. Hard kill fires at max threshold.
4. **Merge:** 3 mock agents on the same project complete tasks. Merge queue serializes them one at a time. Post-merge build/test runs. At least 5 consecutive merge-then-verify cycles succeed.
5. **Crash recovery:** Daemon recovers from agent crash without losing slot state or worktree.
6. **nexode-ctl:** Can inspect state and issue commands via gRPC.

**Kill criterion:** If worktree isolation consistently produces broken merges (build failures after git-clean merges), or if spawning/monitoring agent processes across OS environments is unreliable, we stop.

## What NOT to Build

- No UI (TUI or VS Code extension) — that's Phase 2/3
- No Scoped Context Compiler or vector memory — that's Phase 4
- No AST indexing or Predictive Conflict Routing — that's Phase 4 (D-010)
- No Agent Pools or .swarm/ protocol — that's Phase 3+ (deferred, see `docs/spec/deferred.md`)
- No autonomy tier enforcement beyond basic mode field — that's Phase 1
- No real LLM agent integration — mock agents are sufficient for the spike

## Key Decisions to Respect

| Decision | Summary | Impact on Phase 0 |
|---|---|---|
| D-002 | No top-level `agent_slots` in FullStateSnapshot | Access slots via `projects[].slots[]` |
| D-003 | YAML `manual` ↔ proto `AGENT_MODE_NORMAL` | Session parser translation table |
| D-004 + D-004a | Project-level defaults + array union merge | Core parser feature. Test thoroughly. |
| D-006 | `SlotDispatch` command | Add to proto oneof |
| D-007 | REVIEW belongs to TaskNode, not Worktree | Merge Choreography queries task status |
| D-008 + D-008a | Post-merge build/test verification | `verify` field on project. Run after every merge. |
| D-009 | MERGE_QUEUE + RESOLVING in TaskStatus | Proto enum + merge queue logic |
| D-010 | RESOLVING = Git conflict only in Phase 0 | No AST analysis needed yet |

## Git Conventions

- Branch: `agent/gpt/phase-0-spike`
- Commit messages: `[gpt] type: description`
- Types: `feat`, `fix`, `test`, `refactor`, `chore`, `docs`
- PR to `main` when spike is complete
- Do not modify: `AGENTS.md`, `DECISIONS.md`, `docs/spec/*`, `docs/architecture/*` (those are pc's domain)

## References

- Spec: `docs/spec/master-spec.md` — Sections 2 (Architecture), 3 (Domain Types), 4 (session.yaml), 8 (Phase 0)
- Outline with stable IDs: `docs/spec/spec-outline.md`
- Requirements: `docs/spec/requirements-extracted.md`
- Kanban architecture: `docs/architecture/kanban-state-machine.md`
- Deferred scope: `docs/spec/deferred.md`
- All accepted decisions: `DECISIONS.md`
