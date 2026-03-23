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
  type AgentOutputLine,
  type VerificationResultEvent,
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
            worktreePath: '/tmp/worktrees/wt-a',
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
  assert.equal(snapshot.projects[0]?.slots[0]?.worktreePath, '/tmp/worktrees/wt-a');
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
              worktreePath: '',
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

test('normalizeEvent handles agentOutputLine payload', () => {
  const event = normalizeEvent({
    eventId: 'evt-out-1',
    timestampMs: '500',
    barrierId: '',
    eventSequence: '42',
    agentOutputLine: {
      slotId: 'slot-a',
      agentId: 'agent-1',
      stream: 'stderr',
      line: 'error: something went wrong',
      timestampMs: '499',
    },
  });

  assert.equal(event.eventId, 'evt-out-1');
  assert.equal(event.eventSequence, 42);
  assert.ok(event.agentOutputLine);
  assert.equal(event.agentOutputLine.slotId, 'slot-a');
  assert.equal(event.agentOutputLine.agentId, 'agent-1');
  assert.equal(event.agentOutputLine.stream, 'stderr');
  assert.equal(event.agentOutputLine.line, 'error: something went wrong');
  assert.equal(event.agentOutputLine.timestampMs, 499);
});

test('normalizeEvent defaults agentOutputLine stream to stdout for unknown values', () => {
  const event = normalizeEvent({
    eventId: 'evt-out-2',
    timestampMs: '600',
    eventSequence: '43',
    agentOutputLine: {
      slotId: 'slot-b',
      agentId: 'agent-2',
      stream: 'invalid-stream',
      line: 'hello world',
      timestampMs: '599',
    },
  });

  assert.equal(event.agentOutputLine?.stream, 'stdout');
});

test('agentOutputLine events do not affect StateCache stored state', () => {
  const cache = new StateCache();
  let changeCount = 0;
  const subscription = cache.onDidChange(() => {
    changeCount += 1;
  });

  // Apply an agentOutputLine event via applyEvent — it fires changeEmitter
  // but does not modify projects, taskDag, agents, or alerts.
  // In production, DaemonClient intercepts agentOutputLine events and
  // routes them to the output emitter instead of calling applyEvent.
  cache.applyEvent(
    normalizeEvent({
      eventId: 'evt-out-3',
      timestampMs: 700,
      eventSequence: 1,
      agentOutputLine: {
        slotId: 'slot-a',
        agentId: 'agent-1',
        stream: 'stdout',
        line: 'some output',
        timestampMs: 699,
      },
    }),
  );

  assert.equal(changeCount, 1);
  assert.deepEqual(cache.getProjects(), []);
  assert.deepEqual(cache.getAgentStates(), []);
  assert.deepEqual(cache.getAlerts(), []);

  subscription.dispose();
  cache.dispose();
});

test('normalizeEvent handles verificationResult payload', () => {
  const event = normalizeEvent({
    eventId: 'evt-vr-1',
    timestampMs: '800',
    barrierId: '',
    eventSequence: '50',
    verificationResult: {
      slotId: 'slot-a',
      projectId: 'proj-a',
      success: false,
      step: 'cargo build',
      command: 'cargo build --release',
      statusCode: 1,
      stdout: 'Compiling...',
      stderr: 'error[E0308]: mismatched types\n  --> src/main.rs:42:5',
    },
  });

  assert.equal(event.eventId, 'evt-vr-1');
  assert.equal(event.eventSequence, 50);
  assert.ok(event.verificationResult);
  assert.equal(event.verificationResult.slotId, 'slot-a');
  assert.equal(event.verificationResult.projectId, 'proj-a');
  assert.equal(event.verificationResult.success, false);
  assert.equal(event.verificationResult.step, 'cargo build');
  assert.equal(event.verificationResult.command, 'cargo build --release');
  assert.equal(event.verificationResult.statusCode, 1);
  assert.equal(event.verificationResult.stdout, 'Compiling...');
  assert.ok(event.verificationResult.stderr.includes('error[E0308]'));
});

test('StateCache stores and retrieves verification results', () => {
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
              task: 'Build shell',
              mode: 'AGENT_MODE_PLAN',
              branch: 'agent/slot-a',
              currentAgentId: 'agent-1',
              worktreeId: 'wt-a',
              totalTokens: 100,
              totalCostUsd: 0.42,
              worktreePath: '/tmp/worktrees/wt-a',
            },
          ],
        },
      ],
      taskDag: [
        {
          id: 'slot-a',
          title: 'Build shell',
          description: 'Build',
          status: 'TASK_STATUS_REVIEW',
          assignedAgentId: 'agent-1',
          projectId: 'proj-a',
          dependencyIds: [],
        },
      ],
      totalSessionCost: 1,
      sessionBudgetMaxUsd: 25,
      lastEventSequence: 1,
    }),
  );

  cache.applyEvent(
    normalizeEvent({
      eventId: 'evt-vr-2',
      timestampMs: 900,
      eventSequence: 2,
      verificationResult: {
        slotId: 'slot-a',
        projectId: 'proj-a',
        success: false,
        step: 'cargo test',
        command: 'cargo test',
        statusCode: 101,
        stdout: '',
        stderr: 'test failed',
      },
    }),
  );

  const result = cache.getVerificationResult('slot-a');
  assert.ok(result);
  assert.equal(result.slotId, 'slot-a');
  assert.equal(result.success, false);
  assert.equal(result.statusCode, 101);
  assert.equal(result.stderr, 'test failed');

  const allResults = cache.getVerificationResults();
  assert.equal(allResults.size, 1);
  assert.ok(allResults.has('slot-a'));

  cache.dispose();
});

test('StateCache clears verification result when task transitions to WORKING', () => {
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
              task: 'Build shell',
              mode: 'AGENT_MODE_PLAN',
              branch: 'agent/slot-a',
              currentAgentId: 'agent-1',
              worktreeId: 'wt-a',
              totalTokens: 100,
              totalCostUsd: 0.42,
              worktreePath: '/tmp/worktrees/wt-a',
            },
          ],
        },
      ],
      taskDag: [
        {
          id: 'slot-a',
          title: 'Build shell',
          description: 'Build',
          status: 'TASK_STATUS_REVIEW',
          assignedAgentId: 'agent-1',
          projectId: 'proj-a',
          dependencyIds: [],
        },
      ],
      totalSessionCost: 1,
      sessionBudgetMaxUsd: 25,
      lastEventSequence: 1,
    }),
  );

  // Add a verification result
  cache.applyEvent(
    normalizeEvent({
      eventId: 'evt-vr-3',
      timestampMs: 1000,
      eventSequence: 2,
      verificationResult: {
        slotId: 'slot-a',
        projectId: 'proj-a',
        success: false,
        step: 'cargo build',
        command: 'cargo build',
        statusCode: 1,
        stdout: '',
        stderr: 'error',
      },
    }),
  );

  assert.ok(cache.getVerificationResult('slot-a'));

  // Transition task to WORKING — should clear the verification result
  cache.applyEvent(
    normalizeEvent({
      eventSequence: 3,
      taskStatusChanged: {
        taskId: 'slot-a',
        newStatus: 'TASK_STATUS_WORKING',
        agentId: 'agent-1',
      },
    }),
  );

  assert.equal(cache.getVerificationResult('slot-a'), undefined);
  assert.equal(cache.getVerificationResults().size, 0);

  cache.dispose();
});
