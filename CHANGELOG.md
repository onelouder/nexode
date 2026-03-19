# CHANGELOG.md

> Append-only log of significant changes. Auto-generated from commits or manually maintained.
> Format: `[YYYY-MM-DD] [agent] Description`

---

- [2026-03-08] [jarvis] Repository initialized from sovereign-project-template
- [2026-03-15] [gpt] Added Sprint 1 WAL recovery, synchronous agent harness adapters, context compilation, and recovery-aware daemon bootstrap
- [2026-03-15] [gpt] Added Sprint 2 real-agent fixes: strict exit handling, JSON completion parsing, command acknowledgments, gated live harness smoke tests, and demo script
- [2026-03-15] [gpt] Fixed the Claude live harness contract to use JSON stream output, parse final telemetry correctly, and stabilize credential-backed live tests
- [2026-03-15] [gpt] Verified Codex live execution, updated Codex harness completion/telemetry handling to match real `turn.completed` JSON output, and switched live Codex verification to the CLI default model
- [2026-03-15] [gpt] Added Sprint 3 observer safety: loop detection, sandbox enforcement, event sequencing with gap recovery, uncertainty routing, and slot-scoped resume commands
- [2026-03-15] [gpt] Completed Sprint 4: decomposed the daemon engine into modules, hardened pause/resume task transitions, moved observer git-status checks off the async runtime, and replaced daemon arg parsing with clap
- [2026-03-15] [gpt] Added Sprint 5 `nexode-tui`: a ratatui dashboard with live gRPC state, event log, keyboard controls, command dispatch, and terminal-safe shutdown handling
- [2026-03-15] [pc] Fixed TUI status colors to align with kanban spec D-009 (I-026)
- [2026-03-15] [gpt] Completed Sprint 6: fixed TUI gap recovery and timezone handling, fixed Review resume and immediate merge draining, added daemon→TUI gRPC integration coverage, and cleaned up CLI/docs polish
- [2026-03-17] [gpt] Completed Sprint 7: hardened TUI reconnect behavior, command history/completion/status UX, help overlay, demo wait-for-DONE flow, and LoopDetected label parsing
- [2026-03-17] [gpt] Completed Sprint 8: hardened observer slot/cooldown/path handling, added proto finding_kind support, rejected empty TOKENS telemetry, documented Claude permission flags, declared Rust 1.85 MSRV, and added daemon restart/reconnect integration coverage
- [2026-03-18] [gpt] Added Sprint 9 `extensions/nexode-vscode`: VS Code extension scaffold, gRPC daemon client with reconnect, native project→slot TreeView, slot command palette actions, and connection/metrics status bar
- [2026-03-19] [gpt] Normalized Sprint 10 Phase 3 planning, added the VS Code webview build pipeline and shell panels/sidebar, decoupled `state.ts` from the VS Code runtime, and added Tier 1 extension state tests
- [2026-03-19] [gpt] Completed Sprint 10 Tranche B: added live Synapse Grid and Macro Kanban rendering, task/slot/project join selectors, Kanban drag-and-drop move dispatch, agent-state tracking in `StateCache`, and expanded Tier 1 extension coverage
