# Agent-Human-Collaboration Template

> Multi-agent collaborative project using the Sovereign Project Template.  This sets up a file structure and guide for Human-AI dev teams.

## Quick Start

### For Humans
1. Clone this repo
2. Edit `AGENTS.md` — configure your agent roster and capabilities
3. Update `PLAN_NOW.md` with your first sprint
4. Start working

### For Agents
1. Read `AGENTS.md` (mandatory — this is your contract)
2. Read `PLAN_NOW.md` (what's happening now)
3. Read `HANDOFF.md` (whose turn is it)
4. Read your platform file in `.agents/` if it exists
5. Begin work within your capabilities

## Requirements

- Rust 1.85+ (edition 2024)

## Project Structure

```
├── AGENTS.md          ← Universal agent contract (READ FIRST)
├── PLAN_NOW.md        ← Current sprint / short-horizon plan
├── HANDOFF.md         ← Turn-based coordination baton
├── ARCHITECTURE.md    ← System design and module map
├── ROADMAP.md         ← Milestones and backlog
├── DECISIONS.md       ← Numbered decision log (D-001, D-002...)
├── CHANGELOG.md       ← Append-only change log
├── CLAUDE.md          ← Claude Code platform config (root convention)
├── .agents/           ← Platform-specific agent extensions
│   ├── openclaw.md    ← Jarvis / OpenClaw
│   ├── perplexity.md  ← Perplexity Spaces
│   └── openai.md      ← ChatGPT / Codex
├── docs/
│   ├── adr/           ← Architecture Decision Records (detailed)
│   ├── agent_guides/  ← Deep reference docs
│   ├── designs/       ← Feature and design docs
│   └── background/    ← Research context and references
├── research/          ← Research outputs (YYYY-MM-DD-slug.md)
├── src/               ← Source code
└── tests/             ← Test suite
```

## Conventions

- **Research files:** `research/YYYY-MM-DD-slug.md`
- **Commit messages:** `[agent-name] description`
- **Feature branches:** `agent/<name>/<feature>`
- **Decisions:** Referenced as `D-NNN` across all files

## Template

This project was created from the [Xcognis Sovereign Project Template](https://github.com/Xcognis/agents_template).

To use this template for a new project:
1. Click "Use this template" on GitHub
2. Customize AGENTS.md, ARCHITECTURE.md, and README.md
3. Configure `.agents/` files for your agent platforms

## License

[Choose your license]
