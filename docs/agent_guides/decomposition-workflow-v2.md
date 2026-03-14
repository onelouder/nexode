# Nexode Decomposition Workflow — Refined v2

> Status: Proposed  
> Date: 2026-03-14  
> Author: PC (Perplexity Computer)  
> Supersedes: Previous 10-prompt workflow draft  
> Context: Repo audit of `onelouder/nexode` @ commit `ac8c896`

---

## Current State Assessment

### What exists in the repo

| Artifact | Status | Quality Notes |
|---|---|---|
| `docs/spec/master-spec.md` | Present (1320 lines, v2) | Authoritative. Converted from PDF — has formatting artifacts (broken tables, merged cells). Needs a normalization pass before it can serve as a stable anchor. |
| `docs/spec/spec-outline.md` | Present (263 lines) | Solid. Stable section IDs assigned. Glossary of 60+ domain terms. 8 explicit contradictions cataloged. **Prompt 1 is effectively done.** |
| `docs/spec/requirements-extracted.md` | Present (269 lines) | Comprehensive extraction: ~120 requirements across Core, Architecture, Domain, Config, UI, Orchestration, Phases 0-5, Pools, and Licensing. Ambiguity notes inline. **Prompt 2 is effectively done.** |
| `AGENTS.md` | Present but generic | Describes the multi-agent collaboration protocol (Jarvis/PC/Claude/GPT), not the Nexode product architecture. Needs a Nexode-specific section for the decomposition pipeline. |
| `ARCHITECTURE.md` | Template only | Placeholder. Needs to be populated from spec. |
| `ROADMAP.md` | Template only | Placeholder. |
| `DECISIONS.md` | Template only | No decisions recorded. |
| `docs/architecture/` | Does not exist | Missing. Seam map, trust boundaries, and invariants have no home yet. |
| `docs/phases/` | Does not exist | Missing. Phase exit criteria and tranche indexes have no home yet. |
| `docs/tranches/` | Does not exist | Missing. |

### What the proposed workflow got right

1. The six-pass reduction pipeline (normalize → extract → seams → exits → tranches → harden) is sound.
2. Separating requirements from design, and design from implementation, is critical for Nexode given the hidden coupling.
3. The AGENTS.md layering strategy (root + per-directory) maps well to Codex's instruction model.
4. Prompts 1-2 produced high-quality output — the requirement extraction is thorough and honest about ambiguities.

### Conceptual gaps the proposed workflow does not address

| Gap | Why it matters | Consequence if ignored |
|---|---|---|
| **G-1: The master spec is a PDF-to-markdown conversion with broken formatting** | Tables render as merged text. Section headers are sometimes split across lines (e.g., `## Pha\nse`). The spec-outline already compensates, but invariant extraction will hallucinate if it reads broken source text directly. | Garbage-in at the invariant/seam passes. Silent misreads of table data. |
| **G-2: No contradiction resolution step before invariant extraction** | The spec-outline catalogs 8 explicit contradictions (e.g., `TaskStatus` vs. `Merged` column, `FullStateSnapshot` field mismatch, `manual` vs. `NORMAL` mapping). Prompt 3 (invariants) consumes these contradictions raw. | Invariants will either silently pick a side or produce contradictory invariants, both of which poison downstream tranche definitions. |
| **G-3: The 10-prompt workflow treats the AGENTS.md as a one-time write** | But the AGENTS.md in the repo is a collaboration protocol, not a product decomposition guide. There are actually two separate AGENTS.md concerns: (a) multi-agent collaboration rules, and (b) Nexode product architecture guardrails for Codex. | Codex/Cursor reads the existing AGENTS.md and follows handoff/session protocol instead of decomposition rules. |
| **G-4: No explicit "human review gate" between passes** | The workflow says "edit the markdown manually" but does not specify what the human is checking or what constitutes a blocking defect. | Human reviews become rubber stamps or infinite bikesheds. Neither is useful. |
| **G-5: The workflow assumes Cursor Plan Mode does all passes sequentially** | But Prompts 1-2 are already done by PC. The actual workflow is PC → Human → Cursor/Codex, with different agents owning different passes. Role assignment per pass is unspecified. | Agents overwrite each other's work. No one knows who owns the seam map vs. the tranche files. |
| **G-6: No spec freeze / version pin mechanism** | The spec is v2 and already incorporates errata. If the spec keeps changing while decomposition runs, every downstream artifact drifts. | Tranche definitions reference spec sections that no longer match. |
| **G-7: Missing directories in the repo** | `docs/architecture/`, `docs/phases/`, `docs/tranches/`, `docs/adr/` (empty) exist partially or not at all. The workflow assumes they exist before prompting. | Codex/Cursor creates files in wrong locations or fails to find expected paths. |
| **G-8: Deferred items are mixed into requirements** | 21 requirements are tagged `deferred` (Pools, Phase 4 memory, Phase 5 fork) but there is no `docs/spec/deferred.md` to quarantine them. | Tranche generation wastes cycles on items explicitly out of Phase 0-1 scope, or accidentally includes deferred scope. |

