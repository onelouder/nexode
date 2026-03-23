# Multiplexed Native Workspace Architecture

> Status: APPROVED (2026-03-22)
> Author: claude + jwells
> Spec alignment: sec-05 (UI/UX Surfaces), sec-11 (Phase 3 VS Code Integration)
> Supersedes: None (new capability layer on top of existing webview surfaces)

## Motivation

Nexode's codebase has converged on process management and passive monitoring. The daemon and webview surfaces are mature, but the human-agent transaction layer is narrow: the webview message contract supports only `ready` and `moveTask`. Competitive analysis (Conductor) confirms market expectations for: agent output visibility, diff-based review, pre-merge gating, CI failure re-routing, and lifecycle scripts.

Nexode sits *inside* VS Code. Instead of building custom webview surfaces for per-agent interaction, Nexode can **multiplex VS Code's native surfaces** — Explorer, SCM, editor, terminal, output, problems, comments, decorations — across agent worktrees. This delivers full IDE capabilities (intellisense, go-to-definition, debugger, every installed extension) pointed at each agent's work, with no custom UI to build.

The Synapse Grid and Kanban webviews retain their role as fleet-level overview surfaces. Per-agent interaction shifts to native VS Code surfaces.

## Architecture

```
                    ┌──────────────────────────────────────────────────────┐
                    │                VS Code Window                       │
                    │                                                      │
Fleet Overview      │  Synapse Grid (webview)    Macro Kanban (webview)    │
(existing)          │  - 3 view modes            - 8-column drag-drop     │
                    │  - metric headers           - project filter         │
                    │  - alert badges             - task cards             │
                    ├──────────────────────────────────────────────────────┤
                    │                                                      │
Per-Agent           │  Explorer          → worktree folders per slot       │
Interaction         │  Source Control    → native Git diffs per worktree   │
(new)               │  Editor Tabs      → browse agent code + intellisense│
                    │  Output Panel      → agent stdout/stderr per slot    │
                    │  Problems Panel    → verification failure diagnostics│
                    │  Comments          → agent decision threads          │
                    │  File Decorations  → status badges on folder roots   │
                    │  Task Runner       → lifecycle scripts per worktree  │
                    │  Terminal          → shell access to agent worktrees │
                    ├──────────────────────────────────────────────────────┤
                    │                                                      │
Existing            │  Slot Tree View    Status Bar    Command Palette     │
                    │                                                      │
                    └──────────────────────────────────────────────────────┘
                                          │
                                    gRPC Bridge
                                          │
                    ┌──────────────────────────────────────────────────────┐
                    │              nexode-daemon (Rust)                    │
                    │  + AgentOutputLine event publishing                  │
                    │  + VerificationResult event with diagnostics         │
                    │  + AgentDecision event parsing                       │
                    │  + Failure re-routing (FIXING status)                │
                    │  + Pre-merge gating (MergeGate)                     │
                    │  + Lifecycle script execution                        │
                    └──────────────────────────────────────────────────────┘
```

## Feature Inventory

| ID | Feature | VS Code API | Sprint |
|----|---------|-------------|--------|
| F-01 | Worktree-as-workspace-folder | `workspace.updateWorkspaceFolders()` | 11 |
| F-02 | Agent output streaming | `window.createOutputChannel()` | 11 |
| F-03 | Native SCM + diff review | `commands.executeCommand('vscode.diff')` | 12 |
| F-04 | Verification diagnostics | `languages.createDiagnosticCollection()` | 12 |
| F-05 | Lifecycle scripts as tasks | `tasks.registerTaskProvider()` | 13 |
| F-06 | Decision surfacing (comments) | `comments.createCommentController()` | 14 |
| F-07 | File decorations (agent status) | `window.registerFileDecorationProvider()` | 12 |
| F-08 | Failure re-routing (auto-fix) | N/A (daemon-side) | 13 |
| F-09 | Pre-merge gating | N/A (daemon-side + command) | 14 |
| F-10 | Expanded webview messages | postMessage contract extension | 12 |

## Proto Changes Summary

All changes are append-only. Both proto copies must stay in sync.

### New fields on existing messages

| Message | Field | Number | Type |
|---------|-------|--------|------|
| `AgentSlot` | `worktree_path` | 10 | `string` |
| `FullStateSnapshot` | `worktrees` | 6 | `repeated Worktree` |

### New event types (`HypervisorEvent.oneof payload`)

