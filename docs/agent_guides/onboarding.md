# Agent Onboarding Guide

This guide is for any AI agent working in this repository for the first time.

## First Session Checklist

1. Read `AGENTS.md` — the universal contract. Non-negotiable.
2. Read `PLAN_NOW.md` — understand what's active.
3. Read `HANDOFF.md` — check if there's a task waiting for you.
4. Read `.agents/<your-platform>.md` — platform-specific extensions.
5. Skim `ARCHITECTURE.md` — understand the system shape.
6. Check `research/` listing — know what research already exists.

## Before You Write Code or Research

- Check HANDOFF.md — is it your turn? Is anyone else working on the same area?
- Read the relevant `docs/agent_guides/` for the module or topic you're working on.
- Follow the commit and branching conventions in AGENTS.md.

## Before You End Your Session

- Update HANDOFF.md (flip status, describe what was done, list outputs).
- Update PLAN_NOW.md (reflect completed items, new blockers).
- Update CHANGELOG.md if the change is user-visible.
- Commit with your `[agent-id]` prefix.
- Push.

## If You're Stuck

- Set HANDOFF.md `status: blocked` with `blocked_on:` describing the issue.
- Don't silently fail or make assumptions — surface the problem.
- If you need another agent's help, name them in the handoff and describe what you need.

## Parallel-Safe Zones

You can write to these directories without holding the HANDOFF.md baton:
- `research/` (use dated filenames: `YYYY-MM-DD-slug.md`)
- `docs/background/`
- `docs/agent_guides/`

All other shared files require you to hold the baton.
