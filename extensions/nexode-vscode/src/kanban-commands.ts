import type { MoveTaskMessage } from '../webview/shared/types';

export interface MoveTaskCommand {
  [key: string]: unknown;
  commandId: string;
  moveTask: {
    taskId: string;
    target: MoveTaskMessage['target'];
  };
}

export function createMoveTaskCommand(
  message: MoveTaskMessage,
  createCommandId: () => string = defaultCommandId,
): MoveTaskCommand {
  return {
    commandId: createCommandId(),
    moveTask: {
      taskId: message.taskId,
      target: message.target,
    },
  };
}

function defaultCommandId(): string {
  return `webview-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}
