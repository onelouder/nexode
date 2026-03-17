# Issues & Risks

> Tracked findings from code reviews and architecture sessions.  
> Updated by: pc  
> Format: `I-NNN` for issues (concrete bugs/gaps), `R-NNN` for risks (things that may bite us later).

---

## Open Issues

### ~~I-001: `rusqlite::Connection` is `!Send` — blocks async engine loop integration~~ RESOLVED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `accounting.rs`
- **Severity:** Medium
- **Resolved:** 2026-03-14, commit `93da7894`
- **Resolution:** New `TokenAccountingHandle` wraps the accountant in a dedicated `std::thread` actor with `mpsc::channel` for request/response. `blocking_recv()` on the actor thread, async callers use `oneshot`.

### ~~I-002: No timeout on verification commands in `git.rs`~~ RESOLVED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `git.rs` → `run_shell_step()`
- **Severity:** Medium
- **Resolved:** 2026-03-14, commit `93da7894`
- **Resolution:** Added `wait-timeout` crate. `run_shell_step` now spawns a child with piped output, calls `wait_timeout()`, kills on expiry, returns `VerificationTimedOut` error. Test verifies timeout prevents target branch advancement.

### ~~I-003: Synchronous git operations in async context~~ RESOLVED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `git.rs`
- **Severity:** Medium
- **Resolved:** 2026-03-14, commit `93da7894`
- **Resolution:** `engine.rs` wraps all git operations in `tokio::task::spawn_blocking()` — both worktree creation and merge-and-verify. `GitWorktreeOrchestrator` gained `Clone` derive to support moves into blocking closures.

### I-004: `provider_config` shallow merge not implemented

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `session.rs`
- **Severity:** Low
- **Details:** D-004 specifies "Maps (`provider_config`): shallow merge by key." The session parser does not currently parse or merge a `provider_config` field. Likely intentionally deferred since no agent providers are wired up, but should be tracked.
- **When:** When agent provider configuration is needed (Phase 1+).

### I-005: SQLite schema has no migration versioning

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `accounting.rs`
- **Severity:** Low
- **Details:** `CREATE TABLE IF NOT EXISTS` won't alter existing tables if the schema evolves. No version tracking or migration system.
- **When:** Before any schema changes in later phases.

### ~~I-006: Merge queue and engine loop not yet implemented~~ RESOLVED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `engine.rs` (new, 1346 lines)
- **Severity:** High
- **Resolved:** 2026-03-14, commit `93da7894`
- **Resolution:** Full engine loop with `tokio::select!`, per-project FIFO merge queue, mock agent spawning, telemetry recording, budget hard-kill, and nexode-ctl CLI (308 lines, clap-based). Integration tests prove 3-slot full-auto merge pipeline and budget-triggered archival.

---

## Open Risks

### R-001: Verification worktree cleanup on panic

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `git.rs`
- **Likelihood:** Low
- **Impact:** Low (orphaned worktrees waste disk)
- **Details:** If the daemon panics between creating a verification worktree and cleanup, orphaned worktrees accumulate in `.nexode-worktrees/`. Consider a startup sweep or `Drop` guard.
- **Mitigation:** Add a `prune_stale_verify_worktrees()` call at daemon startup.

### R-002: `sh -lc` in verification loads user dotfiles

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `git.rs` → `run_shell_step()`
- **Likelihood:** Medium
- **Impact:** Medium (non-deterministic build environments)
- **Details:** Verification commands run via `sh -lc` which loads `~/.bash_profile` etc. This ensures tools like `cargo` are on PATH, but user dotfiles could introduce environment variability.
- **Mitigation:** Document this choice. Consider a `clean_env` option or explicit PATH injection later.

