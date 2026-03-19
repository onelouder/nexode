import React, { useEffect, useState } from 'react';

import {
  buildSlotCardModels,
  sortSlotCardModelsForFlatView,
  type SlotCardModel,
} from '../../src/view-models';
import { onHostMessage, postReady } from '../shared/bridge';
import {
  agentTone,
  alertTone,
  formatAgentState,
  formatAlertKind,
  formatAlertMessage,
  formatAlertTime,
  formatCount,
  formatCurrency,
  formatMode,
  formatStatus,
  statusTone,
} from '../shared/format';
import type { StateEnvelope } from '../shared/types';

type SynapseSurface = 'synapse-grid' | 'synapse-sidebar';
type GridViewMode = 'groups' | 'flat' | 'focus';

const EMPTY_STATE: StateEnvelope = {
  surface: 'synapse-grid',
  connection: { state: 'disconnected' },
  snapshot: {
    projects: [],
    taskDag: [],
    totalSessionCost: 0,
    sessionBudgetMaxUsd: 0,
    lastEventSequence: 0,
  },
  agents: [],
  alerts: [],
  metrics: {
    agentCount: 0,
    totalTokens: 0,
    totalSessionCost: 0,
  },
  hasSnapshot: false,
};

export function SynapseGridApp({ surface }: { surface: SynapseSurface }): React.JSX.Element {
  const [state, setState] = useState<StateEnvelope>({
    ...EMPTY_STATE,
    surface,
  });
  const [viewMode, setViewMode] = useState<GridViewMode>('groups');
  const [focusProjectId, setFocusProjectId] = useState('');

  useEffect(() => {
    const dispose = onHostMessage((message) => {
      if (message.type === 'state') {
        setState(message.payload);
      }
    });

    postReady(surface);
    return dispose;
  }, [surface]);

  useEffect(() => {
    if (surface !== 'synapse-grid' || viewMode !== 'focus') {
      return;
    }

    const firstProjectId = state.snapshot.projects[0]?.id ?? '';
    const hasProject = state.snapshot.projects.some((project) => project.id === focusProjectId);
    if (!focusProjectId || !hasProject) {
      setFocusProjectId(firstProjectId);
    }
  }, [focusProjectId, state.snapshot.projects, surface, viewMode]);

  if (surface === 'synapse-sidebar') {
    return <SynapseSidebar state={state} />;
  }

  return (
    <SynapseGrid
      focusProjectId={focusProjectId}
      onFocusProjectChange={setFocusProjectId}
      onViewModeChange={setViewMode}
      state={state}
      viewMode={viewMode}
    />
  );
}

