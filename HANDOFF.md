---
agent: claude
claimed: 2026-03-24T12:00:00-07:00
status: idle
from: claude
task: "Sprint 11-alt reviewed, fixes applied, ready for merge to main. Sprint 12 scoped."
branch: "agent/gpt/sprint-10c-view-modes"
---

# HANDOFF.md

> Last updated: 2026-03-24 by claude
> Sprint 11-alt (off-plan) reviewed and fixed. Ready to merge to main.

## Current Session (2026-03-24)

### Context: Agent Divergence Recovery

Multi-agent coordination broke down after Sprint 10C. The `pc` agent scoped Sprint 11 as "Merge Choreography TreeView + Extension Polish" and handed off to `gpt` on GitHub main (`f550cee`). However, a `claude` session built a different Sprint 11 on a local branch forked from the pre-merge commit (`34dc817`), producing workspace folder management, output streaming, diagnostics, decorations, and diff commands instead of the planned scope.

This session reviewed the off-plan work ("Sprint 11-alt"), confirmed it is architecturally sound and spec-aligned (Phase 3 `sec-11-weeks-5-6` native VS Code integrations), fixed critical bugs, and prepared it for merge. The originally planned Sprint 11 scope (Merge Choreography TreeView + Extension Polish) becomes Sprint 12.

### What was reviewed and fixed

**Sprint 11-alt** (commits `358646d`, `58601fb` — off-plan but valid Phase 3 work):

- Proto: AgentSlot.worktree_path, FullStateSnapshot.worktrees, AgentOutputLine, VerificationResult — **APPROVED**
- Daemon: worktree_path in snapshots, output event publishing, verification result publishing, broadcast buffer 256→2048 — **APPROVED**
- Extension: WorkspaceFolderManager, OutputChannelManager, DaemonClient output bypass — **APPROVED**
- Extension: DiagnosticManager + parser, DecorationProvider, webview action buttons — **APPROVED with fixes**

**Fixes applied in this session:**

1. **CRITICAL: Kanban action buttons used task ID instead of slot ID** — `webview/kanban/App.tsx:229-236`. Changed to `card.slot?.id` with null guard (buttons hidden when no slot assigned).
2. **CRITICAL: Rust diagnostic parser produced generic messages** — `diagnostic-parser.ts`. Rewrote to pre-parse `error[XXXX]: message` header lines and associate them with subsequent `-->` pointer lines. Now extracts real error messages and severity.
3. **HIGH: Missing error handling on `client.connect()`** — `extension.ts:142`. Added try-catch so extension activates gracefully when daemon isn't running.
4. **LOW: DaemonClient event routing undocumented** — `daemon-client.ts:210`. Added comment explaining protobuf oneof assumption for output bypass.

**Tests added:** Rust warning parsing, bare-pointer fallback message. Total: 30 TypeScript tests passing.

### Verification

All commands pass:
- `cargo check --workspace` ✅
- `cargo test --workspace` ✅
- `npm run build` ✅
- `npm run build:webview` ✅
- `npm run check-types` ✅
- `npm test` — 30 tests passing ✅

## Sprint 12 Scope (Next)

**Merge Choreography TreeView + Post-merge cleanup** — see PLAN_NOW.md

This is the originally planned Sprint 11 scope from `pc`'s handoff, renumbered to Sprint 12 after the off-plan Sprint 11-alt:
1. Merge Choreography TreeView (native VS Code TreeView in AuxiliaryBar)
2. Extension Settings Page
3. Extension README + Onboarding
4. Post-merge main branch cleanup (rebase, squash, PR)

## Previous Sessions

### 2026-03-19 (Sprint 10C — gpt)

Sprint 10 Tranche C delivered on `agent/gpt/sprint-10c-view-modes`.

- Synapse Grid view modes (Project Groups, Flat, Focus)
- Shared formatter extraction to `webview/shared/format.ts`
- Observer alert rendering in both webview surfaces
- Review by `pc`: APPROVED as PR #24, merged at `d13add7` on GitHub main

### 2026-03-23 (Sprint 11-alt — claude, off-plan)

Off-plan sprint built on local branch without syncing to GitHub main:

- WorkspaceFolderManager: worktree-as-workspace-folder reconciliation
- OutputChannelManager: per-slot VS Code output channels
- DiagnosticManager: verification failure → Problems panel diagnostics
- DecorationProvider: status badges on workspace folders
- Expanded webview message contract: Output/Diff action buttons

## Current State

**GitHub main:** `f550cee` — Sprint 10 complete, Sprint 11 handoff
**Local branch:** `agent/gpt/sprint-10c-view-modes` at `58601fb` + review fixes (not yet rebased onto origin/main)

### Codebase inventory (Sprints 0-11-alt)

| Component | Location | Language | Lines (approx) | Tests |
|---|---|---|---|---|
| nexode-daemon | `crates/nexode-daemon/` | Rust | ~8100 | 76 (lib+bin) |
| nexode-proto | `crates/nexode-proto/` | Proto/Rust | ~310 | 0 (generated) |
| nexode-ctl | `crates/nexode-ctl/` | Rust | ~625 | 4 |
| nexode-tui | `crates/nexode-tui/` | Rust | ~3050 | 34 (lib+bin) |
| nexode-vscode | `extensions/nexode-vscode/` | TypeScript | ~5700 | 30 (state, view-models, kanban-commands, format, diagnostic-parser) |
| **Total** | | | **~17,785** | **~144** |

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
| R-011 | Low | VS Code extension: Tier 2 integration tests still missing |
| NEW | Low | Sprint 11-alt: WorkspaceFolderManager/OutputChannelManager unit tests missing |
| NEW | Low | Sprint 11-alt: Output ring buffer (500-line VecDeque) not implemented — blocks Sprint 15 |