---

## Refined Workflow: Explicit Step-by-Step

### Overview

The refined workflow has **4 stages** instead of 10 sequential prompts. Each stage has a defined **owner**, **inputs**, **outputs**, a **human review gate**, and **done criteria**.

```
Stage 1: Stabilize Source (fix G-1, G-2, G-6, G-8)
Stage 2: Structural Analysis (Prompts 3-4 equivalent; fix G-3, G-5)
Stage 3: Phase & Tranche Planning (Prompts 5-8 equivalent; fix G-4)
Stage 4: Emit Executable Artifacts (Prompts 9-10 equivalent; fix G-7)
```

---

### Stage 1: Stabilize Source

**Goal:** Lock the spec, resolve contradictions, and quarantine deferred scope so every downstream pass has a clean, stable foundation.

**Owner:** PC (Perplexity Computer)  
**Estimated effort:** 1 session  

#### Step 1.1 — Normalize the master spec

| Field | Value |
|---|---|
| Input | `docs/spec/master-spec.md` (current, with PDF artifacts) |
| Action | Clean formatting: fix broken tables, split headers, normalize section numbering. Do NOT change content or wording. Tag each section with the stable ID from `spec-outline.md`. |
| Output | `docs/spec/master-spec.md` (overwrite, clean version) |
| Done when | Every table renders correctly in GitHub markdown preview. Every section header matches the `spec-outline.md` ID. Diff is formatting-only. |

#### Step 1.2 — Resolve contradictions

| Field | Value |
|---|---|
| Input | `spec-outline.md` section 4 (8 contradictions), `master-spec.md`, `requirements-extracted.md` |
| Action | For each contradiction, propose a resolution as a PROPOSED decision in `DECISIONS.md`. Format: `D-001` through `D-008`. Include the conflicting spec references, proposed resolution, and rationale. Do NOT modify the spec. |
| Output | `DECISIONS.md` with 8 proposed resolutions |
| Done when | Every contradiction from the outline has a corresponding D-NNN entry. |

**HUMAN REVIEW GATE 1:**  
Human reads `DECISIONS.md`, changes status of each to `ACCEPTED`, `MODIFIED`, or `REJECTED`. PC updates `requirements-extracted.md` to reflect accepted resolutions (append resolution notes to the ambiguity column, do not delete original notes).

#### Step 1.3 — Quarantine deferred items

| Field | Value |
|---|---|
| Input | `requirements-extracted.md` |
| Action | Extract all requirements tagged `deferred` into `docs/spec/deferred.md`. Keep them in the main file but add a cross-reference. Create `docs/spec/invariants.md` as an empty template. |
| Output | `docs/spec/deferred.md` |
| Done when | 21 deferred requirements are in the quarantine file with spec anchors preserved. |

