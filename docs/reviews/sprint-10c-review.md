# Sprint 10 Tranche C Code Review: View Modes, Shared Formatters, Observer Alerts

**Branch:** `agent/gpt/sprint-10c-view-modes`
**Reviewer:** pc (Perplexity Computer)
**Date:** 2026-03-19
**Commit reviewed:** `34dc817 [gpt] handoff: sprint 10 tranche c polish → pc`

---

## Summary

Sprint 10 Tranche C completes the Sprint 10 webview work with three focused deliverables: Synapse Grid view mode switching (Project Groups, Flat View, Focus View), shared formatter extraction, and observer alert rendering. The diff is +1055/-193 across 15 files — a clean polish and feature pass that closes both review follow-ups from Tranche B.

The most significant architectural addition is `webview/shared/format.ts` (138 lines), which extracts all formatter, tone, and alert-label utilities into a single shared module. Both Tranche B review findings are addressed: F-01 (duplicate formatters) is resolved by the extraction, and F-07 (`projectFilter = 'all'` test gap) is closed by a new test case in `view-models.test.ts`.

The Synapse Grid view mode implementation is well-structured. The `renderBody()` function dispatches to three distinct render paths based on `viewMode`, with Flat View using a dedicated `sortSlotCardModelsForFlatView()` sort function and Focus View adding an expanded card variant with dependency chips and per-slot alert history. The view mode switcher uses `aria-pressed` button semantics and a project selector dropdown that appears only in Focus mode. The `focusProjectId` state correctly auto-selects the first project and resets when the selected project disappears — same defensive pattern used in Kanban's project filter.

The observer alert buffering in `StateCache` uses a `pushAlert()` method with a 20-item rolling buffer (`MAX_RECENT_ALERTS`), prepending new alerts and slicing. Both `observerAlert` and `uncertaintyFlag` event types are captured, with `uncertaintyFlag` being normalized into the `RecentObserverAlert` shape with a synthesized `uncertaintySignal` sub-field. The `cloneRecentObserverAlert()` function correctly deep-clones the optional sub-objects (`loopDetected`, `sandboxViolation`, `uncertaintySignal`).

---

## Exit Criteria

| Criterion | Status | Notes |
|---|---|---|
| C-01: Synapse Grid view modes | PASS | Three modes implemented: Project Groups (existing, unchanged), Flat View (ungrouped, sorted by status priority → alert density → tokens → project name → slot ID), Focus View (single-project, expanded cards with description, dependencies, alert history). Header tab switcher with `ViewModeButton` and `aria-pressed`. Focus mode includes project selector dropdown. Auto-selection of first project. Stale project reset. |
| C-02: Shared formatter extraction | PASS | `webview/shared/format.ts` (138 lines): `formatCurrency`, `formatCount`, `toTitleWords`, `formatStatus`, `formatAgentState`, `formatMode`, `statusTone`, `agentTone`, `alertTone`, `formatAlertKind`, `formatAlertMessage`, `formatAlertTime`. Cached `Intl.NumberFormat` instances. Both React apps import from shared module. No remaining duplication. |
| C-03: Observer alert rendering | PASS | `StateCache.pushAlert()` with 20-item rolling buffer. `RecentObserverAlert` extends `ObserverAlertEvent` with `eventId`, `timestampMs`, `eventSequence`. Alerts joined into `SlotCardModel.alerts` and `KanbanCardModel.alerts` via `groupAlertsBySlot()`. Synapse Grid: `RecentAlertsPanel` (top-5, collapsible), alert pills on `SlotCard` and sidebar items, `.is-alerted` border highlight, `ExpandedSlotDetails` shows per-slot alert list. Kanban: alert chip in card chip row, `.is-alerted` border, alert message text. |
| C-04: Test coverage expansion | PASS | `format.test.ts`: 3 test cases (enum normalization, numeric/tone output, alert formatters). `state.test.ts`: +1 test case (recent observer alert buffer with both alert types, ordering verification). `view-models.test.ts`: updated existing tests to pass alerts, +2 new test cases (`buildKanbanCardModels` with `'all'` filter closing F-07, `sortSlotCardModelsForFlatView` priority ordering). Total: ~6 new test cases across 3 files. |
| Tranche B F-01 follow-up: Duplicate formatters | PASS | All duplicated functions extracted to `webview/shared/format.ts`. Both React apps now import from the shared module. |
| Tranche B F-07 follow-up: `projectFilter = 'all'` test | PASS | New test case `'buildKanbanCardModels defaults to all projects'` verifies the no-filter path returns tasks from multiple projects. |
| No Rust changes | PASS | Diff is TypeScript/TSX/CSS only. |
| CSP preserved | PASS | No changes to `webview-support.ts` CSP directives. Alert rendering uses class-based styling. |
| Build verification (agent-reported) | PASS | `npm run build`, `npm run build:webview`, `npm run check-types`, `npm test`, `cargo check --workspace`, `cargo test --workspace` all pass per handoff. |