### R-003: Telemetry parsing format is undocumented

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `process.rs` → `ParsedTelemetry::parse()`
- **Likelihood:** Medium
- **Impact:** Low (silent telemetry drops, not data corruption)
- **Details:** The parser expects exactly `TOKENS in=X out=Y cost=Z`. Case-sensitive prefix match, space-delimited key=value pairs. Not documented as the canonical wire format. When real agents are integrated, their output must match this exact format or telemetry is silently ignored.
- **Mitigation:** Document the format in AGENTS.md or a dedicated telemetry spec. Consider a more robust parser (JSON lines, structured logging).
- **Update (Sprint 1):** Parser now supports two formats: `TOKENS in=X out=Y cost=Z` (legacy) and `NEXODE_TELEMETRY:tokens_in=X,tokens_out=X,cost_usd=X` (new). Harness-specific `parse_keyed_telemetry` also handles free-form `key=value` pairs. Still undocumented.

### ~~R-004: Global `AtomicU64` agent IDs not unique across restarts~~ ADDRESSED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `process.rs`
- **Likelihood:** Low
- **Impact:** Low (agent ID collisions in logs after restart)
- **Details:** `AGENT_COUNTER` resets to 1 on daemon restart. Agent IDs like `slot-a-agent-1` will repeat across daemon lifetimes.
- **Mitigation:** Prefix with daemon instance ID (PID, UUID, or epoch timestamp) when moving to production.
- **Addressed:** Sprint 1 (2026-03-15). `daemon_instance_id` (UUID v4 prefix) is now prepended to agent IDs via `next_agent_id(prefix, slot_id)`. Format: `{instance_short}-{slot_id}-agent-{counter}`.

### ~~R-005: Broadcast stream drops lagged events silently~~ RESOLVED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `transport.rs`
- **Likelihood:** Medium (under load)
- **Impact:** Medium (UI misses state transitions)
- **Resolved:** Sprint 3 (2026-03-15), branch `agent/gpt/sprint-3-observer-safety`
- **Resolution:** Event sequence numbers added to `HypervisorEvent` and `FullStateSnapshot`. `BroadcastStream` now surfaces `RecvError::Lagged` as gRPC `DATA_LOSS` instead of silently filtering. `nexode-ctl watch` detects sequence gaps and refreshes via `GetFullState`. End-to-end gap recovery verified in tests.

### R-006: `edition = "2024"` pins MSRV to Rust 1.85+

- **Source:** Phase 0 review (2026-03-14)
- **Module:** All crates
- **Likelihood:** Low
- **Impact:** Low (limits contributor compatibility)
- **Details:** All three crates use `edition = "2024"`, which requires Rust 1.85+. This is the latest stable edition. If contributors or CI use older toolchains, they'll hit build failures.
- **Mitigation:** Document MSRV in README.md or Cargo.toml `[package]` metadata. Intentional choice — just needs to be explicit.

### ~~R-007: `CommandResponse` is fire-and-forget~~ FIXED

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `transport.rs`
- **Likelihood:** High (will matter in Sprint 1)
- **Impact:** Medium (no feedback on command execution)
- **Fixed:** Sprint 2 (2026-03-15), branch `agent/gpt/sprint-2-real-agents`
- **Resolution:** Full command acknowledgment via `oneshot::channel()`. Proto adds `command_id` and `CommandOutcome` enum. Engine validates slot existence and state transitions, returns `Executed`, `InvalidTransition`, or `SlotNotFound` through the oneshot. CLI prints formatted result. 5s timeout in production, 50ms in tests. Four tests cover the round-trip, timeout, and all outcome variants.

### ~~I-007: Merge queue drains on tick only (2s delay)~~ RESOLVED

- **Source:** Phase 0 review v2 (2026-03-14)
- **Module:** `engine.rs`
- **Severity:** Low
- **Resolved:** Sprint 6 (2026-03-15), branch `agent/gpt/sprint-6-integration-polish`
- **Resolution:** Merge queue draining now happens at enqueue call sites instead of waiting for the next engine tick. `move_task_to_merge_queue_drains_immediately` verifies that a task moved to `MERGE_QUEUE` reaches `DONE` in the same command path.

