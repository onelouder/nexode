# AGENTS.md

## Mission
Translate the Nexode specification into bounded engineering artifacts before implementation.

## Rules
- Do not implement code unless explicitly asked.
- Prefer producing markdown artifacts under docs/.
- Use exact section references from docs/spec/master-spec.md.
- Separate requirements, invariants, constraints, and design decisions.
- Surface contradictions instead of resolving them silently.
- Do not create tranches that cross multiple architectural seams unless justified.
- Every tranche must have dependencies, acceptance criteria, and rollback notes.

## Nexode architecture
- L1 daemon is source of truth.
- L2 gRPC is transport, not business logic owner.
- L3 shells are clients, not canonical state holders.
- AgentSlot persists across agent process failure.
- Worktree ownership, cost accrual, and recovery semantics are first-class.
