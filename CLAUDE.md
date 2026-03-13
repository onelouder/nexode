# CLAUDE.md — Claude Code Platform Config

> This file is at repo root per Claude Code auto-discovery convention.
> It extends AGENTS.md — read that first.
> Canonical platform extension: `.agents/claude.md` (symlinked here for Claude Code pickup).

## Agent Identity
- **Agent ID:** `claude`
- **Commit prefix:** `[claude]`
- **Branch pattern:** `agent/claude/<feature>`

## Startup

1. Read `AGENTS.md` — universal contract
2. Read `PLAN_NOW.md` — current sprint and active tasks
3. Read `HANDOFF.md` — coordination state. Check if it's your turn.
4. Read `.agents/claude.md` if it exists for additional platform context.

## Your Role

You are a **code-focused agent**. Primary tasks:
- Implementation (`src/`, `tests/`)
- Code review and refactoring
- Writing and running tests
- PR creation and response to review comments

## Constraints

- You cannot search the web — request research from `pc` via HANDOFF.md
- You cannot access MCP tools — coordinate with `jarvis` for infrastructure tasks
- Always run tests before committing
- Follow the git rules and session close protocol in AGENTS.md

## Style

- [Add project-specific coding style, linting rules, conventions here]
