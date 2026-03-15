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

### R-005: Broadcast stream drops lagged events silently

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `transport.rs`
- **Likelihood:** Medium (under load)
- **Impact:** Medium (UI misses state transitions)
- **Details:** `BroadcastStream` filters out `RecvError::Lagged` events. Under burst conditions (e.g., 10 agents finishing simultaneously), slow clients lose events with no indication. Acknowledged in code comments.
- **Mitigation:** Add event sequence numbers. Implement replay or state catch-up for lagged clients in Phase 2+.

### R-006: `edition = "2024"` pins MSRV to Rust 1.85+

- **Source:** Phase 0 review (2026-03-14)
- **Module:** All crates
- **Likelihood:** Low
- **Impact:** Low (limits contributor compatibility)
- **Details:** All three crates use `edition = "2024"`, which requires Rust 1.85+. This is the latest stable edition. If contributors or CI use older toolchains, they'll hit build failures.
- **Mitigation:** Document MSRV in README.md or Cargo.toml `[package]` metadata. Intentional choice — just needs to be explicit.

### R-007: `CommandResponse` is fire-and-forget

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `transport.rs`
- **Likelihood:** High (will matter in Sprint 1)
- **Impact:** Medium (no feedback on command execution)
- **Details:** `dispatch_command` always returns `success: true` as long as the channel is open, regardless of whether the command was processed or what happened. No request/response correlation.
- **Mitigation:** Engine loop needs to add command acknowledgment with result status. Consider a command ID → result callback pattern.

### I-007: Merge queue drains on tick only (2s delay)

- **Source:** Phase 0 review v2 (2026-03-14)
- **Module:** `engine.rs`
- **Severity:** Low
- **Details:** `drain_merge_queues()` runs on the tick interval (default 2s), not immediately when a task enters MERGE_QUEUE via `enqueue_merge()`. For Phase 0 this is fine. For production, consider draining immediately on enqueue.
- **When:** When merge latency matters (Phase 2+).

### I-008: Daemon `main.rs` uses manual arg parsing instead of `clap`

- **Source:** Phase 0 review v2 (2026-03-14)
- **Module:** `crates/nexode-daemon/src/main.rs`
- **Severity:** Low
- **Details:** The daemon binary does manual `std::env::args()` parsing with `--flag value` matching, while `nexode-ctl` uses `clap` with derive macros. Minor inconsistency — no `--help` support on the daemon.
- **When:** Whenever someone touches daemon CLI args.

### I-009: `completion_detected` overrides non-zero exit as success

- **Source:** Sprint 1 review (2026-03-15), finding F-001
- **Module:** `process.rs:325`
- **Severity:** Medium
- **Details:** `success: status.success() || completion_detected` means an agent that prints a completion marker early and then crashes with a non-zero exit code is reported as successful. Could silently promote a crashed agent to REVIEW instead of respawning.
- **When:** Before real CLI agent testing. Review semantics — consider requiring both `completion_detected && status.success()` for true success.

### I-010: `AgentStateChanged(Executing)` dropped after swap

- **Source:** Sprint 1 review (2026-03-15), finding F-003
- **Module:** `engine.rs` (SlotAgentSwapped handler)
- **Severity:** Medium
- **Details:** The `AgentStateChanged(Executing)` event for the new agent was removed from the `SlotAgentSwapped` handler. After a crash-respawn, gRPC subscribers won't see the new agent enter `Executing` state — only the swap event. Regression from Phase 0 behavior.
- **When:** Phase 2/3 when TUI or VS Code extension subscribes to agent state changes.

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

### I-014: Architecture doc CLI flags out of date

- **Source:** Sprint 1 review (2026-03-15), finding F-009
- **Module:** `docs/architecture/agent-harness.md`
- **Severity:** Low
- **Details:** Sprint instructions specified `codex --approval-mode full-auto`, but the implementation uses `codex exec --full-auto --json` (aligned to actual CLI). Architecture doc should be updated to match.
- **When:** Next docs update by pc.

### I-015: JSON substring matching in completion detection

- **Source:** Sprint 1 review (2026-03-15), finding F-010
- **Module:** `harness.rs:177-178`
- **Severity:** Low
- **Details:** `ClaudeCodeHarness.detect_completion` uses `line.contains("\"type\":\"result\"")` — fragile against whitespace in JSON or the word "completed" in agent output. Combined with I-009, could cause false success.
- **When:** Before real Claude Code CLI testing.