function SynapseGrid({
  state,
  viewMode,
  onViewModeChange,
  focusProjectId,
  onFocusProjectChange,
}: {
  state: StateEnvelope;
  viewMode: GridViewMode;
  onViewModeChange: (mode: GridViewMode) => void;
  focusProjectId: string;
  onFocusProjectChange: (projectId: string) => void;
}): React.JSX.Element {
  const slotCards = buildSlotCardModels(state.snapshot, state.agents, state.alerts);
  const flatCards = sortSlotCardModelsForFlatView(slotCards);
  const focusProject =
    state.snapshot.projects.find((project) => project.id === focusProjectId) ?? state.snapshot.projects[0];
  const focusCards = focusProject ? slotCards.filter((card) => card.project.id === focusProject.id) : [];

  return (
    <main className="surface surface-grid">
      <header className="hero">
        <div className="hero-copy">
          <p className="eyebrow">Phase 3 Shell</p>
          <h1>Synapse Grid</h1>
          <p className="hero-detail">
            {state.hasSnapshot
              ? `Sequence ${state.snapshot.lastEventSequence} across ${state.snapshot.projects.length} projects`
              : 'Connected shell waiting for the first daemon snapshot'}
          </p>
        </div>
        <div className="hero-side">
          <div className="hero-metrics">
            <Metric label="Connection" value={state.connection.state} />
            <Metric label="Agents" value={String(state.metrics.agentCount)} />
            <Metric label="Tokens" value={formatCount(state.metrics.totalTokens)} />
            <Metric label="Session Cost" value={formatCurrency(state.metrics.totalSessionCost)} />
          </div>
          <div className="hero-controls">
            <div className="view-switcher" role="tablist" aria-label="Synapse Grid view mode">
              <ViewModeButton active={viewMode === 'groups'} label="Project Groups" onClick={() => onViewModeChange('groups')} />
              <ViewModeButton active={viewMode === 'flat'} label="Flat View" onClick={() => onViewModeChange('flat')} />
              <ViewModeButton active={viewMode === 'focus'} label="Focus View" onClick={() => onViewModeChange('focus')} />
            </div>
            {viewMode === 'focus' ? (
              <label className="focus-select">
                <span>Project</span>
                <select
                  onChange={(event) => onFocusProjectChange(event.target.value)}
                  value={focusProject?.id ?? ''}
                >
                  {state.snapshot.projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.displayName}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </div>
        </div>
      </header>

      {state.alerts.length > 0 ? <RecentAlertsPanel state={state} /> : null}

      {state.snapshot.projects.length === 0 ? (
        <section className="project-list">
          <EmptyState
            title="Waiting for daemon state"
            detail="The webview shell is wired. Live project state will appear here once the daemon publishes a snapshot."
          />
        </section>
      ) : (
        renderBody(viewMode, state, slotCards, flatCards, focusProject, focusCards)
      )}
    </main>
  );
}

function SynapseSidebar({ state }: { state: StateEnvelope }): React.JSX.Element {
  const slotCards = buildSlotCardModels(state.snapshot, state.agents, state.alerts);

  return (
    <main className="surface surface-sidebar">
      <header className="sidebar-header">
        <p className="eyebrow">Synapse Sidebar</p>
        <h1>{state.connection.state === 'connected' ? 'Live Slots' : 'Disconnected'}</h1>
        <p className="sidebar-copy">
          {formatCount(state.metrics.totalTokens)} tokens · {state.metrics.agentCount} active agents
        </p>
      </header>
      <section className="sidebar-list">
        {slotCards.map((card) => (
          <article className="sidebar-item" key={card.slot.id}>
            <div>
              <p className="slot-id">{card.slot.id}</p>
              <h2>{card.task?.title || card.slot.task || 'Unassigned task'}</h2>
              <p className="sidebar-meta">
                {formatStatus(card.status)} · {card.slot.currentAgentId || 'idle'} · {formatCount(card.slot.totalTokens)} tok
              </p>
            </div>
            <div className="sidebar-chip-row">
              {card.alerts[0] ? (
                <span className="alert-pill" data-tone={alertTone(card.alerts[0])}>
                  {formatAlertKind(card.alerts[0])}
                </span>
              ) : null}
              <span className="status-pill" data-tone={statusTone(card.status)}>
                {formatStatus(card.status)}
              </span>
              <span className="state-pill" data-tone={agentTone(card.agentState)}>
                {formatAgentState(card.agentState)}
              </span>
            </div>
          </article>
        ))}
        {state.snapshot.projects.length === 0 ? (
          <EmptyState title="No slots yet" detail="The sidebar provider is registered and waiting for daemon state." />
        ) : null}
      </section>
    </main>
  );
}

function ViewModeButton({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}): React.JSX.Element {
  return (
    <button
      aria-pressed={active}
      className={`view-switch${active ? ' is-active' : ''}`}
      onClick={onClick}
      type="button"
    >
      {label}
    </button>
  );
}

function SlotCard({
  card,
  expanded = false,
}: {
  card: SlotCardModel;
  expanded?: boolean;
}): React.JSX.Element {
  const primaryAlert = card.alerts[0];

  return (
    <section className={`slot-card${expanded ? ' is-expanded' : ''}${card.alerts.length ? ' is-alerted' : ''}`}>
      <div className="slot-card-header">
        <p className="slot-id">{card.slot.id}</p>
        <span className="status-pill" data-tone={statusTone(card.status)}>
          {formatStatus(card.status)}
        </span>
      </div>
      <h3>{card.task?.title || card.slot.task || 'Unassigned task'}</h3>
      <p className="slot-summary">
        {card.slot.currentAgentId || 'idle'} on {card.slot.branch || 'no branch'}
      </p>
      <div className="pill-row">
        <span className="state-pill" data-tone={agentTone(card.agentState)}>
          {formatAgentState(card.agentState)}
        </span>
        <span className="state-pill" data-tone="neutral">
          {formatMode(card.slot.mode)}
        </span>
        {primaryAlert ? (
          <span className="alert-pill" data-tone={alertTone(primaryAlert)}>
            {formatAlertKind(primaryAlert)}
          </span>
        ) : null}
      </div>
      {primaryAlert ? <p className="slot-alert">{formatAlertMessage(primaryAlert)}</p> : null}
      <dl>
        <div>
          <dt>Status</dt>
          <dd>{card.task?.status ? formatStatus(card.task.status) : 'Untracked'}</dd>
        </div>
        <div>
          <dt>Tokens</dt>
          <dd>{formatCount(card.slot.totalTokens)}</dd>
        </div>
        <div>
          <dt>Cost</dt>
          <dd>{formatCurrency(card.slot.totalCostUsd)}</dd>
        </div>
        <div>
          <dt>Worktree</dt>
          <dd>{card.slot.worktreeId || '-'}</dd>
        </div>
      </dl>
      {expanded ? <ExpandedSlotDetails card={card} /> : null}
    </section>
  );
}

function ExpandedSlotDetails({ card }: { card: SlotCardModel }): React.JSX.Element {
  return (
    <div className="slot-expanded">
      {card.task?.description ? <p className="slot-description">{card.task.description}</p> : null}
      {card.task?.dependencyIds.length ? (
        <div className="dependency-list">
          {card.task.dependencyIds.map((dependencyId) => (
            <span className="dependency-chip" key={dependencyId}>
              {dependencyId}
            </span>
          ))}
        </div>
      ) : null}
      {card.alerts.length ? (
        <div className="slot-alert-list">
          {card.alerts.map((alert) => (
            <article className="alert-item" key={`${alert.eventSequence}-${alert.slotId}`}>
              <div>
                <p className="slot-id">{formatAlertKind(alert)}</p>
                <p className="slot-description">{formatAlertMessage(alert)}</p>
              </div>
              <span className="alert-time">{formatAlertTime(alert.timestampMs)}</span>
            </article>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }): React.JSX.Element {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function EmptyState({ title, detail }: { title: string; detail: string }): React.JSX.Element {
  return (
    <section className="empty-state">
      <h2>{title}</h2>
      <p>{detail}</p>
    </section>
  );
}

function RecentAlertsPanel({ state }: { state: StateEnvelope }): React.JSX.Element {
  return (
    <section className="alert-panel">
      <header className="alert-panel-header">
        <div>
          <p className="eyebrow">Observer Alerts</p>
          <h2>Recent findings</h2>
        </div>
        <p className="sidebar-copy">{state.alerts.length} recent alerts</p>
      </header>
      <div className="alert-list">
        {state.alerts.slice(0, 5).map((alert) => (
          <article className="alert-item" key={`${alert.eventSequence}-${alert.slotId}`}>
            <div>
              <div className="alert-chip-row">
                <span className="alert-pill" data-tone={alertTone(alert)}>
                  {formatAlertKind(alert)}
                </span>
                <span className="slot-id">{alert.slotId || 'unknown slot'}</span>
              </div>
              <p className="slot-description">{formatAlertMessage(alert)}</p>
            </div>
            <span className="alert-time">{formatAlertTime(alert.timestampMs)}</span>
          </article>
        ))}
      </div>
    </section>
  );
}

function renderBody(
  viewMode: GridViewMode,
  state: StateEnvelope,
  slotCards: SlotCardModel[],
  flatCards: SlotCardModel[],
  focusProject: StateEnvelope['snapshot']['projects'][number] | undefined,
  focusCards: SlotCardModel[],
): React.JSX.Element {
  if (viewMode === 'flat') {
    return (
      <section className="project-list">
        <section className="project-card">
          <header className="project-header">
            <div>
              <p className="project-tag">all-projects</p>
              <h2>Flat View</h2>
              <p className="project-meta">All slots sorted by activity priority and alert density.</p>
            </div>
            <p className="project-cost">{formatCount(flatCards.length)} slots</p>
          </header>
          <div className="slot-grid slot-grid-flat">
            {flatCards.map((card) => (
              <SlotCard card={card} key={card.slot.id} />
            ))}
          </div>
        </section>
      </section>
    );
  }

  if (viewMode === 'focus') {
    if (!focusProject) {
      return (
        <section className="project-list">
          <EmptyState title="No project selected" detail="Select a project to enter Focus View." />
        </section>
      );
    }

    return (
      <section className="project-list">
        <section className="focus-panel">
          <header className="focus-header">
            <div>
              <p className="project-tag">{focusProject.id}</p>
              <h2>{focusProject.displayName}</h2>
              <p className="project-meta">{focusProject.repoPath || 'Repository path unavailable'}</p>
            </div>
            <div className="project-summary">
              <p className="project-cost">{formatCurrency(focusProject.currentCostUsd)}</p>
              <p className="project-budget">
                {focusCards.length} slots · {focusCards.reduce((total, card) => total + card.alerts.length, 0)} alerts
              </p>
            </div>
          </header>
          <div className="slot-grid slot-grid-focus">
            {focusCards.map((card) => (
              <SlotCard card={card} expanded key={card.slot.id} />
            ))}
          </div>
        </section>
      </section>
    );
  }

  return (
    <section className="project-list">
      {state.snapshot.projects.map((project) => (
        <article className="project-card" key={project.id}>
          <header className="project-header">
            <div>
              <p className="project-tag">{project.id}</p>
              <h2>{project.displayName}</h2>
              <p className="project-meta">{project.repoPath || 'Repository path unavailable'}</p>
            </div>
            <div className="project-summary">
              <p className="project-cost">{formatCurrency(project.currentCostUsd)}</p>
              <p className="project-budget">
                Budget {formatCurrency(project.budgetWarnUsd)} / {formatCurrency(project.budgetMaxUsd)}
              </p>
            </div>
          </header>
          <div className="slot-grid">
            {slotCards
              .filter((card) => card.project.id === project.id)
              .map((card) => (
                <SlotCard card={card} key={card.slot.id} />
              ))}
          </div>
        </article>
      ))}
    </section>
  );
}
