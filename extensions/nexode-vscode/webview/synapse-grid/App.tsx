import React, { useEffect, useState } from 'react';

import { buildSlotCardModels, type SlotCardModel } from '../../src/view-models';
import { onHostMessage, postReady } from '../shared/bridge';
import type { StateEnvelope } from '../shared/types';

type SynapseSurface = 'synapse-grid' | 'synapse-sidebar';

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

  useEffect(() => {
    const dispose = onHostMessage((message) => {
      if (message.type === 'state') {
        setState(message.payload);
      }
    });

    postReady(surface);
    return dispose;
  }, [surface]);

  if (surface === 'synapse-sidebar') {
    return <SynapseSidebar state={state} />;
  }

  return <SynapseGrid state={state} />;
}

function SynapseGrid({ state }: { state: StateEnvelope }): React.JSX.Element {
  const slotCards = buildSlotCardModels(state.snapshot, state.agents);

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
        <div className="hero-metrics">
          <Metric label="Connection" value={state.connection.state} />
          <Metric label="Agents" value={String(state.metrics.agentCount)} />
          <Metric label="Tokens" value={formatCount(state.metrics.totalTokens)} />
          <Metric label="Session Cost" value={formatCurrency(state.metrics.totalSessionCost)} />
        </div>
      </header>

      <section className="project-list">
        {state.snapshot.projects.length === 0 ? (
          <EmptyState title="Waiting for daemon state" detail="The webview shell is wired. Live project state will appear here once the daemon publishes a snapshot." />
        ) : (
          state.snapshot.projects.map((project) => (
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
          ))
        )}
      </section>
    </main>
  );
}

function SynapseSidebar({ state }: { state: StateEnvelope }): React.JSX.Element {
  const slotCards = buildSlotCardModels(state.snapshot, state.agents);

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

function SlotCard({ card }: { card: SlotCardModel }): React.JSX.Element {
  return (
    <section className="slot-card">
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
      </div>
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
    </section>
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

function formatStatus(status: string): string {
  return toTitleWords(status.replace(/^TASK_STATUS_/, ''));
}

function formatAgentState(state: string): string {
  return toTitleWords(state.replace(/^AGENT_STATE_/, ''));
}

function formatMode(mode: string): string {
  return toTitleWords(mode.replace(/^AGENT_MODE_/, ''));
}

function formatCurrency(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatCount(value: number): string {
  return new Intl.NumberFormat('en-US').format(value);
}

function toTitleWords(value: string): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((segment) => segment.charAt(0) + segment.slice(1).toLowerCase())
    .join(' ');
}

function statusTone(status: SlotCardModel['status']): string {
  switch (status) {
    case 'TASK_STATUS_WORKING':
      return 'info';
    case 'TASK_STATUS_REVIEW':
    case 'TASK_STATUS_MERGE_QUEUE':
    case 'TASK_STATUS_RESOLVING':
      return 'warn';
    case 'TASK_STATUS_DONE':
      return 'success';
    case 'TASK_STATUS_PAUSED':
    case 'TASK_STATUS_ARCHIVED':
      return 'muted';
    default:
      return 'neutral';
  }
}

function agentTone(state: SlotCardModel['agentState']): string {
  switch (state) {
    case 'AGENT_STATE_EXECUTING':
      return 'info';
    case 'AGENT_STATE_REVIEW':
    case 'AGENT_STATE_PLANNING':
      return 'warn';
    case 'AGENT_STATE_IDLE':
      return 'success';
    case 'AGENT_STATE_BLOCKED':
    case 'AGENT_STATE_TERMINATED':
      return 'muted';
    default:
      return 'neutral';
  }
}
