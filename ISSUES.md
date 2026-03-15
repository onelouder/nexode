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

### R-004: Global `AtomicU64` agent IDs not unique across restarts

- **Source:** Phase 0 review (2026-03-14)
- **Module:** `process.rs`
- **Likelihood:** Low
- **Impact:** Low (agent ID collisions in logs after restart)
- **Details:** `AGENT_COUNTER` resets to 1 on daemon restart. Agent IDs like `slot-a-agent-1` will repeat across daemon lifetimes.
- **Mitigation:** Prefix with daemon instance ID (PID, UUID, or epoch timestamp) when moving to production.

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
