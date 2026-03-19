import assert from 'node:assert/strict';
import test from 'node:test';

import { buildKanbanCardModels, buildSlotCardModels } from '../src/view-models';
import type { AgentPresence, FullStateSnapshot } from '../src/state';

test('buildSlotCardModels joins slot task and agent state', () => {
  const snapshot = createSnapshot();
  const cards = buildSlotCardModels(snapshot, createAgents());

  assert.equal(cards.length, 1);
  const slotCard = cards.find((card) => card.slot.id === 'task-a');
  assert.ok(slotCard);
  assert.equal(slotCard.task?.title, 'Implement grid');
  assert.equal(slotCard.status, 'TASK_STATUS_WORKING');
  assert.equal(slotCard.agentState, 'AGENT_STATE_EXECUTING');
});

test('buildKanbanCardModels joins project, slot, and agent details', () => {
  const snapshot = createSnapshot();
  const cards = buildKanbanCardModels(snapshot, createAgents(), 'proj-a');

  assert.equal(cards.length, 2);
  const boundCard = cards.find((card) => card.task.id === 'task-a');
  assert.ok(boundCard);
  assert.equal(boundCard.project?.displayName, 'Project A');
  assert.equal(boundCard.slot?.branch, 'agent/task-a');
  assert.equal(boundCard.agentId, 'agent-1');
  assert.equal(boundCard.agentState, 'AGENT_STATE_EXECUTING');

  const detachedCard = cards.find((card) => card.task.id === 'task-b');
  assert.ok(detachedCard);
  assert.equal(detachedCard.project?.displayName, 'Project A');
  assert.equal(detachedCard.slot, undefined);
  assert.equal(detachedCard.agentId, 'agent-2');
  assert.equal(detachedCard.agentState, 'AGENT_STATE_BLOCKED');
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
  ];
}
