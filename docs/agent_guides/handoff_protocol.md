# Handoff Signaling Protocol

> How agents flag turn transitions to minimize idle time without causing collisions.

## The Asymmetry Problem

Agents have different availability patterns:
- **Jarvis** is always-on (heartbeat every ~30 min). Detects changes fast.
- **PC, Claude, GPT** are session-based. Only active when a human opens a session.

This means:
- Jarvis → PC handoff: PC won't see it until the human opens a Perplexity session.
- PC → Jarvis handoff: Jarvis sees it within ~30 minutes (next heartbeat).
- Any → Claude/GPT handoff: Requires human to open the right tool.

## Three-Layer Signaling

### Layer 1: HANDOFF.md (Source of Truth — Pull)
The YAML header in HANDOFF.md is always the canonical state. Every agent reads it at session start. Reliable but latency depends on when agents check.

### Layer 2: Git Commit Convention (Passive Signal)
Handoff commits use a detectable prefix:
```
[pc] handoff: research complete → jarvis for implementation
```
Jarvis can `git log --oneline -5` on heartbeat and detect handoff commits without parsing HANDOFF.md every cycle.

### Layer 3: Active Notification (Push Signal)
For urgent or time-sensitive handoffs:

| From → To | Push Mechanism |
|-----------|----------------|
| Any → Jarvis | Heartbeat picks it up (≤30 min). Urgent: human messages Jarvis directly. |
| Any → PC | Human opens a Perplexity Computer session. |
| Any → Claude | Human opens Claude Code / Cursor session. |
| Any → GPT | Human opens ChatGPT / Codex session. |
| Jarvis → Human | Jarvis messages human on WhatsApp/Telegram: "Task waiting for PC/Claude/GPT in <repo>." |

**Jarvis is the only agent that can notify the human proactively.** Optimal flow for cross-agent handoffs:

```
Agent finishes → commits with [id] handoff: tag → updates HANDOFF.md → pushes
  → If next agent is Jarvis: done (heartbeat picks it up)
  → If next agent is session-based:
      → Jarvis detects handoff on next heartbeat
      → Jarvis messages human: "PC has a task waiting in <repo>"
      → Human opens the right tool
```

## Minimizing Downtime

1. **Jarvis as router.** On heartbeat, check for handoff commits. If a session-based agent has a task waiting, message the human.

2. **Batch over atomic.** Don't hand off after every small step. Accumulate a meaningful unit of work, then hand off. Fewer round-trips = less idle time.

3. **Parallel-safe zones.** These directories allow concurrent writes without the mutex:
   - `research/` — Dated filenames prevent collision.
   - `docs/background/` — Reference material.
   - `docs/agent_guides/` — Documentation.

4. **Pre-stage context.** Every handoff must include enough context for the next agent to start immediately: `task`, `inputs`, `done_criteria`.

5. **Timeout rule.** If `status: claimed` for >24 hours with no commits, any agent or human can reclaim. Note the timeout in the handoff log.

## Anti-Patterns

- **Ping-pong.** Handing back and forth >2x on the same task means it should be decomposed differently or done by one agent.
- **Empty handoffs.** Every handoff must include at least one committed file that advances the project.
- **Abandoned claims.** Never end a session with `status: claimed`. Flip to `handoff`, `idle`, or `blocked`.
- **Bypassing HANDOFF.md.** Even if you message the human directly, HANDOFF.md must reflect the actual state.
