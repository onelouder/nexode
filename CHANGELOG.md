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