| Message | Field Number | Sprint |
|---------|-------------|--------|
| `AgentOutputLine` | 13 | 11 |
| `VerificationResult` + `DiagnosticEntry` | 14 | 12 |
| `AgentDecision` | 15 | 14 |
| `MergeGateUpdate` + `GateItem` | 16 | 14 |

### New command types (`OperatorCommand.oneof action`)

| Message | Field Number | Sprint |
|---------|-------------|--------|
| `RespondToDecision` | 12 | 14 |
| `UpdateMergeGate` | 13 | 14 |
| `RequestSlotOutput` | 14 | 15 |

### New enum value

| Enum | Value | Number | Sprint |
|------|-------|--------|--------|
| `TaskStatus` | `TASK_STATUS_FIXING` | 9 | 13 |

## Session.yaml Schema Additions

```yaml
# All new fields use serde(default) for backward compatibility
projects:
  - id: my-project
    lifecycle:                        # NEW (Sprint 13)
      setup: "npm install"
      run: "npm run dev"
      archive: "rm -rf node_modules"
    verify:
      build: "cargo build"
      test: "cargo test"
      auto_fix: true                  # NEW (Sprint 13) — re-route failures to agent
      max_fix_attempts: 2             # NEW (Sprint 13) — prevent infinite loops
      gates:                          # NEW (Sprint 14) — pre-merge checklist
        - id: tests-pass
          label: "All tests pass"
          auto: true
        - id: human-reviewed
          label: "Human reviewed code"
          auto: false
```

## Data Flows

### Agent output: process → VS Code OutputChannel

```
Agent stdout → tokio BufReader (process.rs)
  → AgentProcessEvent::Output { slot_id, line, ... }
  → engine publish_event(AgentOutputLine)
  → broadcast::Sender<HypervisorEvent>
  → gRPC SubscribeEvents stream
  → DaemonClient.onDidReceiveAgentOutput (bypass StateCache)
  → OutputChannelManager → vscode.OutputChannel.appendLine()
```

### Verification failure → Problems panel + agent re-routing

```
merge_and_verify() fails (git.rs)
  → DiagnosticParser.parse(stdout, stderr)
  → publish_event(VerificationResult { diagnostics })
  → gRPC → extension → DiagnosticManager
  → vscode.DiagnosticCollection.set(uri, diagnostics)
  → if auto_fix: restart slot with failure context (FIXING status)
```

### Agent decision → inline comment → human reply → agent

```
Agent emits NEXODE_DECISION marker
  → parse in process.rs → publish AgentDecision event
  → gRPC → extension → CommentController
  → vscode.comments.createCommentThread(uri, range)
Human replies in VS Code comment UI
  → dispatch RespondToDecision command
  → daemon writes .nexode/decisions/{id}.md in worktree
  → agent reads on next context compilation
```

## Sprint Sequence

| Sprint | Theme | Features | Verification |
|--------|-------|----------|-------------|
| 11 | Foundation | F-01, F-02 | Worktrees appear as folders; output visible in Output panel |
| 12 | Feedback | F-03, F-04, F-07, F-10 | Diagnostics in Problems; status badges; diff command; webview actions |
| 13 | Repair | F-05, F-08 | Auto-fix re-routes agent; lifecycle tasks in Command Palette |
| 14 | Collaboration | F-06, F-09 | Comment threads at code locations; merge gating blocks MERGE_QUEUE |
| 15 | Integration | Backfill, E2E tests | Reconnect repopulates output; full workflow tested |
| 16 | Hardening | Throttling, resilience | Stable at 10+ concurrent agents under load |

## Risks

| ID | Severity | Risk | Mitigation |
|----|----------|------|-----------|
| R1 | High | Workspace folder mutation races | Debounced batch updates, desired-state reconciliation |
| R2 | High | Output event bandwidth saturation | Rate limit (200 lines/sec/slot), buffer increase to 2048, telemetry filtering |
| R3 | Medium | Proto file sync drift | CI diff check between both copies |
| R4 | Medium | Multi-root workspace UX varies by extension | Documentation, opt-out config, reset command |
| R5 | Medium | Diagnostic parser accuracy per toolchain | Start with rustc/tsc/generic; user-configurable patterns |
| R6 | Low | Decision response file delivery | Queue responses, deliver on next agent spawn if process exited |

## Decisions

This architecture extends but does not modify the locked spec (v2.0.1). New proto fields and enum values are append-only additions. The `TASK_STATUS_FIXING` enum value and all new event/command types are implementation decisions within the Phase 3 scope, consistent with the spec's directive that the extension is a rendering shell consuming daemon state.
