import * as vscode from 'vscode';

import { StateCache, TaskStatusName, formatTaskStatus, formatTokenCount } from './state';

type TreeNode = ProjectNode | SlotNode | MessageNode;

interface ProjectNode {
  kind: 'project';
  projectId: string;
}

interface SlotNode {
  kind: 'slot';
  projectId: string;
  slotId: string;
}

interface MessageNode {
  kind: 'message';
  label: string;
}

const REFRESH_DEBOUNCE_MS = 100;

export class SlotTreeProvider implements vscode.TreeDataProvider<TreeNode>, vscode.Disposable {
  private readonly changeEmitter = new vscode.EventEmitter<TreeNode | undefined | null | void>();
  private readonly disposables: vscode.Disposable[] = [];
  private refreshTimer?: ReturnType<typeof setTimeout>;

  public readonly onDidChangeTreeData = this.changeEmitter.event;

  public constructor(private readonly state: StateCache) {
    this.disposables.push(
      this.state.onDidChange(() => {
        this.scheduleRefresh();
      }),
    );
  }

  public dispose(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = undefined;
    }

    vscode.Disposable.from(...this.disposables).dispose();
    this.changeEmitter.dispose();
  }

  public getTreeItem(element: TreeNode): vscode.TreeItem {
    if (element.kind === 'message') {
      const item = new vscode.TreeItem(element.label, vscode.TreeItemCollapsibleState.None);
      item.contextValue = 'message';
      return item;
    }

    if (element.kind === 'project') {
      const project = this.state.getProjects().find((entry) => entry.id === element.projectId);
      const slotCount = project?.slots.length ?? 0;
      const item = new vscode.TreeItem(
        project?.displayName || element.projectId,
        vscode.TreeItemCollapsibleState.Expanded,
      );
      item.description = slotCount === 1 ? '1 slot' : `${slotCount} slots`;
      item.tooltip = project?.repoPath || project?.displayName || element.projectId;
      item.contextValue = 'project';
      return item;
    }

    const project = this.state.getProjects().find((entry) => entry.id === element.projectId);
    const slot = project?.slots.find((entry) => entry.id === element.slotId);
    const status = this.state.getTaskStatusForSlot(element.slotId);

    const item = new vscode.TreeItem(element.slotId, vscode.TreeItemCollapsibleState.None);
    item.contextValue = 'slot';
    item.iconPath = new vscode.ThemeIcon('circle-filled', statusColor(status));

    if (slot) {
      const detailParts = [formatTaskStatus(status)];
      if (slot.currentAgentId) {
        detailParts.push(slot.currentAgentId);
      }
      detailParts.push(`${formatTokenCount(slot.totalTokens)} tok`);
      item.description = detailParts.join(' · ');

      const task = this.state.getTaskById(slot.id);
      item.tooltip = new vscode.MarkdownString(
        [
          `**${slot.id}**`,
          '',
          `Status: ${formatTaskStatus(status)}`,
          `Agent: ${slot.currentAgentId || '-'}`,
          `Branch: ${slot.branch || '-'}`,
          `Tokens: ${formatTokenCount(slot.totalTokens)}`,
          `Task: ${task?.title || slot.task || '-'}`,
        ].join('  \n'),
      );
    }

    return item;
  }

  public getChildren(element?: TreeNode): TreeNode[] {
    if (!element) {
      const projects = this.state.getProjects();
      if (projects.length === 0) {
        const status = this.state.getConnectionStatus();
        const label =
          status.state === 'reconnecting'
            ? 'Waiting for Nexode daemon to reconnect...'
            : status.state === 'disconnected'
              ? 'Nexode daemon is disconnected'
              : 'No projects available';
        return [{ kind: 'message', label }];
      }

      return projects.map<TreeNode>((project) => ({
        kind: 'project',
        projectId: project.id,
      }));
    }

    if (element.kind !== 'project') {
      return [];
    }

    const project = this.state.getProjects().find((entry) => entry.id === element.projectId);
    return (
      project?.slots.map<TreeNode>((slot) => ({
        kind: 'slot',
        projectId: project.id,
        slotId: slot.id,
      })) ?? []
    );
  }

  private scheduleRefresh(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
    }

    this.refreshTimer = setTimeout(() => {
      this.refreshTimer = undefined;
      this.changeEmitter.fire();
    }, REFRESH_DEBOUNCE_MS);
  }
}

function statusColor(status: TaskStatusName): vscode.ThemeColor {
  switch (status) {
    case 'TASK_STATUS_PENDING':
      return new vscode.ThemeColor('descriptionForeground');
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
    case 'TASK_STATUS_PAUSED':
    case 'TASK_STATUS_ARCHIVED':
    case 'TASK_STATUS_UNSPECIFIED':
    default:
      return new vscode.ThemeColor('disabledForeground');
  }
}
