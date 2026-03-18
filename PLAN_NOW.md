---
updated: 2026-03-18
author: gpt
status: handoff
---

## Current Sprint

- **Goal:** Sprint 9 — VS Code Extension Scaffold
- **Deadline:** 2026-05-03
- **Active Agent:** pc
- **Current Branch:** `agent/gpt/sprint-9-vscode-scaffold`
- **Previous sprint:** Sprint 8 — Daemon Hardening + Issue Sweep (complete, merged to `main` via PR #20 at `eab7705`)

## Tasks

### Part 1: Extension Scaffold

- [x] Create `extensions/nexode-vscode/` directory with `package.json`, `tsconfig.json`
- [x] VS Code extension manifest: activation events, contributes.viewsContainers/views
- [x] Build system (esbuild bundling for production, ts-node or esbuild for dev)
- [x] Extension entry point (`extension.ts`) with activate/deactivate
- [x] `.vscodeignore` and packaging config

### Part 2: gRPC Client

- [x] Load `hypervisor.proto` at runtime with `@grpc/proto-loader`
- [x] `DaemonClient` class: connect, disconnect, connection state tracking
- [x] `GetFullState` — fetch initial snapshot on activation
- [x] `SubscribeEvents` — streaming event consumer with reconnect
- [x] Configuration: daemon host/port from VS Code settings

### Part 3: Slot Status Panel

- [x] TreeView provider showing project → slots hierarchy
- [x] Slot tree items: ID, status badge, agent ID, token count
- [x] Live refresh from event stream
- [x] Status colors matching D-009 kanban spec

### Part 4: Basic Command Dispatch

- [x] Command palette: `Nexode: Pause Slot`, `Nexode: Resume Slot`, `Nexode: Move Task`
- [x] Quick-pick slot selector
- [x] `DispatchCommand` call with `CommandResponse` feedback (info/error messages)
- [x] Status bar item showing connection state

## Blocked

- Manual extension-host smoke is still needed. The installed `code` command is Cursor CLI and rejects `--extensionDevelopmentPath`, so live activation against a real daemon was not runnable from this environment.

## Done This Sprint

- Added a new `extensions/nexode-vscode/` tree that stays outside the Rust workspace and includes `package.json`, `tsconfig.json`, esbuild bundling, `.vscodeignore`, local `.gitignore`, a placeholder activity-bar icon, and a copied `proto/hypervisor.proto`
- Implemented `DaemonClient` with `@grpc/grpc-js` + `@grpc/proto-loader`, unary `GetFullState`, streaming `SubscribeEvents`, `DispatchCommand`, connection-state events, and exponential-backoff reconnect (2s doubling to a 30s cap) matching `sec-11-phase-3-vscode-integration` / `sec-11-week-1-grpc-bridge-state-cache`
- Added a TypeScript `StateCache` that mirrors the TUI `apply_snapshot` / `apply_event` pattern and powers a native VS Code TreeView for project → slot hierarchy with D-009 color mapping, debounced refresh, and aggregate status-bar metrics
- Added command handlers for Pause Slot, Resume Slot, and Move Task with quick-pick selectors and gRPC feedback messaging
- Verification passed: `cd extensions/nexode-vscode && npm install && npm run build && npm run check-types`
- Verification passed: `cargo check --workspace`, `cargo test --workspace`
- Environment limitation noted: Cursor CLI does not support `--extensionDevelopmentPath`, so live VS Code activation and daemon round-trip remain a manual follow-up

## Done Previously (Sprint 8)

- Observer hardening: unknown-slot guard, cooldown-based re-alerting, path candidate filtering (I-020, I-021, I-023)
- Proto `FindingKind` enum with daemon→TUI round-trip (I-024)
- Empty telemetry rejection (I-013), Claude harness doc update (I-029)
- MSRV declaration and documentation (R-006)
- Daemon restart → TUI reconnect integration test
- Test count: 108 → 114 (+6)
- Review: `docs/reviews/sprint-8-review.md` — APPROVED

## Next Up

- `pc` review Sprint 9 extension scaffold
- Manual smoke in a real VS Code extension host: load `extensions/nexode-vscode`, connect to a running daemon, verify snapshot/event flow and command dispatch
