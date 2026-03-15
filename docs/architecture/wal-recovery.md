# WAL Recovery Architecture

> **Status:** Proposed
> **Author:** pc
> **Date:** 2026-03-15
> **Sprint:** Sprint 1 (WAL + Agent Harness)
> **Requirements:** REQ-P1-002, REQ-P1-006, REQ-P2-006

---

## Problem

The Phase 0 daemon holds all runtime state in memory. If the process crashes, is killed, or the machine reboots, all session state is lost — task statuses, worktree assignments, cost totals, merge queue positions. The operator must restart from scratch.

Phase 1 requires the daemon to survive restarts: reload session state, re-attach to surviving agent processes, and respawn dead ones (REQ-P1-006).

## Design

### WAL File Format

The WAL is an append-only binary file stored at `.nexode/wal.binlog`, resolved relative to the directory containing `session.yaml`.

Each entry is a framed record:

```
┌──────────┬──────────┬──────────────────┐
│ len: u32 │ crc: u32 │ payload: [u8]    │
│ (4 bytes)│ (4 bytes)│ (len bytes)      │
└──────────┴──────────┴──────────────────┘
```

- `len`: byte length of the payload (not including the 8-byte header).
- `crc`: CRC-32C of the payload bytes. Used for integrity validation on read.
- `payload`: `bincode`-serialized `WalEntry` enum.

All integers are little-endian.

### Entry Types

```
SessionStarted    — Written once at daemon startup. Records config hash and instance ID.
SlotStateChanged  — Written on every task status transition. Captures slot/project/agent/PID/worktree.
TelemetryRecorded — Written on every telemetry update. Captures token counts and cost.
MergeCompleted    — Written after each merge attempt. Captures outcome (success/conflict/verify-fail).
Checkpoint        — Written periodically (every 60s). Contains a full serialized RuntimeState snapshot.
```

### Write Protocol

1. The WAL writer serializes the entry with `bincode`.
2. Computes CRC-32C of the serialized bytes.
3. Writes `[len][crc][payload]` to the file.
4. Calls `fsync()` on the file descriptor.

**Ordering guarantee:** All WAL writes happen on the engine's single thread (the engine loop is single-threaded by design). No concurrent write coordination is needed.

**Performance:** The WAL is not on the hot path for agent output streaming. The primary write sources are:
- Task status transitions (~10s of writes per minute across all slots)
- Telemetry updates (~1 per agent output line, but batched by the accounting actor)
- Checkpoint (1 per minute)

At this volume, synchronous `fsync()` is acceptable. If profiling shows it's a bottleneck, switch to `fdatasync()` or batch writes.

### Recovery Protocol

On daemon startup:

```
1. Parse session.yaml → SessionConfig
2. Look for .nexode/wal.binlog
   ├── Not found → fresh start (current behavior)
   └── Found → enter recovery mode
3. Scan WAL backward for most recent Checkpoint entry
4. Deserialize Checkpoint → RuntimeState
5. Replay all entries after Checkpoint in forward order
6. For each slot with recorded agent_pid:
   │  ├── PID alive? → log "re-attaching to PID {pid}" (see Re-attachment below)
   │  └── PID dead?  → mark for respawn
7. For each slot with worktree_path:
   │  ├── Path exists? → keep
   │  └── Path gone?   → clear worktree reference, log warning
8. Respawn all dead slots that were in WORKING state
9. Resume engine loop
```

### Agent Process Re-attachment

Re-attaching to a surviving agent process (step 6) is the hardest part.

**Option A: Don't re-attach.** On recovery, kill all surviving agent processes and respawn. This is simple and correct — the agent was probably mid-task anyway and restarting from its last git commit is safe.

**Option B: Re-attach via PID.** Check if PID is alive, re-open its stdout/stderr via `/proc/{pid}/fd/{1,2}`. This is Linux-specific, fragile (FD redirection may not work after parent death), and the daemon won't see output that was produced while it was down.

**Recommendation:** Start with Option A for Sprint 1. Kill + respawn is safe, simple, and correct. The slot's worktree preserves any committed work. Uncommitted work is lost, but that's the expected crash semantics. If agent restart costs prove too high (long context loading), Option B can be explored in a future sprint.

### Session Config Drift

The `SessionStarted` entry records a SHA-256 hash of the `session.yaml` file contents. On recovery:

- If the hash matches: proceed normally.
- If the hash differs: log a warning (`"session.yaml has changed since last run; recovered state may not match current config"`). Proceed anyway. The operator may have intentionally added/removed slots.
- If a recovered slot ID doesn't exist in the current config: skip it (log warning).
- If a current config slot ID doesn't exist in recovered state: start fresh for that slot.

### Compaction

After writing a `Checkpoint`, all entries before it are eligible for truncation. Compaction strategy:

1. Write `Checkpoint` to the current WAL file.
2. Rename `.nexode/wal.binlog` to `.nexode/wal.binlog.prev`.
3. Open a new `.nexode/wal.binlog`, write the `Checkpoint` as the first entry.
4. Delete `.nexode/wal.binlog.prev`.

If the daemon crashes during compaction, recovery finds either the old file (with full history) or the new file (with at least the checkpoint). Either way, recovery succeeds.

### Daemon Instance ID

Each daemon startup generates a UUID v4 `daemon_instance_id`. This is recorded in `SessionStarted` and used to:

1. Distinguish agent IDs across restarts (fixes R-004).
2. Detect stale WAL files from a different daemon instance.
3. Provide a correlation ID for logs.

The instance ID is passed to `next_agent_id()` as a prefix: `{instance_id_short}-{slot_id}-agent-{counter}`.

## Interaction with Existing Modules

| Module | Change |
|---|---|
| `engine.rs` | Add `Wal` field. Write entries on state changes. Periodic checkpoint. Recovery path. |
| `accounting.rs` | No change — the accounting DB is already persistent. WAL `TelemetryRecorded` entries are for state recovery, not for replacing SQLite. |
| `process.rs` | No change — the process manager is stateless. Recovery re-creates supervisors. |
| `git.rs` | No change — worktrees persist on disk. Recovery verifies they exist. |
| `transport.rs` | No change — gRPC bridge is re-created on startup. |
| `session.rs` | Add `config_hash()` method to compute SHA-256 of raw session file. |

## File Layout

```
.nexode/
├── wal.binlog                 # Active WAL file
├── wal.binlog.prev            # Previous WAL (during compaction only)
└── token-accounting.sqlite3   # Existing accounting DB
```

## Testing Strategy

| Test | What it proves |
|---|---|
| WAL write + read round-trip | Serialization/deserialization is correct |
| CRC validation | Corrupt entries are detected and skipped |
| Checkpoint compaction | Old entries are removed, new file starts with checkpoint |
| Full recovery integration | Kill daemon, restart, verify state matches |
| Config drift handling | Changed session.yaml still allows recovery |
| Missing worktree | Recovery handles deleted worktrees gracefully |
