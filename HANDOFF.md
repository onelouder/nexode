---
agent: gpt
status: handoff
from: gpt
timestamp: 2026-03-18T10:10:00-07:00
task: "Sprint 9 — VS Code Extension Scaffold"
branch: "agent/gpt/sprint-9-vscode-scaffold"
next: pc
---

# Handoff: Sprint 9 Extension Scaffold Complete

## What Was Done

- Added a new `extensions/nexode-vscode/` top-level extension package outside the Rust workspace with `package.json`, `tsconfig.json`, `esbuild.mjs`, `.vscodeignore`, local `.gitignore`, a copied `proto/hypervisor.proto`, and a placeholder activity-bar icon
- Implemented `src/daemon-client.ts` using `@grpc/grpc-js` + `@grpc/proto-loader`
  - `GetFullState`, `SubscribeEvents`, and `DispatchCommand`
  - connection-state events
  - exponential-backoff reconnect from 2s to a 30s cap
  - endpoint reconfiguration from VS Code settings
- Implemented `src/state.ts` as a local snapshot/event mirror aligned to the TUI `apply_snapshot` / `apply_event` pattern
- Implemented `src/slot-tree-provider.ts`
  - project → slot hierarchy
  - D-009-style status color mapping via `ThemeIcon`
  - slot descriptions with status, agent id, and token totals
  - debounced refresh
- Implemented `src/commands.ts`
  - `Nexode: Pause Slot`
  - `Nexode: Resume Slot`
  - `Nexode: Move Task`
  - quick-pick selectors and command-response feedback
- Implemented `src/status-bar.ts`
  - connected / reconnecting / disconnected footer indicator
  - aggregate agent count and token totals
  - click-through to the Nexode activity bar

## Verification

- `cd extensions/nexode-vscode && npm install`
- `cd extensions/nexode-vscode && npm run build`
- `cd extensions/nexode-vscode && npm run check-types`
- `cargo check --workspace`
- `cargo test --workspace`
- `code --version` confirms only Cursor CLI is installed locally
- `code --extensionDevelopmentPath ...` is not supported by Cursor CLI, so live extension-host activation was not runnable here

## Outputs

- `extensions/nexode-vscode/package.json`
- `extensions/nexode-vscode/package-lock.json`
- `extensions/nexode-vscode/tsconfig.json`
- `extensions/nexode-vscode/esbuild.mjs`
- `extensions/nexode-vscode/.vscodeignore`
- `extensions/nexode-vscode/.gitignore`
- `extensions/nexode-vscode/proto/hypervisor.proto`
- `extensions/nexode-vscode/resources/nexode-icon.svg`
- `extensions/nexode-vscode/src/extension.ts`
- `extensions/nexode-vscode/src/daemon-client.ts`
- `extensions/nexode-vscode/src/state.ts`
- `extensions/nexode-vscode/src/slot-tree-provider.ts`
- `extensions/nexode-vscode/src/commands.ts`
- `extensions/nexode-vscode/src/status-bar.ts`
- `PLAN_NOW.md`
- `CHANGELOG.md`
- `HANDOFF.md`

## Next Agent

Recommended next step: `pc` review Sprint 9 and merge if approved.

Residual risk to review:

- Manual smoke remains: open the extension in a real VS Code extension host, connect to a live daemon, and confirm snapshot/event flow plus command dispatch
- Cursor CLI in this environment cannot launch an extension-development host (`--extensionDevelopmentPath` unsupported)
- The extension currently uses runtime proto loading plus handwritten normalization instead of generated static TypeScript stubs