---

## Verification Suite (agent-reported)

| Command | Result |
|---|---|
| `npm run build` | PASS |
| `npm run build:webview` | PASS |
| `npm run check-types` | PASS |
| `npm test` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |

**Test counts:** ~17 TypeScript test cases across 4 files (up from ~11 in Tranche B). 114 Rust tests unchanged.

---

## Findings

### F-01 [Info] `Intl.NumberFormat` instances cached at module scope — correct optimization

**Location:** `webview/shared/format.ts:3-10`

```typescript
const CURRENCY_FORMATTER = new Intl.NumberFormat('en-US', { ... });
const COUNT_FORMATTER = new Intl.NumberFormat('en-US');
```

The formatters are instantiated once at module load and reused across calls. This avoids the per-call allocation cost of `new Intl.NumberFormat()` that was present in the Tranche B inline versions. Good optimization — `Intl.NumberFormat` construction is expensive relative to the `format()` call.

**Recommendation:** No action.

### F-02 [Info] `alertTone` returns `'danger'` but Tranche B tone systems only had `'info'`, `'warn'`, `'success'`, `'muted'`, `'neutral'`

**Location:** `webview/shared/format.ts:76`

The new `alertTone()` function introduces a `'danger'` tone for sandbox violations. This is a new tone value that didn't exist in Tranche B's `statusTone`/`agentTone` systems. Both CSS files add corresponding `.alert-pill[data-tone='danger']` and `.chip[data-tone='danger']` rules with red border/text styling. The expansion is clean — `'danger'` only applies to alert pills, not to status or agent pills, which is semantically correct (sandbox violations are more severe than any task status).

**Recommendation:** No action. Well-designed tone expansion.

### F-03 [Low] `RecentAlertsPanel` renders a fixed `slice(0, 5)` — no expand/collapse toggle

**Location:** `webview/synapse-grid/App.tsx:355`

```typescript
{state.alerts.slice(0, 5).map((alert) => (
```

The alert panel shows at most 5 recent alerts with no way to see the remaining 15 in the 20-item buffer. The panel header shows the total count (`{state.alerts.length} recent alerts`) but the user can only see 5. This is a reasonable default for the initial implementation — the panel would become unwieldy with 20 items — but a toggle or "show more" link would be a natural future addition.

**Recommendation:** Not blocking. A future enhancement could add a `showAll` state toggle. The 5-item limit keeps the panel compact and focused on the most recent findings.

### F-04 [Info] `pushAlert` prepends new alerts, maintaining reverse-chronological order

**Location:** `state.ts:508-510`

```typescript
private pushAlert(alert: RecentObserverAlert): void {
  this.alerts = [cloneRecentObserverAlert(alert), ...this.alerts].slice(0, MAX_RECENT_ALERTS);
}
```

New alerts are prepended, so `this.alerts[0]` is always the most recent. The `slice(0, MAX_RECENT_ALERTS)` trims from the tail. This means the alert buffer maintains reverse-chronological order by insertion. The `RecentAlertsPanel` renders them in this order (newest first), and `card.alerts[0]` used for the primary alert pill on slot/task cards is the most recent alert for that slot. Correct.

The allocation pattern (spread into new array + slice) creates a new array on every event. At 20 items max this is negligible.

**Recommendation:** No action. Correct and performant for the scale.

### F-05 [Info] `uncertaintyFlag` to `RecentObserverAlert` normalization synthesizes `uncertaintySignal`

**Location:** `state.ts:392-401`

