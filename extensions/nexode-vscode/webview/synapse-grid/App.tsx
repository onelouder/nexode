import React, { useEffect, useState } from 'react';

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
  return (
    <main className="surface surface-grid">
      <header className="hero">
        <div>
          <p className="eyebrow">Phase 3 Shell</p>
          <h1>Synapse Grid</h1>
        </div>
        <div className="hero-metrics">
          <Metric label="Connection" value={state.connection.state} />
          <Metric label="Projects" value={String(state.snapshot.projects.length)} />
          <Metric label="Tasks" value={String(state.snapshot.taskDag.length)} />
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
                </div>
                <p className="project-cost">${project.currentCostUsd.toFixed(2)}</p>
              </header>
              <div className="slot-grid">
                {project.slots.map((slot) => (
                  <section className="slot-card" key={slot.id}>
                    <p className="slot-id">{slot.id}</p>
                    <h3>{slot.task || 'Unassigned task'}</h3>
                    <dl>
                      <div>
                        <dt>Agent</dt>
                        <dd>{slot.currentAgentId || 'idle'}</dd>
                      </div>
                      <div>
                        <dt>Branch</dt>
                        <dd>{slot.branch || '-'}</dd>
                      </div>
                      <div>
                        <dt>Tokens</dt>
                        <dd>{slot.totalTokens.toLocaleString('en-US')}</dd>
                      </div>
                      <div>
                        <dt>Cost</dt>
                        <dd>${slot.totalCostUsd.toFixed(2)}</dd>
                      </div>
                    </dl>
                  </section>
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
  return (
    <main className="surface surface-sidebar">
      <header className="sidebar-header">
        <p className="eyebrow">Synapse Sidebar</p>
        <h1>{state.connection.state === 'connected' ? 'Live Slots' : 'Disconnected'}</h1>
      </header>
      <section className="sidebar-list">
        {state.snapshot.projects.flatMap((project) =>
          project.slots.map((slot) => (
            <article className="sidebar-item" key={slot.id}>
              <div>
                <p className="slot-id">{slot.id}</p>
                <h2>{slot.task || 'Unassigned task'}</h2>
              </div>
              <p className="sidebar-status">{slot.currentAgentId || 'idle'}</p>
            </article>
          )),
        )}
        {state.snapshot.projects.length === 0 ? (
          <EmptyState title="No slots yet" detail="The sidebar provider is registered and waiting for daemon state." />
        ) : null}
      </section>
    </main>
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