### ~~I-008: Daemon `main.rs` uses manual arg parsing instead of `clap`~~ RESOLVED

- **Source:** Phase 0 review v2 (2026-03-14)
- **Module:** `crates/nexode-daemon/src/main.rs`
- **Severity:** Low
- **Resolved:** Sprint 4 (2026-03-15), branch `agent/gpt/sprint-4-engine-hardening`
- **Resolution:** Daemon now uses `clap` derive macros matching `nexode-ctl` conventions. Supports `--session`, `--port`, positional session path, `--help`, and `--version`. Three CLI tests added.

### ~~I-009: `completion_detected` overrides non-zero exit as success~~ FIXED

- **Source:** Sprint 1 review (2026-03-15), finding F-001
- **Module:** `process.rs:325`
- **Severity:** Medium
- **Fixed:** Sprint 2 (2026-03-15), branch `agent/gpt/sprint-2-real-agents`
- **Resolution:** Success now requires `status.success() && (completion_detected || !requires_completion_signal)`. Four unit tests validate the full truth table: non-zero exit + marker = failure, zero exit + no marker + required = failure, zero exit + marker = success, zero exit + no marker + not required = success.

### ~~I-010: `AgentStateChanged(Executing)` dropped after swap~~ FIXED

- **Source:** Sprint 1 review (2026-03-15), finding F-003
- **Module:** `engine.rs` (SlotAgentSwapped handler)
- **Severity:** Medium
- **Fixed:** Sprint 2 (2026-03-15), branch `agent/gpt/sprint-2-real-agents`
- **Resolution:** `AgentStateChanged { agent_id: swapped.new_agent_id, new_state: Executing }` is now emitted immediately after `SlotAgentSwapped`. Test `slot_agent_swapped_emits_executing_event` validates the event stream.

### I-011: Recovery re-enqueues merge slot without worktree check

- **Source:** Sprint 1 review (2026-03-15), finding F-004
- **Module:** `recovery.rs:107-111`
- **Severity:** Low
- **Details:** If the daemon crashes mid-merge (after worktree cleanup but before the WAL write), recovery re-enqueues the slot at the front of the merge queue even though its worktree no longer exists. The merge will then fail at runtime.
- **When:** When merge reliability matters.

### I-012: Token/byte conflation in `truncate_payload`

- **Source:** Sprint 1 review (2026-03-15), finding F-005
- **Module:** `context.rs:77-96`
- **Severity:** Low
- **Details:** `max_context_tokens` from HarnessConfig is passed as a byte count to `truncate_payload`. Tokens ≠ bytes (~4 bytes/token). Currently `max_context_tokens` is always `None`, so not exercised.
- **When:** When `max_context_tokens` is actually used.

### I-013: Empty telemetry from malformed `TOKENS` lines

- **Source:** Sprint 1 review (2026-03-15), finding F-008
- **Module:** `process.rs:396-414`
- **Severity:** Low
- **Details:** `parse_space_delimited` returns `Some(ParsedTelemetry { all None })` for lines starting with `TOKENS ` that have no valid key=value pairs. Results in WAL entries with all-zero telemetry.
- **When:** Low urgency — only affects the legacy `TOKENS` prefix format.

### ~~I-014: Architecture doc CLI flags out of date~~ RESOLVED

- **Source:** Sprint 1 review (2026-03-15), finding F-009
- **Module:** `docs/architecture/agent-harness.md`
- **Severity:** Low
- **Resolved:** Sprint 6 (2026-03-15), branch `agent/gpt/sprint-6-integration-polish`
- **Resolution:** The harness architecture doc now matches the implementation: Claude uses `--verbose --output-format stream-json`, and Codex uses `codex exec --full-auto --json` with optional `--model`.

### ~~I-015: JSON substring matching in completion detection~~ FIXED

