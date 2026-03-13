# AGENTS.md — Universal Agent Contract

> This file is the single source of truth for all agents working on this project.
> Platform-specific extensions live in `.agents/`. They extend but **never override** this file.

## Project

- **Name:** [PROJECT_NAME]
- **Description:** [Brief description]
- **Owner:** [Human owner / org]
- **Repository:** [GitHub URL]

## Agent Roster

| Agent ID | Platform | Role | Primary Tasks |
|----------|----------|------|---------------|
| `jarvis` | OpenClaw (DGX Spark) | Coordinator / Implementer | Architecture, orchestration, shell ops, monitoring |
| `pc` | Perplexity Computer | Researcher / Analyst | Deep research, analysis, documents, financial data |
| `claude` | Claude Code / Cursor | Coder | Implementation, refactoring, code review |
| `gpt` | OpenAI Codex / ChatGPT | Coder | Implementation, testing, debugging |

## Agent Capabilities

| Capability | jarvis | pc | claude | gpt |
|------------|:------:|:--:|:------:|:---:|
| Shell / exec | ✅ | ❌ | ✅ | ✅ |
| Web search | ✅ | ✅ native | ❌ | ✅ |
| Academic search | ⚡️ | ✅ native | ❌ | ❌ |
| File read/write | ✅ | ✅ GitHub | ✅ | ✅ |
| Git operations | ✅ | ✅ GitHub | ✅ | ✅ |
| Deep research + citations | ⚡️ | ✅ primary | ❌ | ⚡️ |
| Code generation | ✅ | ⚡️ | ✅ primary | ✅ primary |
| Document creation (PDF/PPTX/XLSX) | ❌ | ✅ native | ❌ | ❌ |
| Financial data / market analysis | ❌ | ✅ native | ❌ | ❌ |
| Persistent memory | ✅ HECL | ❌ | ❌ | ❌ |
| MCP tools | ✅ sovereign | ⚡️ via connector | ❌ | ❌ |
| Test execution | ✅ | ❌ | ✅ | ✅ |
| Proactive / heartbeat | ✅ | ❌ | ❌ | ❌ |
| Parallel subagents | ⚡️ | ✅ native | ❌ | ❌ |
| Image / video generation | ❌ | ✅ native | ❌ | ✅ |

**Legend:** ✅ = full  ⚡️ = limited  ❌ = none

**Route work accordingly.** Research → `pc`. Coding/shell → `jarvis` or `claude`. Proactive monitoring → `jarvis`. Documents and reports → `pc`. Financial analysis → `pc`.

## Session Protocol

Every agent, every session, follows this four-phase cycle:

### Phase 1: Orient (read-only)
```
Read: AGENTS.md          → This file. Rules and capabilities.
Read: PLAN_NOW.md        → What's active right now.
Read: HANDOFF.md         → Who has the baton? Is there a task for me?
Read: .agents/<platform>.md → Platform-specific extensions.
```
If `PLAN_NOW.md` or `HANDOFF.md` is missing or empty, say so. Do not guess.

### Phase 2: Claim
If HANDOFF.md assigns work to you:
```yaml
agent: <your-id>
claimed: <ISO-8601 timestamp>
status: claimed
```
If HANDOFF.md is `idle` and you have work to do, claim it.

### Phase 3: Execute
Do the work. Follow the conventions below.

### Phase 4: Close (before ending your session)
```
1. Update HANDOFF.md   → Flip status to `handoff` or `idle`. Describe what was done.
                          Name the next agent if handing off. List output files.
2. Update PLAN_NOW.md  → Reflect completed items, new blockers, state changes.
3. Update CHANGELOG.md → If user-visible or API-visible changes were made.
4. Commit with message  → [agent-id] handoff: summary → next-agent (if handing off)
                          [agent-id] type: summary (if not handing off)
5. Push.
```
**Never end a session with `status: claimed` still set.** Flip to `handoff`, `idle`, or `blocked`.

## Handoff Signaling

See `docs/agent_guides/handoff_protocol.md` for the full signaling protocol. Summary:

- **Layer 1 (pull):** HANDOFF.md YAML header is the source of truth.
- **Layer 2 (passive):** Commit messages with `[agent-id] handoff:` prefix are machine-detectable.
- **Layer 3 (push):** Jarvis messages human on WhatsApp/Telegram when a session-based agent has a task waiting. Human opens the right tool.

### Handoff States

```yaml
status: idle          # No active work. Anyone can claim.
status: claimed       # Agent is actively working. Others: hands off shared files.
status: blocked       # Agent hit a wall. Needs input or another agent.
status: review        # Work done, needs review before merging.
status: handoff       # Work done, baton explicitly passed to next agent.
```

### Timeout Rule
If a handoff has been `claimed` for >24 hours with no commits, any agent or human can reclaim it.

## Git Rules

1. Always `git pull --rebase` before committing.
2. One agent writes to a file at a time — HANDOFF.md governs turns.
3. Commit messages: `[agent-id] type: description`
   - Types: `feat`, `fix`, `docs`, `refactor`, `test`, `research`, `chore`, `handoff`
   - Examples:
     ```
     [pc] research: vendor comparison for immersion cooling
     [jarvis] feat: add data ingestion pipeline
     [claude] refactor: extract auth module
     [gpt] test: add integration tests for payments
     [pc] handoff: research complete → jarvis for implementation
     ```
4. Never force-push to `main`.
5. Feature branches: `agent/<agent-id>/<feature>` (e.g., `agent/pc/cooling-research`).
6. PRs to `main` require at least one review (human or agent).
7. Squash merge preferred for feature branches.

## Parallel-Safe Zones

These directories are safe for concurrent writes (no mutex needed):
- `research/` — Dated filenames prevent collision.
- `docs/background/` — Reference material.
- `docs/agent_guides/` — Deep documentation.

All other shared files follow the HANDOFF.md turn protocol.

## Conflict Resolution

1. **File-level isolation preferred.** Agents work on different files when possible.
2. **HANDOFF.md is the mutex** for shared files listed in the task.
3. **Merge conflicts:** Describe in HANDOFF.md, set `status: blocked`, tag `blocked_on: merge-conflict`.
4. **Disagreements on approach:** Propose in DECISIONS.md as `PROPOSED`. Human or designated reviewer accepts.
5. **Humans are final authority.**

## Progressive Disclosure

Do NOT load everything every session. Read only what the current task requires:

| Context Needed | Read |
|---------------|------|
| Every session (mandatory) | `AGENTS.md`, `PLAN_NOW.md`, `HANDOFF.md` |
| Before structural changes | `ARCHITECTURE.md`, relevant `DECISIONS.md` entries |
| Before coding a module | Relevant source files + `docs/agent_guides/` for that area |
| Before research | `research/` listing (avoid duplicating prior work) |
| On demand only | `ROADMAP.md`, `CHANGELOG.md`, `docs/designs/` |

## Agent-Specific Configuration

Platform extensions live in `.agents/` and are read by the corresponding agent at startup:

```
.agents/
├── openclaw.md      → Jarvis: heartbeat behavior, WORKING.md disambiguation, MCP tools
├── perplexity.md    → PC: GitHub connector details, research output conventions
├── claude.md        → Claude Code / Cursor extensions (also mirrored as CLAUDE.md at root)
└── openai.md        → ChatGPT / Codex extensions
```

These files EXTEND this contract. They do not override it. If there's a conflict, AGENTS.md wins.
