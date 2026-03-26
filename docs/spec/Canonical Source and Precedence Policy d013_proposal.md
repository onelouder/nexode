## D-013: Canonical Source Authority — Hybrid Rule

- **Date:** 2026-03-26
- **By:** jwells + pc
- **Status:** PROPOSED
- **Context:** After 12 sprints the codebase has grown well beyond the locked spec's proto schema and prose. The `hypervisor.proto` now contains 14 message/enum definitions that do not appear in `master-spec.md v2.0.1-locked` (e.g., `ObserverAlert`, `VerificationResult`, `AgentOutputLine`, `ResumeSlot`, `CommandResponse`). Some of these additions are covered by ACCEPTED decisions (D-006 → `SlotDispatch`, D-009 → `MERGE_QUEUE`/`RESOLVING` enum values, D-012 → `MoveTask` semantics); others emerged through sprint implementation without formal documentation. Meanwhile the locked spec remains accurate for architectural fundamentals (4-Layer model, domain hierarchy, session YAML schema, phase sequencing). No sprint has contradicted a clear spec statement — divergences have occurred where the spec was silent, ambiguous, or under-specified. The project needs an explicit governance rule so that all three agents (pc, gpt/claude, and the human reviewer) resolve conflicts consistently as Sprint 12-13 work increases complexity.
- **Decision:** The following four-tier authority hierarchy governs all spec-vs-implementation conflicts:

  **Tier 1 — Locked spec is baseline.**
  `docs/spec/master-spec.md` (v2.0.1-locked, 2026-03-14) is the architectural baseline. Any clear, unambiguous statement in the locked spec holds unless explicitly overridden by a Tier 2 decision. "Clear and unambiguous" means the spec defines a specific data type, field, enum value, state transition, or behavioral contract — not aspirational prose, phase roadmap sketches, or illustrative examples.

  **Tier 2 — ACCEPTED decisions override spec on their specific topic.**
  An ACCEPTED entry in `DECISIONS.md` supersedes locked spec text for the exact scope it addresses. The decision MUST cite what spec text it overrides or interprets and why. PROPOSED decisions (e.g., D-011) are non-binding and SHALL NOT constrain implementation choices until accepted.

  **Tier 3 — Implemented proto/code fills gaps where spec is silent.**
  Where the locked spec does not define a message, enum, field, event, or behavioral contract, and no ACCEPTED decision covers it, the implemented proto and Rust/TypeScript code is the current canonical definition. These gap-fills are valid but carry documentation debt. They SHOULD be retroactively cataloged in `docs/spec/extensions.md` (see Consequences below) and MAY be promoted to DECISIONS.md entries if they become architecturally significant.

  **Tier 4 — Sprint planning docs are tactical, not canonical.**
  `PLAN_NOW.md`, `HANDOFF.md`, sprint review docs, and `.agents/prompts/` are coordination artifacts. They describe intent and sequencing but do not establish architectural authority. If a sprint plan contradicts Tiers 1-3, the plan is wrong.

  **Conflict resolution procedure:**
  When an agent or reviewer encounters a spec-vs-implementation conflict:
  1. Check Tier 2: Is there an ACCEPTED decision on this topic? → Decision governs.
  2. Check Tier 1: Does the locked spec make a clear statement? → Spec governs; implementation must align or a new decision must be proposed to override.
  3. Neither applies: → Tier 3: current implementation governs; document the gap.

- **Rationale:** The project has been operating under this hierarchy implicitly since Sprint 1. D-001 through D-012 all follow the pattern: identify a spec gap or ambiguity, propose a resolution, accept it, and let implementation proceed. Formalizing the rule prevents three failure modes that become likely as sprint count increases: (a) an agent rolls back a valid proto extension because it doesn't appear in the locked spec; (b) a reviewer accepts a spec-contradicting change because "the code does it this way"; (c) planning documents are treated as spec amendments without going through the decision process. The four-tier hierarchy matches the project's actual governance practice while making the rules explicit for all agents.
- **Immediate Tier 3 gap inventory:** The following proto entities exist in `hypervisor.proto` as of `3a815ea` but are not defined in the locked spec and not fully covered by an existing ACCEPTED decision:

  | Entity | Sprint Origin | Covered By | Documentation Status |
  |---|---|---|---|
  | `SlotDispatch` | Sprint 1 | D-006 (ACCEPTED) | Fully documented |
  | `MERGE_QUEUE`, `RESOLVING` enum values | Sprint 1 | D-009 (ACCEPTED) | Fully documented |
  | `MoveTask` semantics | Sprint 10 | D-012 (ACCEPTED) | Fully documented |
  | `ObserverAlert` | Sprint 3 | Partial (D-011 is PROPOSED, not ACCEPTED) | Needs `extensions.md` entry |
  | `LoopDetected` | Sprint 3 | — | Needs `extensions.md` entry |
  | `SandboxViolation` | Sprint 3 | — | Needs `extensions.md` entry |
  | `UncertaintySignal` | Sprint 3 | — | Needs `extensions.md` entry |
  | `ObserverIntervention` (enum) | Sprint 3 | — | Needs `extensions.md` entry |
  | `FindingKind` (enum) | Sprint 3 | — | Needs `extensions.md` entry |
  | `VerificationResult` | Sprint 3 | — | Needs `extensions.md` entry |
  | `AgentOutputLine` | Sprint 11 | — | Needs `extensions.md` entry |
  | `ResumeSlot` | Sprint 10 | — | Needs `extensions.md` entry |
  | `CommandResponse` / `CommandOutcome` | Sprint 1 | — | Needs `extensions.md` entry |
  | `SubscribeRequest` / `StateRequest` | Sprint 1 | — | Needs `extensions.md` entry |

  None of these contradict locked spec text. All are gap-fills for contracts the spec anticipated in prose but did not formally define.

- **Consequences:**
  1. All agents (pc, gpt, claude) apply the four-tier hierarchy when evaluating spec-vs-implementation conflicts. No agent may reject a proto entity solely because it is absent from the locked spec — Tier 3 applies.
  2. `docs/spec/extensions.md` SHALL be created to catalog Tier 3 gap-fills. Each entry records: entity name, sprint of origin, purpose, and cross-reference to any related decision. This file is append-only and does not require the full decision format — it is a registry, not a decision log.
  3. The locked spec (`master-spec.md`) remains read-only. It is never edited. All evolution flows through DECISIONS.md (for architectural choices) and `extensions.md` (for gap-fill documentation).
  4. Future proto additions SHOULD be accompanied by either a DECISIONS.md entry (if they interpret, override, or extend spec text) or an `extensions.md` entry (if they fill a gap where the spec is silent). Undocumented additions are valid under Tier 3 but accumulate documentation debt.
  5. Questions Q2-Q7 from the gap report can now be resolved by applying this hierarchy directly: each question is either a Tier 1 vs Tier 3 conflict (requiring a new decision to override spec) or a Tier 3 gap-fill (requiring only documentation).
