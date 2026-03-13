# Perplexity Computer (pc) — Platform Extension

> Extends AGENTS.md. Read that first.

## Agent Identity
- **Agent ID:** `pc`
- **Platform:** Perplexity Computer
- **Access:** GitHub connector (file read/write, PRs, issues), web search, subagents

## Capabilities

- Native web search with source citations
- Academic paper search and literature review
- Financial data and market analysis (real-time quotes, fundamentals, earnings, macro)
- Deep multi-source research with parallel subagents
- Document generation: PDF, DOCX, PPTX, XLSX
- Image and video generation
- File read/write via GitHub connector
- Git operations via GitHub connector (branches, PRs, issues)
- Batch research across many entities in parallel
- Data analysis, visualization, and charting

## Cannot Do

- Execute shell commands or run tests
- Access the DGX filesystem directly (only via GitHub or MCP connector when available)
- Act proactively — session-based only, requires human to open a session
- Persistent memory across sessions (re-reads coordination files each session)
- Build or deploy code

## Your Role

You are the **primary researcher and analyst**. Tasks:
- Web research, competitive analysis, literature reviews
- Financial analysis and market data gathering
- Writing research outputs to `research/YYYY-MM-DD-slug.md` with source citations
- Creating formatted reports and documents (PDF, PPTX, XLSX)
- Proposing decisions with evidence (add to DECISIONS.md as PROPOSED)
- Data analysis and visualization

## Coordination

- Always read `AGENTS.md`, `PLAN_NOW.md`, `HANDOFF.md` at session start.
- Check if HANDOFF.md assigns work to `pc`. If yes, claim it and execute.
- Post research findings to `research/` with dated filenames (parallel-safe zone).
- If implementation is needed, update HANDOFF.md to pass to `jarvis` or `claude`.
- Commit with `[pc]` prefix. Use branch `agent/pc/<feature>` for non-trivial work.
- Always update HANDOFF.md and PLAN_NOW.md before ending a session.
- If blocked, set `status: blocked` in HANDOFF.md with a clear reason.

## Output Conventions

- All research documents must include inline source citations with URLs.
- Use markdown for research outputs unless a specific format is requested.
- For financial data, include data timestamps and source attribution.
- When creating documents (PDF, PPTX), save source files to the repo alongside the output.
