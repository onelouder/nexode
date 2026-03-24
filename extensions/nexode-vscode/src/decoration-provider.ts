import * as vscode from 'vscode';

import type { StateCache, TaskStatusName } from './state';

function statusBadge(status: TaskStatusName): string {
  switch (status) {
    case 'TASK_STATUS_WORKING':
      return 'WK';
    case 'TASK_STATUS_REVIEW':
      return 'RV';
    case 'TASK_STATUS_MERGE_QUEUE':
      return 'MQ';
    case 'TASK_STATUS_RESOLVING':
      return 'RS';
    case 'TASK_STATUS_DONE':
      return 'DN';
    case 'TASK_STATUS_PAUSED':
      return 'PA';
    case 'TASK_STATUS_PENDING':
      return 'PE';
    case 'TASK_STATUS_ARCHIVED':
      return 'AR';
    default:
      return '';
  }
}

function statusColor(status: TaskStatusName): vscode.ThemeColor {
  switch (status) {
    case 'TASK_STATUS_WORKING':
      return new vscode.ThemeColor('terminal.ansiCyan');
    case 'TASK_STATUS_REVIEW':
      return new vscode.ThemeColor('terminal.ansiYellow');
    case 'TASK_STATUS_MERGE_QUEUE':
      return new vscode.ThemeColor('terminal.ansiBlue');
    case 'TASK_STATUS_RESOLVING':
      return new vscode.ThemeColor('terminal.ansiRed');
    case 'TASK_STATUS_DONE':
      return new vscode.ThemeColor('terminal.ansiGreen');
    default:
      return new vscode.ThemeColor('disabledForeground');
  }
}

export class NexodeDecorationProvider implements vscode.FileDecorationProvider, vscode.Disposable {
  private readonly changeEmitter = new vscode.EventEmitter<vscode.Uri | vscode.Uri[] | undefined>();
  readonly onDidChangeFileDecorations = this.changeEmitter.event;
  private readonly stateSubscription: vscode.Disposable;

  constructor(private readonly state: StateCache) {
    this.stateSubscription = this.state.onDidChange(() => {
      this.changeEmitter.fire(undefined); // fire for all URIs
    });
  }

  provideFileDecoration(uri: vscode.Uri): vscode.FileDecoration | undefined {
    // Match URI against known slot worktree paths
    const uriPath = uri.fsPath;
    for (const project of this.state.getProjects()) {
      for (const slot of project.slots) {
        if (!slot.worktreePath) {
          continue;
        }
        if (uriPath === slot.worktreePath || uriPath === slot.worktreePath + '/') {
          const status = this.state.getTaskStatusForSlot(slot.id);
          const badge = statusBadge(status);
          if (!badge) {
            return undefined;
          }

          const tokens = slot.totalTokens;
          const tokenStr = tokens > 1000 ? `${(tokens / 1000).toFixed(1)}k` : `${tokens}`;

          return {
            badge,
            color: statusColor(status),
            tooltip: `${slot.currentAgentId || 'idle'} · ${tokenStr} tok · ${badge}`,
          };
        }
      }
    }
    return undefined;
  }

  dispose(): void {
    this.stateSubscription.dispose();
    this.changeEmitter.dispose();
  }
}
