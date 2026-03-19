import assert from 'node:assert/strict';
import test from 'node:test';

import { createMoveTaskCommand } from '../src/kanban-commands';

test('createMoveTaskCommand maps webview move messages to daemon commands', () => {
  const command = createMoveTaskCommand(
    {
      type: 'moveTask',
      taskId: 'task-a',
      target: 'TASK_STATUS_REVIEW',
    },
    () => 'cmd-fixed',
  );

  assert.deepEqual(command, {
    commandId: 'cmd-fixed',
    moveTask: {
      taskId: 'task-a',
      target: 'TASK_STATUS_REVIEW',
    },
  });
});
