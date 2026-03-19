import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildKanbanCardModels,
  buildSlotCardModels,
  sortSlotCardModelsForFlatView,
  type SlotCardModel,
} from '../src/view-models';
import type { AgentPresence, FullStateSnapshot, RecentObserverAlert } from '../src/state';

test('buildSlotCardModels joins slot task, agent state, and alerts', () => {
  const snapshot = createSnapshot();
  const cards = buildSlotCardModels(snapshot, createAgents(), createAlerts());

  assert.equal(cards.length, 2);
  const slotCard = cards.find((card) => card.slot.id === 'task-a');
  assert.ok(slotCard);
  assert.equal(slotCard.task?.title, 'Implement grid');
  assert.equal(slotCard.status, 'TASK_STATUS_WORKING');
  assert.equal(slotCard.agentState, 'AGENT_STATE_EXECUTING');
  assert.equal(slotCard.alerts.length, 1);
});

test('buildKanbanCardModels joins project, slot, and agent details', () => {
  const snapshot = createSnapshot();
  const cards = buildKanbanCardModels(snapshot, createAgents(), 'proj-a', createAlerts());

  assert.equal(cards.length, 2);
  const boundCard = cards.find((card) => card.task.id === 'task-a');
  assert.ok(boundCard);
  assert.equal(boundCard.project?.displayName, 'Project A');
  assert.equal(boundCard.slot?.branch, 'agent/task-a');
  assert.equal(boundCard.agentId, 'agent-1');
  assert.equal(boundCard.agentState, 'AGENT_STATE_EXECUTING');
  assert.equal(boundCard.alerts.length, 1);

  const detachedCard = cards.find((card) => card.task.id === 'task-b');
  assert.ok(detachedCard);
  assert.equal(detachedCard.project?.displayName, 'Project A');
  assert.equal(detachedCard.slot, undefined);
  assert.equal(detachedCard.agentId, 'agent-2');
  assert.equal(detachedCard.agentState, 'AGENT_STATE_BLOCKED');
});

test('buildKanbanCardModels defaults to all projects', () => {
  const cards = buildKanbanCardModels(createSnapshot(), createAgents(), undefined, createAlerts());

  assert.equal(cards.length, 3);
  assert.ok(cards.some((card) => card.task.projectId === 'proj-b'));
});

test('sortSlotCardModelsForFlatView prioritizes active work then alert density', () => {
  const cards = sortSlotCardModelsForFlatView(createFlatCards());

  assert.deepEqual(
    cards.map((card) => card.slot.id),
    ['slot-working-alert', 'slot-working-calm', 'slot-review', 'slot-pending'],
  );
});

function createSnapshot(): FullStateSnapshot {
  return {
    projects: [
      {
        id: 'proj-a',
        displayName: 'Project A',
        repoPath: '/tmp/project-a',
        color: '#00bcd4',
        tags: [],
        budgetMaxUsd: 25,
        budgetWarnUsd: 10,
        currentCostUsd: 3.2,
        slots: [
          {
            id: 'task-a',
            projectId: 'proj-a',
            task: 'Implement grid',
            mode: 'AGENT_MODE_NORMAL',
            branch: 'agent/task-a',
            currentAgentId: 'agent-1',
            worktreeId: 'wt-a',
            totalTokens: 125,
            totalCostUsd: 0.82,
          },
        ],
      },
      {
        id: 'proj-b',
        displayName: 'Project B',
        repoPath: '/tmp/project-b',
        color: '#ffbf47',
        tags: [],
        budgetMaxUsd: 18,
        budgetWarnUsd: 9,
        currentCostUsd: 1.1,
        slots: [
          {
            id: 'task-c',
            projectId: 'proj-b',
            task: 'Handle alerts',
            mode: 'AGENT_MODE_PLAN',
            branch: 'agent/task-c',
            currentAgentId: 'agent-3',
            worktreeId: 'wt-c',
            totalTokens: 42,
            totalCostUsd: 0.31,
          },
        ],
      },
    ],
    taskDag: [
      {
        id: 'task-a',
        title: 'Implement grid',
        description: 'Render the grid shell',
        status: 'TASK_STATUS_WORKING',
        assignedAgentId: 'agent-1',
        projectId: 'proj-a',
        dependencyIds: [],
      },
      {
        id: 'task-b',
        title: 'Review interactions',
        description: 'Verify drag and drop',
        status: 'TASK_STATUS_REVIEW',
        assignedAgentId: 'agent-2',
        projectId: 'proj-a',
        dependencyIds: ['task-a'],
      },
      {
        id: 'task-c',
        title: 'Handle alerts',
        description: 'Render observer badges',
        status: 'TASK_STATUS_PENDING',
        assignedAgentId: 'agent-3',
        projectId: 'proj-b',
        dependencyIds: [],
      },
    ],
    totalSessionCost: 3.2,
    sessionBudgetMaxUsd: 25,
    lastEventSequence: 11,
  };
}

function createAgents(): AgentPresence[] {
  return [
    {
      agentId: 'agent-1',
      slotId: 'task-a',
      state: 'AGENT_STATE_EXECUTING',
    },
    {
      agentId: 'agent-2',
      slotId: 'task-b',
      state: 'AGENT_STATE_BLOCKED',
    },
    {
      agentId: 'agent-3',
      slotId: 'task-c',
      state: 'AGENT_STATE_IDLE',
    },
  ];
}

function createAlerts(): RecentObserverAlert[] {
  return [
    {
      eventId: 'evt-a',
      timestampMs: 100,
      eventSequence: 12,
      slotId: 'task-a',
      agentId: 'agent-1',
      loopDetected: {
        reason: 'repeated command',
        intervention: 'OBSERVER_INTERVENTION_PAUSE',
        findingKind: 'FINDING_KIND_LOOP_DETECTED',
      },
    },
  ];
}

function createFlatCards(): SlotCardModel[] {
  return [
    createFlatCard('slot-review', 'TASK_STATUS_REVIEW', 30, 0),
    createFlatCard('slot-working-alert', 'TASK_STATUS_WORKING', 120, 2),
    createFlatCard('slot-pending', 'TASK_STATUS_PENDING', 60, 0),
    createFlatCard('slot-working-calm', 'TASK_STATUS_WORKING', 90, 0),
  ];
}

function createFlatCard(
  id: string,
  status: SlotCardModel['status'],
  totalTokens: number,
  alertCount: number,
): SlotCardModel {
  return {
    project: {
      id: 'proj-flat',
      displayName: 'Flat Project',
      repoPath: '/tmp/flat',
      color: '#00bcd4',
      tags: [],
      budgetMaxUsd: 25,
      budgetWarnUsd: 10,
      currentCostUsd: 1,
      slots: [],
    },
    slot: {
      id,
      projectId: 'proj-flat',
      task: id,
      mode: 'AGENT_MODE_NORMAL',
      branch: `agent/${id}`,
      currentAgentId: `${id}-agent`,
      worktreeId: `wt-${id}`,
      totalTokens,
      totalCostUsd: 0.5,
    },
    task: {
      id,
      title: id,
      description: '',
      status,
      assignedAgentId: `${id}-agent`,
      projectId: 'proj-flat',
      dependencyIds: [],
    },
    status,
    agentState: 'AGENT_STATE_EXECUTING',
    alerts: Array.from({ length: alertCount }, (_, index) => ({
      eventId: `${id}-${index}`,
      timestampMs: 100 + index,
      eventSequence: index,
      slotId: id,
      agentId: `${id}-agent`,
      uncertaintySignal: {
        reason: 'needs review',
      },
    })),
  };
}
