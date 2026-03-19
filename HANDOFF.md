---
agent: gpt
claimed: 2026-03-19T07:41:41-07:00
status: claimed
from: gpt
task: "Sprint 10 — React Webviews + Extension Tests"
branch: "agent/gpt/sprint-10-react-webviews"
next: gpt
---

# HANDOFF.md

> Last updated: 2026-03-19 by gpt
> Sprint 9 review complete. Merged as PR #21 at `0c8cee4`.

## Current Session (2026-03-19)

Sprint 10 is actively claimed by `gpt`. This session is normalizing the implementation plan against the locked Phase 3 spec and accepted decisions before code work continues.

Planning targets for this session:

- align Sprint 10 scope with `sec-11-week-1-grpc-bridge-state-cache` and `sec-11-weeks-2-4-multi-monitor-react-webviews`
- preserve D-005 manual multi-monitor scope for Phase 3
- preserve D-009 / D-010 Kanban semantics, including `MERGE_QUEUE` and `RESOLVING`
- clarify `MoveTask` vs `AssignTask` semantics for the Kanban board before implementing webview plumbing
- after plan normalization, turn the restored local VS Code extension WIP into a green plumbing tranche

## Overnight Checkpoint (2026-03-18)

Sprint 10 implementation has not started yet. No extension or Rust source files were edited in this session; the intentional changes here are only the baton updates in `HANDOFF.md` and `PLAN_NOW.md`.

This session only established the Sprint 10 baton and verified scope:

- `PLAN_NOW.md` now marks Sprint 10 as the active branch context
- `HANDOFF.md` now closes the session cleanly instead of leaving `status: claimed`
- The next session should begin implementation from the existing Sprint 9 scaffold in `extensions/nexode-vscode/`
- `git pull --rebase --autostash origin main` restored pre-existing local Sprint 10 WIP in `extensions/nexode-vscode/.vscodeignore`, `extensions/nexode-vscode/esbuild.mjs`, `extensions/nexode-vscode/package.json`, and `extensions/nexode-vscode/tsconfig.json`; those edits were not created or validated in this session and remain uncommitted

### Recommended first actions tomorrow

1. Add the webview build pipeline and package scripts
2. Implement the Synapse Grid panel and React webview
3. Implement the Macro Kanban panel and React webview
4. Add Tier 1 unit tests for `src/state.ts`
5. Run extension and workspace verification before handing off to `pc`

## Current State

