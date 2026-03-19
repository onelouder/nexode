import assert from 'node:assert/strict';
import test from 'node:test';

import {
  StateCache,
  coerceEnum,
  coerceNumber,
  coerceString,
  normalizeCommandResponse,
  normalizeEvent,
  normalizeSnapshot,
} from '../src/state';

test('coerce helpers handle malformed values', () => {
  assert.equal(coerceString('slot-a'), 'slot-a');
  assert.equal(coerceString(42), '');

  assert.equal(coerceNumber('12.5'), 12.5);
  assert.equal(coerceNumber(BigInt(9)), 9);
  assert.equal(coerceNumber('not-a-number'), 0);

  assert.equal(coerceEnum('beta', ['alpha', 'beta'] as const), 'beta');
  assert.equal(coerceEnum('gamma', ['alpha', 'beta'] as const), 'alpha');
});

test('normalizeSnapshot tolerates missing and malformed input', () => {
  const empty = normalizeSnapshot(undefined);
  assert.deepEqual(empty.projects, []);
  assert.deepEqual(empty.taskDag, []);
  assert.equal(empty.totalSessionCost, 0);
  assert.equal(empty.sessionBudgetMaxUsd, 0);
  assert.equal(empty.lastEventSequence, 0);

  const snapshot = normalizeSnapshot({
    projects: [
      {
        id: 'proj-a',
        displayName: 'Project A',
        repoPath: '/tmp/project-a',
        color: '#00bcd4',
        tags: ['ui', 42],
        budgetMaxUsd: '25',
        budgetWarnUsd: 10,
        currentCostUsd: '3.5',
        slots: [
          {
            id: 'slot-a',
            projectId: 'proj-a',
            task: 'Build shell',
            mode: 'AGENT_MODE_PLAN',
            branch: 'agent/slot-a',
            currentAgentId: 'agent-1',
            worktreeId: 'wt-a',
            totalTokens: '123',
            totalCostUsd: '0.42',
          },
        ],
      },
    ],
    taskDag: [
      {
        id: 'slot-a',
        title: 'Build shell',
        description: 'Add a shell',
        status: 'TASK_STATUS_WORKING',
        assignedAgentId: 'agent-1',
        projectId: 'proj-a',
        dependencyIds: ['slot-pre'],
      },
    ],
    totalSessionCost: '3.5',
    sessionBudgetMaxUsd: '100',
    lastEventSequence: '12',
  });

  assert.equal(snapshot.projects[0]?.tags[0], 'ui');
  assert.equal(snapshot.projects[0]?.tags[1], '');
  assert.equal(snapshot.projects[0]?.slots[0]?.totalTokens, 123);
  assert.equal(snapshot.taskDag[0]?.status, 'TASK_STATUS_WORKING');
  assert.equal(snapshot.totalSessionCost, 3.5);
  assert.equal(snapshot.lastEventSequence, 12);
});

test('normalizeEvent captures Phase 3 event variants', () => {
  const event = normalizeEvent({
    eventId: 'evt-1',
    timestampMs: '10',
    barrierId: 'bar-1',
    eventSequence: '33',
    uncertaintyFlag: {
      agentId: 'agent-2',
      taskId: 'slot-b',
      reason: 'loop',
    },
    worktreeStatusChanged: {
      worktreeId: 'wt-b',
      newRisk: '0.75',
    },
    observerAlert: {
      slotId: 'slot-b',
      agentId: 'agent-2',
      loopDetected: {
        reason: 'repeated command',
        intervention: 'OBSERVER_INTERVENTION_PAUSE',
        findingKind: 'FINDING_KIND_LOOP_DETECTED',
      },
    },
  });

  assert.equal(event.eventSequence, 33);
  assert.equal(event.uncertaintyFlag?.reason, 'loop');
  assert.equal(event.worktreeStatusChanged?.newRisk, 0.75);
  assert.equal(event.observerAlert?.loopDetected?.intervention, 'OBSERVER_INTERVENTION_PAUSE');
});

