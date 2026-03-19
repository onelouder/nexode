import React, { useEffect, useState } from 'react';

import { buildKanbanCardModels, type KanbanCardModel } from '../../src/view-models';
import { onHostMessage, postHostMessage, postReady } from '../shared/bridge';
import {
  agentTone,
  alertTone,
  formatAgentState,
  formatAlertKind,
  formatAlertMessage,
  formatCount,
  formatCurrency,
  statusTone,
} from '../shared/format';
import type { StateEnvelope } from '../shared/types';
import type { TaskStatusName } from '../../src/state';

const EMPTY_STATE: StateEnvelope = {
  surface: 'macro-kanban',
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

const COLUMNS: Array<{ status: TaskStatusName; label: string }> = [
  { status: 'TASK_STATUS_PENDING', label: 'Pending' },
  { status: 'TASK_STATUS_WORKING', label: 'Working' },
  { status: 'TASK_STATUS_REVIEW', label: 'Review' },
  { status: 'TASK_STATUS_MERGE_QUEUE', label: 'Merge Queue' },
  { status: 'TASK_STATUS_RESOLVING', label: 'Resolving' },
  { status: 'TASK_STATUS_DONE', label: 'Done' },
  { status: 'TASK_STATUS_PAUSED', label: 'Paused' },
  { status: 'TASK_STATUS_ARCHIVED', label: 'Archived' },
];

export function KanbanApp(): React.JSX.Element {
  const [state, setState] = useState<StateEnvelope>(EMPTY_STATE);
  const [projectFilter, setProjectFilter] = useState<string>('all');
  const [dragTaskId, setDragTaskId] = useState<string>('');
  const [dropTarget, setDropTarget] = useState<TaskStatusName | ''>('');

  useEffect(() => {
    const dispose = onHostMessage((message) => {
      if (message.type === 'state') {
        setState(message.payload);
      }
    });

    postReady('macro-kanban');
    return dispose;
  }, []);

  useEffect(() => {
    if (projectFilter === 'all') {
      return;
    }

    const hasProject = state.snapshot.projects.some((project) => project.id === projectFilter);
    if (!hasProject) {
      setProjectFilter('all');
    }
  }, [projectFilter, state.snapshot.projects]);

  const cards = buildKanbanCardModels(state.snapshot, state.agents, projectFilter, state.alerts);

  return (
    <main className="kanban-shell">
      <header className="kanban-header">
        <div className="header-copy">
          <p className="eyebrow">Phase 3 Shell</p>
          <h1>Macro Kanban</h1>
          <div className="header-metrics">
            <MetricChip label="Connection" value={state.connection.state} />
            <MetricChip label="Tasks" value={String(cards.length)} />
            <MetricChip label="Agents" value={String(state.metrics.agentCount)} />
            <MetricChip label="Session Cost" value={formatCurrency(state.metrics.totalSessionCost)} />
          </div>
        </div>
        <label className="filter">
          <span>Project</span>
          <select value={projectFilter} onChange={(event) => setProjectFilter(event.target.value)}>
            <option value="all">All projects</option>
            {state.snapshot.projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.displayName}
              </option>
            ))}
          </select>
        </label>
      </header>

      <section className="column-strip">
        {COLUMNS.map((column) => (
          <KanbanColumn
            key={column.status}
            label={column.label}
            status={column.status}
            cards={cards.filter((card) => card.task.status === column.status)}
            activeDropTarget={dropTarget}
            draggingTaskId={dragTaskId}
            onDragStart={(taskId) => {
              setDragTaskId(taskId);
              setDropTarget(column.status);
            }}
            onDragEnd={() => {
              setDragTaskId('');
              setDropTarget('');
            }}
            onDragOver={(target) => {
              if (!dragTaskId) {
                return;
              }
              setDropTarget(target);
            }}
            onDropTask={(taskId, target) => {
              setDragTaskId('');
              setDropTarget('');

              const task = cards.find((entry) => entry.task.id === taskId);
              if (!task || task.task.status === target) {
                return;
              }

              postHostMessage({
                type: 'moveTask',
                taskId,
                target,
              });
            }}
          />
        ))}
      </section>
    </main>
  );
}

function KanbanColumn({
  label,
  status,
  cards,
  activeDropTarget,
  draggingTaskId,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDropTask,
}: {
  label: string;
  status: TaskStatusName;
  cards: KanbanCardModel[];
  activeDropTarget: TaskStatusName | '';
  draggingTaskId: string;
  onDragStart: (taskId: string) => void;
  onDragEnd: () => void;
  onDragOver: (target: TaskStatusName) => void;
  onDropTask: (taskId: string, target: TaskStatusName) => void;
}): React.JSX.Element {
  return (
    <article className={`kanban-column${activeDropTarget === status ? ' is-active-drop' : ''}`}>
      <header className="column-header">
        <h2>{label}</h2>
        <span>{cards.length}</span>
      </header>
      <div
        className="column-body"
        onDragOver={(event) => {
          event.preventDefault();
          onDragOver(status);
        }}
        onDrop={(event) => {
          event.preventDefault();
          const taskId = draggingTaskId || event.dataTransfer.getData('text/plain');
          onDropTask(taskId, status);
        }}
      >
        {cards.length === 0 ? (
          <p className="empty-copy">No tasks</p>
        ) : (
          cards.map((card) => (
            <section
              className={`task-card${draggingTaskId === card.task.id ? ' is-dragging' : ''}${card.alerts.length ? ' is-alerted' : ''}`}
              draggable
              key={card.task.id}
              onDragStart={(event) => {
                event.dataTransfer.effectAllowed = 'move';
                event.dataTransfer.setData('text/plain', card.task.id);
                onDragStart(card.task.id);
              }}
              onDragEnd={onDragEnd}
            >
              <div className="card-header">
                <p className="task-id">{card.task.id}</p>
                <span className="chip" data-tone={statusTone(card.task.status)}>
                  {label}
                </span>
              </div>
              <h3>{card.task.title || 'Untitled task'}</h3>
              {card.task.description ? <p className="task-description">{card.task.description}</p> : null}
              <p className="task-meta">
                {card.project?.displayName || card.task.projectId || 'unscoped'} · {card.agentId || 'unassigned'}
              </p>
              <div className="card-chip-row">
                <span className="chip" data-tone={agentTone(card.agentState)}>
                  {formatAgentState(card.agentState)}
                </span>
                <span className="chip" data-tone="neutral">
                  {card.slot?.branch || 'no branch'}
                </span>
                {card.alerts[0] ? (
                  <span className="chip" data-tone={alertTone(card.alerts[0])}>
                    {formatAlertKind(card.alerts[0])}
                  </span>
                ) : null}
              </div>
              {card.alerts[0] ? <p className="task-alert">{formatAlertMessage(card.alerts[0])}</p> : null}
              <dl className="task-details">
                <div>
                  <dt>Tokens</dt>
                  <dd>{formatCount(card.slot?.totalTokens ?? 0)}</dd>
                </div>
                <div>
                  <dt>Cost</dt>
                  <dd>{formatCurrency(card.slot?.totalCostUsd ?? 0)}</dd>
                </div>
              </dl>
            </section>
          ))
        )}
      </div>
    </article>
  );
}

function MetricChip({ label, value }: { label: string; value: string }): React.JSX.Element {
  return (
    <div className="metric-chip">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