- **Source:** Sprint 1 review (2026-03-15), finding F-010
- **Module:** `harness.rs:177-178`
- **Severity:** Low
- **Fixed:** Sprint 2 (2026-03-15), branch `agent/gpt/sprint-2-real-agents`
- **Resolution:** Now uses `serde_json::from_str` + `json_field_is()` for proper JSON parsing. Test verifies `"task completed successfully"` no longer triggers false positives, while valid JSON result objects are correctly detected.

### ~~I-016: `is_valid_task_transition` diverges from Kanban State Machine spec~~ RESOLVED

- **Source:** Sprint 2 review (2026-03-15), finding F-001
- **Module:** `engine/commands.rs`, `is_valid_task_transition()`
- **Severity:** Medium
- **Resolved:** Sprint 4 (2026-03-15), branch `agent/gpt/sprint-4-engine-hardening`
- **Resolution:** `is_valid_task_transition` now takes a third argument `pre_pause_status: Option<TaskStatus>`. `MergeQueue → Paused` removed. `Paused → Working` only valid if `pre_pause_status == Some(Working)`. `Paused → MergeQueue` only valid if `pre_pause_status == Some(MergeQueue)`. Four unit tests cover the truth table. Note: `pre_pause_status` is runtime-only (not WAL-persisted) due to bincode backward-safety constraints.

### ~~I-017: `AgentStateChanged` proto missing `slot_id` field~~ RESOLVED

- **Source:** Sprint 2 review (2026-03-15), finding F-002
- **Module:** `nexode.proto`, `AgentStateChanged` message
- **Severity:** Low
- **Resolved:** Sprint 3 (2026-03-15), branch `agent/gpt/sprint-3-observer-safety`
- **Resolution:** `slot_id` field (field 3) added to `AgentStateChanged` proto message. All engine event emissions now populate it.

### I-018: `parse_json_summary_telemetry` could double-count on multiple result lines

- **Source:** Sprint 2 review (2026-03-15), finding F-003
- **Module:** `harness.rs`, `parse_json_summary_telemetry()`
- **Severity:** Low
- **Details:** The function fires on lines where `type == "result"`, `event == "done"`, or `status == "completed"`. If a CLI ever emits multiple result-type lines, each matching line with usage fields produces a `ParsedTelemetry`, and the engine's `apply_telemetry` increments cumulative totals — causing double-counting. In practice, Claude emits exactly one `type: "result"` line, so current risk is low.
- **When:** When adding support for new CLI agents or if Claude/Codex output format changes.

### I-019: `demo.sh` doesn't wait for DONE after MoveTask

- **Source:** Sprint 2 review (2026-03-15), finding F-005
- **Module:** `scripts/demo.sh`
- **Severity:** Low
- **Details:** After sending `dispatch move-task slot-a merge-queue`, the script immediately prints status and exits. The merge happens asynchronously on the daemon's tick interval (2s), so "Final status" may still show `merge_queue` instead of `done`.
- **When:** Minor cosmetic. Consider adding a wait loop for DONE after the MoveTask dispatch.

### I-020: `observe_output` creates slot state for unknown/removed slots

- **Source:** Sprint 3 review (2026-03-15), finding F-001
- **Module:** `observer.rs:118`
- **Severity:** Low
- **Details:** `observe_output()` calls `self.slots.entry(slot_id).or_default()` unconditionally, creating a `SlotLoopState` even if `observe_status()` was never called for that slot. If output arrives for a slot that was already removed (e.g., due to a race between process event delivery and a status transition), the detector silently re-creates state for a dead slot.
- **When:** Low urgency. Consider guarding with a slot-exists check when the observer is used at higher concurrency.

### I-021: Alert-only loop findings suppress re-alerting permanently

- **Source:** Sprint 3 review (2026-03-15), finding F-002
- **Module:** `observer.rs:88-91`
- **Severity:** Low
- **Details:** Each `SlotLoopState` has `emitted_*_alert` flags. Once fired, the alert won't fire again unless the slot is reset (which only happens on pause/kill/resume). If `LoopAction::Alert` is configured (no intervention), the operator gets one alert and no follow-ups. An operator who sees an alert and doesn't act gets no second warning.
- **When:** When `LoopAction::Alert` is the configured intervention. Consider a configurable alert cooldown.

