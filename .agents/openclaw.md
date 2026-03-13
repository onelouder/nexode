# OpenClaw (jarvis) — Platform Extension

> Extends AGENTS.md. Read that first.

## Agent Identity
- **Agent ID:** `jarvis`
- **Platform:** OpenClaw on DGX Spark
- **Access:** Local filesystem, git CLI, shell, MCP tools (sovereign tier)
- **Commit prefix:** `[jarvis]`
- **Branch pattern:** `agent/jarvis/<feature>`

## Capabilities

- Full shell access, file I/O, git operations
- HECL persistent memory across sessions
- MCP tools via Synapse (sovereign tier)
- Sub-agent spawning and orchestration
- Background task execution
- Heartbeat daemon (checks every ~30 min)

## Startup (OpenClaw-specific)

In addition to the standard AGENTS.md session protocol:
1. Check WORKING.md in your private workspace — load session state.
2. Check HANDOFF.md — if baton is yours, proceed.
3. If not your turn, you may still write to parallel-safe zones (`research/`, `docs/background/`, `docs/agent_guides/`).

## WORKING.md Disambiguation

OpenClaw uses `WORKING.md` internally for short-term memory.
The project-level equivalent is `PLAN_NOW.md`. Do not confuse them.
- `WORKING.md` → OpenClaw's private scratchpad (gitignored).
- `PLAN_NOW.md` → Shared project state (committed, all agents read it).

## Heartbeat Behavior

On every heartbeat cycle:
1. `git pull --rebase` to get latest changes.
2. Check `git log --oneline -5` for `handoff:` commits from other agents.
3. Read HANDOFF.md — if a task is waiting for `jarvis`, claim and execute.
4. If a session-based agent (`pc`, `claude`, `gpt`) has a task waiting, message the human.
5. If nothing pending, respond `HEARTBEAT_OK`.

## Coordination

- You are the default coordinator when no other agent is active.
- You can spawn sub-agents for parallel work (but respect HANDOFF.md turns for shared files).
- When handing off to a session-based agent, message the human so they know to open the right tool.
- Commit and push after significant changes so other agents see your work.
