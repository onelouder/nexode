---
agent: pc
status: handoff
from: pc
timestamp: 2026-03-17T22:30:00-07:00
task: "Sprint 9 — VS Code Extension Scaffold"
branch: "main"
next: gpt
---

# Handoff: Sprint 9 Ready for Codex

## What Just Happened

Sprint 8 (Daemon Hardening + Issue Sweep) was reviewed and merged to `main` via PR #20 (squash merge at `eab7705`). All exit criteria met, 114 tests pass, no regressions. Six issues closed (I-013, I-020, I-021, I-023, I-024, I-029). R-006 MSRV addressed. No new issues above Info severity.

Sprint 8 delivered:
- Observer hardening: unknown-slot guard, cooldown-based re-alerting, path candidate filtering
- Proto `FindingKind` enum with daemon→TUI round-trip
- Empty telemetry rejection for malformed `TOKENS` lines
- Claude harness doc update (`-p`, `--permission-mode bypassPermissions`)
- `rust-version = "1.85"` in all Cargo.toml + README
- Daemon restart → TUI reconnect integration test

Sprint 8 review: `docs/reviews/sprint-8-review.md`

## Sprint 9 Scope

Sprint 9 begins the VS Code Extension milestone (M3b). This is the first sprint that introduces a TypeScript/Node.js component. The goal is to scaffold the extension, establish the gRPC connection to the daemon, and render a minimal slot status panel.

### Part 1: Extension Scaffold

- Create `crates/nexode-vscode/` (or `extensions/nexode-vscode/`) directory
- Initialize with `yo code` or manual `package.json` + `tsconfig.json`
- Add VS Code extension manifest (`contributes.viewsContainers`, `views`)
- Add build system (esbuild or webpack for bundling)
- Extension activates on `nexode.*` commands

### Part 2: gRPC Client

- TypeScript gRPC client connecting to daemon at `localhost:{port}`
- Implement `GetFullState` call to fetch initial snapshot
- Implement `SubscribeEvents` streaming for real-time updates
- Connection status tracking (connected/disconnected/reconnecting)
- R-008 mitigation: consider WebSocket bridge if Extension Host IPC is a concern

### Part 3: Slot Status Webview

- TreeView or WebviewPanel showing project → slots hierarchy
- Display slot ID, task status, agent ID, token count
- Live updates from event stream
- Color-coded status badges matching D-009 kanban spec

### Part 4: Basic Command Dispatch

- Command palette entries: `Nexode: Pause Slot`, `Nexode: Resume Slot`, `Nexode: Move Task`
- Quick-pick for slot selection
- `DispatchCommand` gRPC call with `CommandResponse` feedback

## Sprint 9 Prompt

`.agents/prompts/sprint-9-codex.md` (to be written by Codex or pc)

## Read First

- `AGENTS.md`
- `.agents/openai.md`
- `HANDOFF.md` (this file)
- `PLAN_NOW.md`
- `ROADMAP.md` — M3b milestone
- `docs/spec/master-spec.md` — Phase 2 requirements
- `crates/nexode-proto/proto/hypervisor.proto` — gRPC service definition

## Context for Codex

### Daemon gRPC Service

The daemon exposes a gRPC service at `hypervisor.proto`:
- `SubscribeEvents(SubscribeRequest) → stream HypervisorEvent` — real-time event stream
- `DispatchCommand(OperatorCommand) → CommandResponse` — operator commands
- `GetFullState(StateRequest) → FullStateSnapshot` — full state snapshot

The proto file at `crates/nexode-proto/proto/hypervisor.proto` is the source of truth. The TypeScript client should generate types from this proto file.

### Existing Clients

- `nexode-ctl` (Rust CLI) — `crates/nexode-ctl/src/main.rs`
- `nexode-tui` (Rust TUI) — `crates/nexode-tui/src/`

Both are useful references for how to consume the daemon API.

### Key Constraint

This sprint focuses on the VS Code extension. Do NOT modify daemon, TUI, or proto code. The proto is stable. If the extension needs proto changes, flag them as deferred issues.

### Test Baseline

- Daemon: 73 lib + 3 bin = 76 tests
- Ctl: 4 tests
- TUI: 28 lib + 6 bin = 34 tests
- Total: 114 tests

### Remaining Open Issues

All Low severity, not blocking Sprint 9:
- I-004: `provider_config` shallow merge
- I-005: SQLite schema migration versioning
- I-011: Recovery re-enqueues merge slot without worktree check
- I-012: Token/byte conflation in `truncate_payload`
- I-018: `parse_json_summary_telemetry` double-count risk
