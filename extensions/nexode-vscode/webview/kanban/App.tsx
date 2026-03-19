import React, { useEffect, useState } from 'react';

import { onHostMessage, postReady } from '../shared/bridge';
import type { StateEnvelope } from '../shared/types';
import type { TaskStatusName, TaskNode } from '../../src/state';

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

  useEffect(() => {
    const dispose = onHostMessage((message) => {
      if (message.type === 'state') {
        setState(message.payload);
      }
    });

    postReady('macro-kanban');
    return dispose;
  }, []);

  const tasks =
    projectFilter === 'all'
      ? state.snapshot.taskDag
      : state.snapshot.taskDag.filter((task) => task.projectId === projectFilter);

  return (
    <main className="kanban-shell">
      <header className="kanban-header">
        <div>
          <p className="eyebrow">Phase 3 Shell</p>
          <h1>Macro Kanban</h1>
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
            tasks={tasks.filter((task) => task.status === column.status)}
          />
        ))}
      </section>
    </main>
  );
}

function KanbanColumn({
  label,
  tasks,
}: {
  label: string;
  tasks: TaskNode[];
}): React.JSX.Element {
  return (
    <article className="kanban-column">
      <header className="column-header">
        <h2>{label}</h2>
        <span>{tasks.length}</span>
      </header>
      <div className="column-body">
        {tasks.length === 0 ? (
          <p className="empty-copy">No tasks</p>
        ) : (
          tasks.map((task) => (
            <section className="task-card" key={task.id}>
              <p className="task-id">{task.id}</p>
              <h3>{task.title || 'Untitled task'}</h3>
              <p className="task-meta">{task.projectId || 'unscoped'} · {task.assignedAgentId || 'unassigned'}</p>
            </section>
          ))
        )}
      </div>
    </article>
  );
}