test('normalizeCommandResponse applies safe defaults', () => {
  const response = normalizeCommandResponse({
    success: true,
    commandId: 'cmd-1',
    outcome: 'COMMAND_OUTCOME_EXECUTED',
  });
  assert.equal(response.success, true);
  assert.equal(response.outcome, 'COMMAND_OUTCOME_EXECUTED');

  const fallback = normalizeCommandResponse({
    success: 1,
    errorMessage: 'bad',
    outcome: 'UNKNOWN_OUTCOME',
  });
  assert.equal(fallback.success, true);
  assert.equal(fallback.errorMessage, 'bad');
  assert.equal(fallback.outcome, 'COMMAND_OUTCOME_UNSPECIFIED');
});

test('StateCache applies snapshot and event mutations', () => {
  const cache = new StateCache();
  let changeCount = 0;
  const subscription = cache.onDidChange(() => {
    changeCount += 1;
  });

  cache.applySnapshot(
    normalizeSnapshot({
      projects: [
        {
          id: 'proj-a',
          displayName: 'Project A',
          repoPath: '/tmp/project-a',
          color: '#00bcd4',
          tags: [],
          budgetMaxUsd: 25,
          budgetWarnUsd: 10,
          currentCostUsd: 1,
          slots: [
            {
              id: 'slot-a',
              projectId: 'proj-a',
              task: 'Ship webview shell',
              mode: 'AGENT_MODE_PLAN',
              branch: 'agent/slot-a',
              currentAgentId: '',
              worktreeId: 'wt-a',
              totalTokens: 100,
              totalCostUsd: 0.42,
            },
          ],
        },
      ],
      taskDag: [
        {
          id: 'slot-a',
          title: 'Ship webview shell',
          description: 'Implement shell',
          status: 'TASK_STATUS_WORKING',
          assignedAgentId: '',
          projectId: 'proj-a',
          dependencyIds: [],
        },
      ],
      totalSessionCost: 1,
      sessionBudgetMaxUsd: 25,
      lastEventSequence: 7,
    }),
  );

  assert.deepEqual(cache.getAgentsBySlot('slot-a'), []);

  cache.applyEvent(
    normalizeEvent({
      eventSequence: 8,
      agentStateChanged: {
        agentId: 'agent-7',
        newState: 'AGENT_STATE_EXECUTING',
        slotId: 'slot-a',
      },
    }),
  );
  cache.applyEvent(
    normalizeEvent({
      eventSequence: 9,
      agentTelemetryUpdated: {
        agentId: 'agent-7',
        incrTokens: 25,
        tps: 2.5,
      },
    }),
  );
  cache.applyEvent(
    normalizeEvent({
      eventSequence: 10,
      taskStatusChanged: {
        taskId: 'slot-a',
        newStatus: 'TASK_STATUS_REVIEW',
        agentId: 'agent-7',
      },
    }),
  );
  cache.applyEvent(
    normalizeEvent({
      eventSequence: 11,
      projectBudgetAlert: {
        projectId: 'proj-a',
        currentUsd: 2.25,
        limitUsd: 10,
        hardKill: false,
      },
    }),
  );
  cache.applyEvent(
    normalizeEvent({
      eventSequence: 12,
      slotAgentSwapped: {
        slotId: 'slot-a',
        oldAgentId: 'agent-7',
        newAgentId: 'agent-9',
        reason: 'handoff',
      },
    }),
  );

  assert.equal(cache.getTaskStatusForSlot('slot-a'), 'TASK_STATUS_REVIEW');
  assert.equal(cache.getProjects()[0]?.slots[0]?.currentAgentId, 'agent-9');
  assert.equal(cache.getProjects()[0]?.slots[0]?.totalTokens, 125);
  assert.equal(cache.getProjects()[0]?.currentCostUsd, 2.25);
  assert.equal(cache.getAgentState('agent-7'), undefined);
  assert.equal(cache.getAgentState('agent-9'), 'AGENT_STATE_UNSPECIFIED');
  assert.deepEqual(cache.getAgentsBySlot('slot-a'), [
    {
      agentId: 'agent-9',
      slotId: 'slot-a',
      state: 'AGENT_STATE_UNSPECIFIED',
    },
  ]);
  assert.equal(cache.getAggregateMetrics().agentCount, 1);
  assert.equal(cache.getAggregateMetrics().totalTokens, 125);
  assert.equal(cache.getSnapshot().lastEventSequence, 12);
  assert.ok(changeCount >= 6);

  subscription.dispose();
  cache.dispose();
});