```typescript
} else if (event.uncertaintyFlag) {
  this.pushAlert({
    eventId: event.eventId,
    timestampMs: event.timestampMs,
    eventSequence: event.eventSequence,
    slotId: event.uncertaintyFlag.taskId,
    agentId: event.uncertaintyFlag.agentId,
    uncertaintySignal: {
      reason: event.uncertaintyFlag.reason,
    },
  });
}
```

The `uncertaintyFlag` event type uses `taskId` field while `observerAlert` uses `slotId`. The normalization correctly maps `uncertaintyFlag.taskId` → `slotId`. It also synthesizes an `uncertaintySignal` sub-object from the flat `reason` field, which allows `formatAlertKind()` and `formatAlertMessage()` to handle both event sources with the same code path. Good design.

**Recommendation:** No action. Clean normalization.

### F-06 [Info] `sortSlotCardModelsForFlatView` multi-key sort is well-ordered

**Location:** `view-models.ts:99-126`

The sort function applies 5 tiebreakers in priority order: (1) status priority (Working first, Done/Unspecified last), (2) alert density (more alerts first), (3) token count (higher first), (4) project display name (alphabetical), (5) slot ID (alphabetical). The test case validates the full sort chain: two WORKING cards are ordered by alert density (alert > calm), REVIEW comes after WORKING, PENDING comes after REVIEW.

The `FLAT_VIEW_STATUS_ORDER` array puts WORKING at index 0 and UNSPECIFIED at index 8, with `?? FLAT_VIEW_STATUS_ORDER.length` as the fallback for unknown statuses. This means any unrecognized status sorts to the very end.

**Recommendation:** No action. Well-designed multi-key sort.

### F-07 [Info] Focus View `ExpandedSlotDetails` uses compound key for alert list

**Location:** `webview/synapse-grid/App.tsx:312`

```typescript
key={`${alert.eventSequence}-${alert.slotId}`}
```