**Main branch:** `0c8cee4` — Sprint 9: VS Code Extension Scaffold (#21)

### What just shipped (Sprint 9)

The `nexode-vscode` extension scaffold under `extensions/nexode-vscode/`. First TypeScript component in the workspace. Covers master-spec section 11 "Week 1: Extension Scaffold":

- gRPC daemon client with exponential-backoff reconnect (`daemon-client.ts`)
- Local state cache with snapshot + event-driven updates (`state.ts`)
- TreeView slot browser with color-coded status icons (`slot-tree-provider.ts`)
- Status Bar HUD showing connection state, agent count, tokens (`status-bar.ts`)
- Command palette: pause/resume/move via QuickPick selectors (`commands.ts`)
- Configuration: `nexode.daemonHost`, `nexode.daemonPort` with live reload

Review: `docs/reviews/sprint-9-review.md` — APPROVED, no findings above Low severity.

### Codebase inventory (Sprints 0-9)

| Component | Location | Language | Lines (approx) | Tests |
|---|---|---|---|---|
| nexode-daemon | `crates/nexode-daemon/` | Rust | ~8000 | 76 (lib+bin) |
| nexode-proto | `crates/nexode-proto/` | Proto/Rust | ~300 | 0 (generated) |
| nexode-ctl | `crates/nexode-ctl/` | Rust | ~600 | 4 |
| nexode-tui | `crates/nexode-tui/` | Rust | ~3000 | 34 (lib+bin) |
| nexode-vscode | `extensions/nexode-vscode/` | TypeScript | ~1400 | 0 |
| **Total** | | | ~13,300 | **114** |

### Sanity Check: Sprints 1-9 vs Master-Spec

#### Phase 0 (Section 8) — COMPLETE

| Requirement | Status | Sprint |
|---|---|---|
| Session config parser with cascade | Done | 0 |
| Git worktree orchestrator | Done | 0 |
| Agent process manager with telemetry | Done | 0 |
| Token accountant (SQLite) | Done | 0 |
| gRPC skeleton (events, commands, state) | Done | 0 |
| Crash recovery (<2s respawn) | Done | 0 |
| Merge-and-verify pipeline | Done | 0 |

#### Phase 1 (Section 9) — COMPLETE

| Requirement | Status | Sprint |
|---|---|---|
| Full domain objects (Session, Project, Slot, Agent, Task) | Done | 0-1 |
| Full gRPC service (3 RPCs) | Done | 0-2 |
| OperatorCommand routing (all variants) | Done | 0-2 |
| Event sourcing (all mutations emit events) | Done | 0-3 |
| WAL persistence + crash recovery | Done | 1 |
| AgentHarness trait + Claude/Codex harnesses | Done | 1-2 |
| Context compiler (task + globs + git diff) | Done | 1 |
| Heartbeat loop (2s liveness check) | Done | 0 |
| Budget loop (warn/max alerts, hard kill) | Done | 0 |
| HITL checkpoint (uncertainty routing) | Done | 3 |
| Command acknowledgment (oneshot response) | Done | 2 |
| Event sequence numbers + gap recovery | Done | 3 |

#### Phase 2 (Section 10) — COMPLETE

| Requirement | Status | Sprint |
|---|---|---|
| ratatui TUI: three-pane layout | Done | 5 |
| gRPC subscriber with state mirror | Done | 5 |
| Project group headers with color | Done | 5 |
| Keyboard controls (p/r/k/?/tab) | Done | 5-7 |
| Event log (last N events with timestamps) | Done | 5 |
| Command mode with tab-complete | Done | 7 |
| Auto-reconnect with backoff | Done | 7 |
| Help overlay | Done | 7 |
| Observer: loop detection, sandbox, uncertainty | Done | 3 |

**Phase 2 gaps (not yet built):**
- Live token velocity spark-line charts (spec 10 "Weeks 2-3") — the TUI shows token counts but not spark-line charts. Low priority; the core metric display works.
- HITL popup modal overlay (spec 10 "Weeks 4-6") — uncertainty events trigger auto-pause and appear in the event log, but there's no modal overlay prompt. The operator resumes via command mode. Functionally equivalent; UX polish is deferred.
- Budget warning visual flash (spec 10 "Weeks 4-6") — budget alerts fire and display, but no orange/red flash animation on project headers. Cosmetic.

#### Phase 3 Week 1 (Section 11) — COMPLETE (Sprint 9)

| Requirement | Status | Sprint |
|---|---|---|
| Extension pack: `nexode-vscode` TypeScript project | Done | 9 |
| Registers all commands, views, chat participants | Partial | 9 |
| gRPC client: connect, event stream, state mirror | Done | 9 |
| Status Bar HUD: agent count, total cost, tokens | Done | 9 |

**Partial note:** The extension registers commands and views but not the `@nexode` chat participant. The chat participant is spec section 11 "Weeks 5-8" scope. Commands (pause/resume/move) and views (TreeView, activity bar) are fully registered.

#### Phase 3 Weeks 2-8 (Section 11) — NOT STARTED

| Requirement | Status | Target |
|---|---|---|
| Synapse Grid WebviewPanel (React) | Not started | Sprint 10 |
| Macro Kanban WebviewPanel (React) | Not started | Sprint 10 |
| Merge Choreography TreeView | Not started | Sprint 10+ |
| Universal Command Chat (@nexode participant) | Not started | Sprint 10+ |
| Extension polish (settings, onboarding, README) | Not started | Sprint 10+ |
| VS Code Marketplace publishing | Not started | Sprint 10+ |

### Open Issues

| ID | Severity | Summary |
|---|---|---|
| I-004 | Low | `provider_config` shallow merge not implemented |
| I-005 | Low | SQLite schema has no migration versioning |
| I-011 | Low | Recovery re-enqueues merge slot without worktree check |
| I-012 | Low | Token/byte conflation in `truncate_payload` |
| I-018 | Low | `parse_json_summary_telemetry` could double-count |
| R-001 | Low | Verification worktree cleanup on panic |
| R-002 | Medium | `sh -lc` in verification loads user dotfiles |
| R-003 | Low | Telemetry parsing format undocumented |
| R-008 | High | VS Code Extension Host IPC bottleneck at N>3 |
| R-009 | Medium | Semantic drift between concurrent agents |
| R-010 | Medium | Agent CLI output format instability |
| R-011 | High | VS Code extension has no test coverage |

### Key architectural decisions

1. **Runtime proto loading** over generated stubs — avoids `protoc` build dependency, trades type safety for build simplicity. The `state.ts` normalization layer compensates with defensive coercion.
2. **TreeView** (native VS Code) for slot display, **WebviewPanel** (React) reserved for Synapse Grid and Kanban. This matches the spec: TreeView for hierarchical data, Webview for rich interactive UIs.
3. **Generation-based connection tracking** in `DaemonClient` — same pattern as the Rust daemon's slot agent tracking. Prevents stale callback pollution after reconnects.

## For Sprint 10 Agent

See `PLAN_NOW.md` for the Sprint 10 task definition.