#### Step 1.4 — Pin the spec version

| Field | Value |
|---|---|
| Input | Clean `master-spec.md` |
| Action | Add a front-matter block: `spec_version: 2.0.1-locked`, `locked_date: 2026-03-14`, `locked_sha: <commit>`. Add a rule to the root AGENTS.md: "Do not modify `docs/spec/master-spec.md` during decomposition. All amendments go to `DECISIONS.md`." |
| Output | Updated `master-spec.md` header, updated `AGENTS.md` |
| Done when | Commit pushed with `[pc] docs: lock spec for decomposition` message. |

---

### Stage 2: Structural Analysis

**Goal:** Extract invariants, trust boundaries, and the seam map. This is where hidden coupling becomes visible.

**Owner:** PC produces drafts. Cursor/Codex can assist with code-structural validation against the Rust skeleton.  
**Estimated effort:** 1-2 sessions  

#### Step 2.1 — Create missing directory structure

| Field | Value |
|---|---|
| Input | Repo root |
| Action | Create: `docs/architecture/`, `docs/phases/`, `docs/tranches/` with README stubs. |
| Output | Directories exist. `docs/tranches/README.md` describes the tranche file format. |

#### Step 2.2 — Extract invariants

| Field | Value |
|---|---|
| Input | `master-spec.md`, `requirements-extracted.md`, accepted `DECISIONS.md` |
| Action | Produce `docs/spec/invariants.md`. For each invariant: ID, spec anchor, invariant statement phrased as a falsifiable assertion, the domain entities involved, and the architectural layer that is the enforcement point. Categories: state ownership, slot/process semantics, repo/worktree ownership, crash recovery, cost accounting, UI/daemon authority boundaries. |
| Output | `docs/spec/invariants.md` |
| Done when | Every `invariant`-typed requirement from `requirements-extracted.md` appears. At least 5 additional invariants synthesized from cross-cutting concerns (e.g., "the daemon never blocks on UI acknowledgment" — implied but not stated as a single requirement). |

#### Step 2.3 — Map trust boundaries

| Field | Value |
|---|---|
| Input | `master-spec.md`, `invariants.md` |
| Action | Produce `docs/architecture/trust-boundaries.md`. For each boundary: name, attacker/failure model, what is trusted, what is not trusted, enforcement mechanism. Boundaries: CLI agent subprocesses, local filesystem/git, gRPC clients, VS Code webviews, secrets/API keys, remote daemon connections, inter-agent isolation (worktree sandboxing). |
| Output | `docs/architecture/trust-boundaries.md` |
| Done when | Every boundary has an explicit failure model. |

#### Step 2.4 — Build the seam map

| Field | Value |
|---|---|
| Input | `requirements-extracted.md`, `invariants.md` |
| Action | Produce `docs/architecture/seam-map.md`. For each non-deferred requirement: primary seam, secondary seams, source-of-truth component, contract surfaces touched, persistence impact, likely test level. Seam categories: substrate, daemon core, process harness, config/schema, gRPC/protobuf, persistence/recovery, observability, TUI, VS Code extension host, VS Code webview, memory/retrieval, packaging/licensing. |
| Output | `docs/architecture/seam-map.md` |
| Done when | Every non-deferred requirement has at least a primary seam assignment. Cross-seam requirements are flagged with a coupling risk note. |

