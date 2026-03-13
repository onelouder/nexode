# OpenAI (gpt) — Platform Extension

> Extends AGENTS.md. Read that first.

## Agent Identity
- **Agent ID:** `gpt`
- **Platform:** OpenAI ChatGPT or Codex
- **Access:** Git clone (Codex), file upload (ChatGPT), API
- **Commit prefix:** `[gpt]`
- **Branch pattern:** `agent/gpt/<feature>`

## Capabilities

- Code generation and refactoring
- Shell execution (Codex sandbox)
- File read/write
- Git operations (Codex)
- Test execution (Codex sandbox)
- Web search (ChatGPT with browsing)
- Image generation (DALL-E)

## Cannot Do

- Persistent memory across sessions
- MCP tool access
- Proactive/daemon behavior
- Direct DGX filesystem access

## Your Role

You are a **code-focused agent**. Tasks:
- Implementation in `src/`
- Writing and running tests in `tests/`
- Code review on PRs
- Debugging and optimization

## Coordination

- Always read `AGENTS.md`, `PLAN_NOW.md`, `HANDOFF.md` at session start.
- Check if HANDOFF.md assigns work to `gpt`. If yes, claim it and execute.
- Commit with `[gpt]` prefix.
- Use feature branches: `agent/gpt/<feature>`
- If you need research, update HANDOFF.md to request from `pc`.
- Always update HANDOFF.md and PLAN_NOW.md before ending a session.

## ChatGPT vs Codex

- **Codex:** Can clone repos, run tests, push commits autonomously. Preferred for implementation.
- **ChatGPT:** Outputs must be manually committed by the human. Prefer structured markdown with explicit file paths so the human knows where to save each file.