test('StateCache preserves seeded agent state across snapshots', () => {
  const cache = new StateCache();

  cache.applySnapshot(
    normalizeSnapshot({
      projects: [
        {
          id: 'proj-a',
          displayName: 'Project A',
          repoPath: '/tmp/project-a',
          color: '#00bcd4',
          tags: [],
          budgetMaxUsd: 25,
          budgetWarnUsd: 10,
          currentCostUsd: 1,
          slots: [
            {
              id: 'slot-a',
              projectId: 'proj-a',
              task: 'Ship webview shell',
              mode: 'AGENT_MODE_NORMAL',
              branch: 'agent/slot-a',
              currentAgentId: 'agent-1',
              worktreeId: 'wt-a',
              totalTokens: 100,
              totalCostUsd: 0.42,
            },
          ],
        },
      ],
      taskDag: [],
      totalSessionCost: 1,
      sessionBudgetMaxUsd: 25,
      lastEventSequence: 1,
    }),
  );

  assert.equal(cache.getAgentState('agent-1'), 'AGENT_STATE_UNSPECIFIED');

  cache.applyEvent(
    normalizeEvent({
      eventSequence: 2,
      agentStateChanged: {
        agentId: 'agent-1',
        newState: 'AGENT_STATE_EXECUTING',
        slotId: 'slot-a',
      },
    }),
  );
  cache.applySnapshot(
    normalizeSnapshot({
      projects: [
        {
          id: 'proj-a',
          displayName: 'Project A',
          repoPath: '/tmp/project-a',
          color: '#00bcd4',
          tags: [],
          budgetMaxUsd: 25,
          budgetWarnUsd: 10,
          currentCostUsd: 2,
          slots: [
            {
              id: 'slot-a',
              projectId: 'proj-a',
              task: 'Ship webview shell',
              mode: 'AGENT_MODE_NORMAL',
              branch: 'agent/slot-a',
              currentAgentId: 'agent-1',
              worktreeId: 'wt-a',
              totalTokens: 140,
              totalCostUsd: 0.63,
            },
          ],
        },
      ],
      taskDag: [],
      totalSessionCost: 2,
      sessionBudgetMaxUsd: 25,
      lastEventSequence: 3,
    }),
  );

  assert.equal(cache.getAgentState('agent-1'), 'AGENT_STATE_EXECUTING');
  cache.dispose();
});

test('StateCache records recent observer alerts and uncertainty flags', () => {
  const cache = new StateCache();

  cache.applyEvent(
    normalizeEvent({
      eventId: 'evt-1',
      timestampMs: 10,
      eventSequence: 1,
      observerAlert: {
        slotId: 'slot-a',
        agentId: 'agent-1',
        sandboxViolation: {
          path: '/tmp/outside',
          reason: 'outside worktree',
        },
      },
    }),
  );
  cache.applyEvent(
    normalizeEvent({
      eventId: 'evt-2',
      timestampMs: 20,
      eventSequence: 2,
      uncertaintyFlag: {
        agentId: 'agent-2',
        taskId: 'slot-b',
        reason: 'needs human input',
      },
    }),
  );

  assert.deepEqual(cache.getAlerts(), [
    {
      eventId: 'evt-2',
      timestampMs: 20,
      eventSequence: 2,
      slotId: 'slot-b',
      agentId: 'agent-2',
      uncertaintySignal: {
        reason: 'needs human input',
      },
    },
    {
      eventId: 'evt-1',
      timestampMs: 10,
      eventSequence: 1,
      slotId: 'slot-a',
      agentId: 'agent-1',
      sandboxViolation: {
        path: '/tmp/outside',
        reason: 'outside worktree',
      },
    },
  ]);

  cache.dispose();
});
