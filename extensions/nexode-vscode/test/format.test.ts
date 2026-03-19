import assert from 'node:assert/strict';
import test from 'node:test';

import {
  agentTone,
  alertTone,
  formatAgentState,
  formatAlertKind,
  formatAlertMessage,
  formatCurrency,
  formatMode,
  formatStatus,
  statusTone,
  toTitleWords,
} from '../webview/shared/format';

test('shared formatters normalize enum-style values', () => {
  assert.equal(toTitleWords('MERGE_QUEUE'), 'Merge Queue');
  assert.equal(formatStatus('TASK_STATUS_MERGE_QUEUE'), 'Merge Queue');
  assert.equal(formatAgentState('AGENT_STATE_EXECUTING'), 'Executing');
  assert.equal(formatMode('AGENT_MODE_FULL_AUTO'), 'Full Auto');
});

test('shared formatters produce stable numeric and tone output', () => {
  assert.equal(formatCurrency(12.5), '$12.50');
  assert.equal(formatCurrency(0), '$0.00');
  assert.equal(statusTone('TASK_STATUS_REVIEW'), 'warn');
  assert.equal(statusTone('TASK_STATUS_DONE'), 'success');
  assert.equal(agentTone('AGENT_STATE_BLOCKED'), 'muted');
  assert.equal(agentTone('AGENT_STATE_IDLE'), 'success');
});

test('shared alert formatters render observer findings', () => {
  assert.equal(
    formatAlertKind({
      loopDetected: {
        reason: 'repeated command',
        intervention: 'OBSERVER_INTERVENTION_PAUSE',
        findingKind: 'FINDING_KIND_LOOP_DETECTED',
      },
    }),
    'Loop Detected',
  );
  assert.equal(
    formatAlertMessage({
      sandboxViolation: {
        path: '/tmp/outside',
        reason: 'outside worktree',
      },
    }),
    'outside worktree (/tmp/outside)',
  );
  assert.equal(
    alertTone({
      uncertaintySignal: {
        reason: 'needs human input',
      },
    }),
    'info',
  );
});
