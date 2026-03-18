---
updated: 2026-03-17
author: pc
status: planning
---

## Current Sprint

- **Goal:** Sprint 9 — VS Code Extension Scaffold
- **Deadline:** 2026-05-03
- **Active Agent:** gpt (Codex)
- **Current Branch:** `agent/gpt/sprint-9-vscode-scaffold` (to be created)
- **Previous sprint:** Sprint 8 — Daemon Hardening + Issue Sweep (complete, merged to `main` via PR #20 at `eab7705`)

## Tasks

### Part 1: Extension Scaffold

- [ ] Create `extensions/nexode-vscode/` directory with `package.json`, `tsconfig.json`
- [ ] VS Code extension manifest: activation events, contributes.viewsContainers/views
- [ ] Build system (esbuild bundling for production, ts-node or esbuild for dev)
- [ ] Extension entry point (`extension.ts`) with activate/deactivate
- [ ] `.vscodeignore` and packaging config

### Part 2: gRPC Client

- [ ] Generate TypeScript types from `hypervisor.proto` (grpc-tools or buf)
- [ ] `DaemonClient` class: connect, disconnect, connection state tracking
- [ ] `GetFullState` — fetch initial snapshot on activation
- [ ] `SubscribeEvents` — streaming event consumer with reconnect
- [ ] Configuration: daemon host/port from VS Code settings

### Part 3: Slot Status Panel

- [ ] TreeView provider showing project → slots hierarchy
- [ ] Slot tree items: ID, status badge, agent ID, token count
- [ ] Live refresh from event stream
- [ ] Status colors matching D-009 kanban spec

### Part 4: Basic Command Dispatch

- [ ] Command palette: `Nexode: Pause Slot`, `Nexode: Resume Slot`, `Nexode: Move Task`
- [ ] Quick-pick slot selector
- [ ] `DispatchCommand` call with `CommandResponse` feedback (info/error messages)
- [ ] Status bar item showing connection state

## Blocked

- Nothing

## Done This Sprint

- (Sprint 9 not yet started)

## Done Previously (Sprint 8)

- Observer hardening: unknown-slot guard, cooldown-based re-alerting, path candidate filtering (I-020, I-021, I-023)
- Proto `FindingKind` enum with daemon→TUI round-trip (I-024)
- Empty telemetry rejection (I-013), Claude harness doc update (I-029)
- MSRV declaration and documentation (R-006)
- Daemon restart → TUI reconnect integration test
- Test count: 108 → 114 (+6)
- Review: `docs/reviews/sprint-8-review.md` — APPROVED