### ~~I-022: `run_observer_tick` runs blocking git-status in async context~~ RESOLVED

- **Source:** Sprint 3 review (2026-03-15), finding F-003
- **Module:** `engine/mod.rs`
- **Severity:** Low
- **Resolved:** Sprint 4 (2026-03-15), branch `agent/gpt/sprint-4-engine-hardening`
- **Resolution:** Observer tick now uses `JoinSet::spawn_blocking` to run `has_worktree_changes()` concurrently for all working slots. Results are collected and fed to the observer after all checks complete.

### I-023: `candidate_paths` may false-positive on URLs and source locations

- **Source:** Sprint 3 review (2026-03-15), finding F-004
- **Module:** `observer.rs:341-353`
- **Severity:** Low
- **Details:** The path candidate extraction matches any whitespace-delimited token containing `/` or `\`. This matches URLs (`https://...`), Rust source locations (`src/lib.rs:42:`), MIME types (`application/json`), etc. Most false positives are harmless because they resolve inside the worktree, but an absolute path appearing in agent log output (e.g., `/etc/passwd` in an error message) would trigger a sandbox violation.
- **When:** Low urgency. The current behavior is conservative (false pause > false pass). Consider filtering URLs and source-location patterns.

### I-024: `LoopDetected` proto flattens three distinct observer finding kinds

- **Source:** Sprint 3 review (2026-03-15), finding F-006
- **Module:** `engine.rs:1825-1833`, `hypervisor.proto`
- **Severity:** Low
- **Details:** `ObserverFindingKind::LoopDetected`, `Stuck`, and `BudgetVelocity` all map to the same proto variant `observer_alert::Detail::LoopDetected`. A UI client can't switch on the finding kind without parsing the `reason` string.
- **When:** Phase 2/3 when building TUI or VS Code extension. Consider adding a `finding_kind` enum to the proto message or splitting into three variants.

### ~~I-025: `Review → Paused` creates un-resumable state via `ResumeAgent`/`ResumeSlot`~~ RESOLVED

- **Source:** Sprint 4 review (2026-03-15), finding F-01
- **Module:** `engine/commands.rs:235-241`
- **Severity:** Low
- **Resolved:** Sprint 6 (2026-03-15), branch `agent/gpt/sprint-6-integration-polish`
- **Resolution:** `resume_target()` now returns `Some(Review)` for slots paused from Review. Both unit and engine tests verify `Review → Paused → ResumeSlot → Review`.

### ~~I-026: TUI status colors diverge from kanban spec (D-009)~~ RESOLVED

- **Source:** Sprint 5 review (2026-03-15), finding F-01
- **Module:** `crates/nexode-tui/src/ui.rs:248-262`
- **Severity:** Medium
- **Resolved:** Sprint 5 (2026-03-15), pre-merge fix at `994822b`
- **Resolution:** Status colors aligned to kanban spec: WORKING→Cyan (Teal), MERGE_QUEUE→Blue, RESOLVING→Red, PAUSED→DarkGray (Gray), DONE→Green+Dim.

### ~~I-027: Event gap recovery drops triggering event~~ RESOLVED

- **Source:** Sprint 5 review (2026-03-15), finding F-02
- **Module:** `crates/nexode-tui/src/main.rs:322-330`
- **Severity:** Low
- **Resolved:** Sprint 6 (2026-03-15), branch `agent/gpt/sprint-6-integration-polish`
- **Resolution:** TUI gap recovery now reapplies the triggering event when its sequence exceeds the refreshed snapshot. Tests cover both replay and no-replay cases.

### ~~I-028: TUI timestamps always UTC under multi-threaded tokio~~ RESOLVED

