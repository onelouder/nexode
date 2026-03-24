import assert from 'node:assert/strict';
import test from 'node:test';

// merge-tree.ts imports vscode which is unavailable in unit tests.
// We test the pure logic directly here — these must stay in sync with
// the constants and functions in src/merge-tree.ts.

type TaskStatusName =
  | 'TASK_STATUS_UNSPECIFIED'
  | 'TASK_STATUS_PENDING'
  | 'TASK_STATUS_WORKING'
  | 'TASK_STATUS_REVIEW'
  | 'TASK_STATUS_MERGE_QUEUE'
  | 'TASK_STATUS_RESOLVING'
  | 'TASK_STATUS_DONE'
  | 'TASK_STATUS_PAUSED'
  | 'TASK_STATUS_ARCHIVED';

const MERGE_STATUSES: readonly TaskStatusName[] = [
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_RESOLVING',
];

function computeConflictRiskFromCount(
  status: TaskStatusName,
  concurrentCount: number,
): 'Low' | 'Medium' | 'High' {
  if (status === 'TASK_STATUS_RESOLVING') return 'High';
  if (concurrentCount >= 3) return 'High';
  if (concurrentCount >= 2) return 'Medium';
  return 'Low';
}

// -- Tests ----------------------------------------------------------------

test('MERGE_STATUSES contains only review-related statuses', () => {
  assert.deepEqual([...MERGE_STATUSES], [
    'TASK_STATUS_REVIEW',
    'TASK_STATUS_MERGE_QUEUE',
    'TASK_STATUS_RESOLVING',
  ]);
});

test('computeConflictRiskFromCount returns High for RESOLVING regardless of count', () => {
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_RESOLVING', 1), 'High');
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_RESOLVING', 0), 'High');
});

test('computeConflictRiskFromCount returns Low for single slot in REVIEW', () => {
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_REVIEW', 1), 'Low');
});

test('computeConflictRiskFromCount returns Medium for 2 concurrent slots', () => {
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_REVIEW', 2), 'Medium');
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_MERGE_QUEUE', 2), 'Medium');
});

test('computeConflictRiskFromCount returns High for 3+ concurrent slots', () => {
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_REVIEW', 3), 'High');
  assert.equal(computeConflictRiskFromCount('TASK_STATUS_MERGE_QUEUE', 5), 'High');
});

test('MERGE_STATUSES does not include terminal or working statuses', () => {
  const excluded: TaskStatusName[] = [
    'TASK_STATUS_UNSPECIFIED',
    'TASK_STATUS_PENDING',
    'TASK_STATUS_WORKING',
    'TASK_STATUS_DONE',
    'TASK_STATUS_PAUSED',
    'TASK_STATUS_ARCHIVED',
  ];
  for (const status of excluded) {
    assert.equal(MERGE_STATUSES.includes(status), false, `${status} should not be in MERGE_STATUSES`);
  }
});