**HUMAN REVIEW GATE 2:**  
Human reviews `invariants.md`, `trust-boundaries.md`, and `seam-map.md`. Key questions:
- Are any invariants wrong? (If yes, fix before proceeding — invariants propagate into every tranche.)
- Are any seam assignments surprising? (Surprising seams often indicate hidden coupling that the spec didn't make explicit.)
- Are any trust boundaries missing? (Especially around agent CLI stdout parsing — this is a real attack/failure surface.)

PC updates artifacts based on human feedback. Commit: `[pc] docs: structural analysis complete`.

---

### Stage 3: Phase and Tranche Planning

**Goal:** Define phase exit criteria and generate bounded, mergeable tranche definitions.

**Owner:** Cursor Plan Mode (primary), with PC review  
**Estimated effort:** 2-3 sessions  

#### Step 3.1 — Define phase exit criteria

| Field | Value |
|---|---|
| Input | `requirements-extracted.md`, `invariants.md`, `seam-map.md`, spec phase sections |
| Action | Produce `docs/phases/phase-exit-criteria.md`. For phases 0-5: goal, exit criteria (binary/measurable), non-goals, enabling dependencies, kill criteria. Use the existing spec phase definitions as the starting point — do not reinvent the roadmap. |
| Output | `docs/phases/phase-exit-criteria.md` |
| Done when | Every exit criterion is either a demo scenario or a CI-checkable assertion. |

#### Step 3.2 — Generate tranche candidates

| Field | Value |
|---|---|
| Input | All Stage 2 outputs + `phase-exit-criteria.md` |
| Action | Produce `docs/tranches/candidate-tranches.md`. For each phase, grouped by track: tranche ID, title, objective, requirement IDs covered, dependencies, risk (L/M/H), estimated span (0.5d/1d/2d/3d), dominant seam, validation mode, reasons the tranche might need splitting. Prefer thin vertical slices. Flag any tranche that touches daemon core + protobuf + UI simultaneously. |
| Output | `docs/tranches/candidate-tranches.md` |
| Done when | Phase 0 and Phase 1 exit criteria are fully covered by at least one tranche each. |

#### Step 3.3 — Stress-test and split oversized tranches

| Field | Value |
|---|---|
| Input | `candidate-tranches.md` |
| Action | For each tranche, test: >1 dominant seam? >1 public contract change? Cannot be validated independently? Unclear source of truth? No rollback path? >8-12 touched files? Split or rewrite any that fail. Produce `docs/tranches/candidate-tranches-v2.md` with rationale for every split. |
| Output | `docs/tranches/candidate-tranches-v2.md` |
| Done when | No tranche fails more than 1 of the 6 tests. |

**HUMAN REVIEW GATE 3:**  
Human reviews tranche candidates. Key questions:
- Can I explain what each tranche does in one sentence? (If not, it's too broad.)
- Does every tranche have a clear "how do I know it's done" test? (If not, acceptance criteria are weak.)
- Are there any tranches I'd be scared to merge? (Fear usually means the scope is too wide or the rollback is unclear.)

Approved tranches get promoted to individual files in Step 3.4.

#### Step 3.4 — Emit tranche definition files

| Field | Value |
|---|---|
| Input | `candidate-tranches-v2.md`, `invariants.md`, `phase-exit-criteria.md` |
| Action | Create one file per approved tranche: `docs/tranches/P{phase}-T{nn}-{slug}.md`. Structure: Objective, Spec anchors, Scope in/out, Architecture seam, Public contract changes, Data model impact, Invariants that must hold, Failure modes, Touched files (estimated), Validation plan, Acceptance criteria, Rollback path, Open questions. |
| Output | Individual tranche files |
| Done when | Every tranche from v2 has a file. Every file references exact requirement IDs. |

---

### Stage 4: Emit Executable Artifacts

**Goal:** Produce phase indexes and implementation prompts that Codex/Cursor can execute without re-discovering architecture.

**Owner:** PC (indexes), Cursor/Codex (implementation prompts)  
**Estimated effort:** 1 session  

#### Step 4.1 — Generate phase indexes

| Field | Value |
|---|---|
| Input | All tranche files |
| Action | Produce `docs/phases/phase-0.md` through `docs/phases/phase-5.md`. Each includes: phase goal, exit criteria, tranche table (with links), dependency graph, risk register, freeze points, unresolved decisions. |
| Output | Phase index files |
| Done when | Every tranche is listed in exactly one phase. Every exit criterion maps to at least one tranche. Uncovered criteria are explicitly flagged. |

#### Step 4.2 — Update AGENTS.md for implementation mode

| Field | Value |
|---|---|
| Input | Root `AGENTS.md`, all structural docs |
| Action | Add a `## Nexode Architecture` section to the root AGENTS.md with: layer model summary, key invariants (top 10), source-of-truth rules (daemon > gRPC > shell), and a rule that "every implementation PR must reference a tranche file." Add nested `.agents/codex-impl.md` for implementation-phase rules. |
| Output | Updated `AGENTS.md`, new `.agents/codex-impl.md` |
| Done when | Codex reads the file and can answer: "What layer owns this behavior?" for any requirement. |

#### Step 4.3 — Generate implementation prompts

| Field | Value |
|---|---|
| Input | Phase index, individual tranche file, `AGENTS.md` |
| Action | For each Phase 0 tranche, produce a standalone implementation prompt. Include: files to inspect first, scope boundaries, stop conditions, required tests, expected output. Keep prompts in `docs/tranches/prompts/P{phase}-T{nn}-impl.md`. |
| Output | Implementation prompt files |
| Done when | Each prompt is self-contained — an agent with no prior context can read the tranche file + prompt and produce a correct, bounded PR. |

**HUMAN REVIEW GATE 4:**  
Human spot-checks 2-3 implementation prompts by feeding them to Cursor Plan Mode and reviewing the proposed plan. If the plan matches expectations, the full set is approved.

Commit: `[pc] docs: decomposition pipeline complete, ready for Phase 0 implementation`.

---

## The Operating Loop (Post-Pipeline)

Once the pipeline is complete, the sprint loop is:

```
1. Human picks a tranche from the phase index
2. Cursor/Codex reads the tranche file + implementation prompt
3. Cursor/Codex produces a PR (feature branch: agent/claude/P0-T01-session-parser)
4. PC reviews the PR against:
   - Tranche acceptance criteria
   - Invariants that must hold
   - Scope boundaries (did it touch files outside the tranche?)
5. PC posts review on GitHub
6. Human merges or requests changes
7. Update PLAN_NOW.md and HANDOFF.md per session protocol
8. Repeat
```

---

## Recommended First Session Plan

Given that Prompts 1-2 are done, here is the concrete first-session checklist:

### Session 1 (Tonight — PC)
- [ ] Step 1.1: Normalize master-spec.md (fix broken tables/headers)
- [ ] Step 1.2: Write D-001 through D-008 contradiction resolutions in DECISIONS.md
- [ ] Step 1.3: Create `docs/spec/deferred.md`
- [ ] Step 1.4: Pin spec version in front-matter
- [ ] Step 2.1: Create missing directories
- [ ] Commit and push as a single PR for human review

### Session 2 (After human review of Session 1 — PC)
- [ ] Step 2.2: Extract invariants
- [ ] Step 2.3: Map trust boundaries
- [ ] Step 2.4: Build seam map
- [ ] Commit and push for human review

### Session 3 (After human review of Session 2 — Cursor Plan Mode)
- [ ] Step 3.1: Phase exit criteria
- [ ] Step 3.2: Generate tranche candidates
- [ ] Step 3.3: Stress-test and split

### Session 4 (After human review of Session 3 — PC + Cursor)
- [ ] Step 3.4: Emit tranche definition files
- [ ] Step 4.1: Phase indexes
- [ ] Step 4.2: Update AGENTS.md for implementation
- [ ] Step 4.3: Implementation prompts for Phase 0 only

**Total: 4 sessions to reach implementation-ready state for Phase 0.**

---

## Appendix: Prompt Templates (Updated)

These replace the original 10 prompts. They are numbered to match the steps above, not the original prompt sequence.

### Prompt 1.1 — Normalize Spec

```
Read docs/spec/master-spec.md and docs/spec/spec-outline.md.

Task:
Rewrite docs/spec/master-spec.md with these formatting fixes only:
1. Repair all broken tables (restore column alignment, merge split cells).
2. Fix split section headers (e.g., "## Pha\nse" → "## Phase").
3. Remove PDF page footers ("Nexode Agent IDE — Master Specification v2March 2026...").
4. Add the stable section ID from spec-outline.md as an HTML anchor above each heading.
5. Do NOT change content, wording, or organization.

Output: The cleaned master-spec.md, overwriting the current file.

Validation: Every table should render correctly in GitHub markdown preview.
```

### Prompt 1.2 — Resolve Contradictions

```
Read:
- docs/spec/spec-outline.md (section 4: contradictions)
- docs/spec/master-spec.md
- docs/spec/requirements-extracted.md

Task:
For each of the 8 cataloged contradictions, produce a D-NNN entry in DECISIONS.md.

Use this format:
## D-NNN: [Short title]
- **Date:** 2026-03-14
- **By:** pc
- **Status:** PROPOSED
- **Contradiction:** [Which spec sections conflict and how]
- **Option A:** [One resolution]
- **Option B:** [Alternative resolution]
- **Recommendation:** [Which option and why]
- **Impact on requirements:** [Which REQ-IDs are affected]

Rules:
- Do not modify the spec.
- Do not choose — propose. Human accepts.
- If a contradiction has a clear "the spec says X in the normative section
  and Y in an example," prefer the normative section.
```

### Prompt 2.2 — Extract Invariants

```
Read:
- docs/spec/master-spec.md
- docs/spec/requirements-extracted.md
- DECISIONS.md (accepted resolutions only)

Task:
Produce docs/spec/invariants.md.

For each invariant:
- Invariant ID (INV-NNN)
- Source: requirement ID(s) or synthesized from cross-cutting analysis
- Statement: A falsifiable assertion (e.g., "A slot's cost history MUST survive agent replacement")
- Enforcement point: Which layer/component enforces this
- Failure mode: What happens if violated
- Test strategy: How to verify

Categories:
1. State ownership (who owns what data)
2. Slot/process semantics (lifecycle, identity, replacement)
3. Repo/worktree ownership (isolation, cleanup)
4. Crash recovery (WAL, reattach, slot continuity)
5. Cost accounting (accrual, budget enforcement)
6. UI/daemon authority (who can mutate what)

Rules:
- Include all requirements typed as "invariant" in requirements-extracted.md.
- Synthesize additional invariants from cross-cutting concerns.
- Phrase as falsifiable assertions, not design goals.
- Do not include deferred (Phase 4/5) invariants unless they constrain Phase 0-1 design.
```

### Prompt 2.4 — Build Seam Map

```
Read:
- docs/spec/requirements-extracted.md
- docs/spec/invariants.md

Task:
Produce docs/architecture/seam-map.md.

For each non-deferred requirement ID:
| Req ID | Primary Seam | Secondary Seams | Source of Truth | Contract Surfaces | Persistence Impact | Test Level | Coupling Risk |

Seam categories:
- substrate (L0)
- daemon-core (L1 state machine)
- process-harness (L1 agent lifecycle)
- config-schema (L1 session/yaml)
- grpc-protobuf (L2 contract)
- persistence-recovery (L1 SQLite/WAL)
- observability (L1 events/telemetry)
- tui (L3 terminal)
- vscode-extension-host (L3 TS)
- vscode-webview (L3 React)
- memory-retrieval (L1 vector/AST, Phase 4)
- packaging-licensing (cross-cutting)

Rules:
- Requirements touching 3+ seams get a "coupling risk: HIGH" flag.
- Group the table by phase (Phase 0, Phase 1, Phase 2, etc.) so it doubles as a phase-scoped view.
- The primary seam is where the source of truth lives, not where the most code changes.
```