This compound key is unique assuming `eventSequence` is globally unique (which it is — it's a monotonic counter from the daemon). Using `eventSequence` alone would suffice, but including `slotId` is a safe defensive measure. The same pattern is used in `RecentAlertsPanel` at line 356.

**Recommendation:** No action.

### F-08 [Low] `MetricChip` component in Kanban is not extracted to shared module

**Location:** `webview/kanban/App.tsx:247-254`

The `MetricChip` component in Kanban and the `Metric` component in Synapse Grid serve the same purpose (label + value display) but have slightly different markup and class names. `MetricChip` uses `<div className="metric-chip">` with `<span>` + `<strong>`, while `Metric` uses `<div className="metric">` with `<span>` + `<strong>`. The structural difference is CSS class name only.

This is a minor residual duplication from Tranche B. The formatter extraction (C-02) focused on pure functions, not React components. Component sharing across webview surfaces would require a shared React component library, which is a larger architectural decision.

**Recommendation:** Not blocking. If a third surface is added, consider extracting shared presentation components to `webview/shared/components/`. For two surfaces, the duplication is tolerable.

---

## Architecture Assessment

### What's good

1. **Clean view mode dispatch.** The `renderBody()` function at the end of `App.tsx` cleanly dispatches to three render paths. Each mode gets its own layout: Groups renders project cards with slot grids, Flat renders all cards in a single grid with priority sorting, Focus renders a single project with expanded cards. The mode switcher uses `aria-pressed` button semantics for accessibility.

2. **Shared formatter module is comprehensive.** `format.ts` exports 12 functions covering formatting, tone mapping, and alert label/message extraction. The `Intl.NumberFormat` caching is a good optimization. The alert formatters (`alertTone`, `formatAlertKind`, `formatAlertMessage`, `formatAlertTime`) are well-structured — they use `Pick<RecentObserverAlert, ...>` parameter types so they can accept partial objects in tests.

3. **Alert data flow is end-to-end clean.** The chain is: daemon event → `StateCache.applyEvent()` → `pushAlert()` → `getAlerts()` → `createStateMessage()` → `StateEnvelope.alerts` → `groupAlertsBySlot()` → `SlotCardModel.alerts` / `KanbanCardModel.alerts` → React component rendering. Each step is testable and tested.

4. **Focus View is a meaningful addition.** It's not just a filter — it expands slot cards to show task description, dependency chips, and per-slot alert history. This is exactly the "expanded card detail" the spec called for. The auto-selection of the first project and stale-project reset match the Kanban's defensive project filter pattern.

5. **Flat View sort is well-considered.** Status priority → alert density → token count → alphabetical is a reasonable activity-based ordering. The test verifies the exact sort output for 4 cards with different status/alert combinations. The `FLAT_VIEW_STATUS_ORDER` constant is separate from `TASK_STATUSES` in state.ts because it has a different purpose (presentation priority vs enum completeness).

6. **Test coverage is well-targeted.** The new tests cover: (a) formatter correctness for enum normalization, currency formatting, and tone mapping; (b) alert buffer ordering and multi-type capture; (c) sort function priority chain; (d) the previously-missing `projectFilter = 'all'` path. The `createFlatCards()` helper is particularly well-constructed — it creates cards with controlled status, token, and alert count values to validate each tiebreaker.

### What's missing (expected post-Sprint 10)

1. **Rich per-cell presentation.** Spark-lines and progress bars were listed as a stretch goal. Not delivered — this is fine.

2. **Barrier-aware fan-out.** The webview does not participate in the event barrier protocol. Post-Sprint 10 scope.

3. **Chat Participant (`@nexode`).** Sprint 11+ scope.

4. **Merge Choreography TreeView.** Sprint 11+ scope.

5. **Tier 2 extension host tests.** Still deferred (R-011).

6. **Collapsible alert panel.** The `RecentAlertsPanel` is always visible when alerts exist, showing up to 5. No collapse/expand toggle. Minor UX gap.

---

## Spec Alignment Check

| PLAN_NOW Tranche C Scope | Delivered |
|---|---|
| C-01: Synapse Grid view modes | Yes — Project Groups, Flat View, Focus View with header controls, project selector, auto-selection |
| C-02: Shared formatter extraction | Yes — `webview/shared/format.ts`, 12 functions, both apps updated |
| C-03: Observer alert rendering | Yes — `StateCache` buffer, `StateEnvelope.alerts`, `SlotCardModel.alerts`, `KanbanCardModel.alerts`, `RecentAlertsPanel`, per-card alert pills/messages |
| C-04: Test coverage expansion | Yes — 6 new test cases across 3 files (format, alert buffer, sort, all-filter) |
| Tranche B F-01: Duplicate formatters | Yes — extracted to shared module |
| Tranche B F-07: `projectFilter = 'all'` test | Yes — new test case |
| No Rust changes | Yes — TypeScript/TSX/CSS only |
| CSP preserved | Yes — class-based alert styling, no inline styles |

The tranche delivers all scoped items and closes both Tranche B review follow-ups. The stretch goal (rich per-cell presentation) was not delivered, which is fine — it was explicitly optional.

---

## Sprint 10 Completion Assessment

With Tranche C merged, Sprint 10 is complete. Across three tranches:

| Tranche | PR | Lines | Key Deliverables |
|---|---|---|---|
| A | #22 | +2350/-33 | Webview build pipeline, panel shells, postMessage bridge, Emitter<T>, Phase 3 normalization, Tier 1 tests |
| B | #23 | +1086/-103 | Live rendering, drag-and-drop, agent tracking, view-models join layer |
| C | (pending) | +1055/-193 | View modes, shared formatters, observer alerts, expanded coverage |
| **Total** | | **+4491/-329** | |

Sprint 10 delivers everything in sec-11 "Weeks 2-4" except the Merge Choreography TreeView (explicitly deferred to Sprint 11+) and the Chat Participant (Sprint 11+ per spec). The extension codebase has grown from ~2800 to ~4900+ lines with ~17 Tier 1 test cases.

---

## Verdict

**APPROVED.** Sprint 10 Tranche C is a clean polish pass that completes the Sprint 10 webview work. The view mode implementation is well-structured, the shared formatter extraction addresses the Tranche B duplication finding, and the observer alert rendering provides end-to-end visibility into daemon observer findings. No findings above Low severity. The two Low findings (F-03 alert panel truncation, F-08 metric component duplication) are minor and do not affect correctness.

Sprint 10 is complete. The next sprint should focus on either the Merge Choreography TreeView or the Chat Participant — both are sec-11 "Weeks 5-8" deliverables.