- **Source:** Sprint 5 review (2026-03-15), finding F-03
- **Module:** `crates/nexode-tui/src/events.rs:112-117`
- **Severity:** Low
- **Resolved:** Sprint 6 (2026-03-15), branch `agent/gpt/sprint-6-integration-polish`
- **Resolution:** The TUI now captures `UtcOffset::current_local_offset()` in plain `main()` before the Tokio runtime starts, stores it in `AppState`, and passes it into event timestamp formatting. The log header explicitly labels UTC fallback mode.

---

## Risks from External Analysis

> Source: Gemini Agent Hypervisor IDE Architectural Analysis (March 2026). Validated against Nexode architecture by pc on 2026-03-15.

### R-008: VS Code Extension Host IPC bottleneck at N>3 agent streams

- **Source:** Gemini analysis Section 3 / Section 9 risk table (2026-03-15)
- **Module:** Future Phase 2+ (VS Code extension, not yet built)
- **Likelihood:** High
- **Impact:** High (UI lockups, dropped agent state updates)
- **Details:** The VS Code Extension Host runs in a single Node.js process with serialized IPC to the renderer. At 3+ agents streaming tokens at 50-100 tok/s each plus LSP updates and file watches, the channel saturates. Community reports document CPU spikes from single extensions performing heavy computation (GitHub issue #233842). This will matter when the Nexode VS Code extension is built.
- **Mitigation:** The gRPC transport (`transport.rs`) already exists as a high-throughput channel. The VS Code extension should connect directly to the daemon via gRPC/WebSocket, bypassing the Extension Host for agent data streams. Only use Extension Host for traditional extension functionality (themes, keybindings, language support). This is a deeper fork than a standard extension — plan for it.
- **When:** Phase 2+ when building the VS Code extension.

### R-009: Semantic drift between concurrent agents (post-merge failure)

- **Source:** Gemini analysis Section 6 / CodeCRDT paper findings (2026-03-15)
- **Module:** `engine.rs`, `git.rs` (merge-and-verify)
- **Likelihood:** Medium
- **Impact:** High (main branch corruption, silent semantic conflicts)
- **Details:** Git merge is syntactically aware but semantically blind. If Agent A alters a function signature in Worktree A and Agent B writes a new module calling the old signature in Worktree B, `git merge` succeeds but the build fails. D-008 post-merge verification (build + test) catches compile failures, but not runtime semantic conflicts (e.g., behavioral changes, API contract violations, duplicate logic). The CodeCRDT research measured 5-10% semantic conflict rates even with zero character-level merge failures, rising to 80% on tightly-coupled tasks.
- **Mitigation:** Sprint 3 Observer agent is the primary mitigation — it should monitor for loop states, uncertainty routing, and coherence drift. Longer term, consider a pre-merge semantic check that compares AST signatures across active worktrees before allowing a merge to proceed. Task decomposition that assigns agents to disjoint code regions is the most effective prevention.
- **When:** Now — this is a live risk as soon as multiple agents work on related code. Observer agent (Sprint 3) partially addresses it.

### R-010: Agent CLI output format instability (harness fragility)

- **Source:** Codex verify experience (2026-03-15) + Gemini analysis Section 9
- **Module:** `harness.rs` (all harness implementations)
- **Likelihood:** High (already experienced)
- **Impact:** Medium (false failures, broken telemetry)
- **Details:** Both Claude and Codex required harness adjustments after real verification. Claude needed `--verbose --output-format stream-json` (Sprint 2). Codex needed `type: "turn.completed"` detection and default model path (Codex verify). These CLIs are in active development (Codex is alpha `0.104.0-alpha.1`). Future versions may change JSON schemas, add/remove flags, or alter completion signals without notice.
- **Mitigation:** The `AgentHarness` trait already isolates format-specific logic. Keep fallback detection paths (e.g., Codex checks `turn.completed` || `event: done` || `status: completed`). Consider a harness version pinning mechanism or a harness self-test that validates assumptions against the installed CLI version.
- **When:** Ongoing. Every agent CLI upgrade is a potential harness regression.
