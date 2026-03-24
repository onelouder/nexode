import * as vscode from 'vscode';

import {
  StateCache,
  SlotSummary,
  TaskStatusName,
  formatTaskStatus,
  formatTokenCount,
} from './state';

type MergeTreeNode = ProjectGroupNode | MergeSlotNode | EmptyNode;

interface ProjectGroupNode {
  kind: 'projectGroup';
  projectId: string;
}

interface MergeSlotNode {
  kind: 'mergeSlot';
  projectId: string;
  slotId: string;
}

interface EmptyNode {
  kind: 'empty';
  label: string;
}

/** Statuses that belong in the merge choreography view. */
const MERGE_STATUSES: readonly TaskStatusName[] = [
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_RESOLVING',
];

const REFRESH_DEBOUNCE_MS = 100;

export class MergeTreeProvider
  implements vscode.TreeDataProvider<MergeTreeNode>, vscode.Disposable
{
  private readonly changeEmitter = new vscode.EventEmitter<
    MergeTreeNode | undefined | null | void
  >();
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

  public getTreeItem(element: MergeTreeNode): vscode.TreeItem {
    if (element.kind === 'empty') {
      const item = new vscode.TreeItem(element.label, vscode.TreeItemCollapsibleState.None);
      item.contextValue = 'message';
      return item;
    }

    if (element.kind === 'projectGroup') {
      return this.buildProjectItem(element);
    }

    return this.buildSlotItem(element);
  }

  public getChildren(element?: MergeTreeNode): MergeTreeNode[] {
    if (!element) {
      return this.getRootChildren();
    }

    if (element.kind !== 'projectGroup') {
      return [];
    }

    return this.getMergeSlots()
      .filter((s) => s.project.id === element.projectId)
      .map<MergeSlotNode>((s) => ({
        kind: 'mergeSlot',
        projectId: s.project.id,
        slotId: s.slot.id,
      }));
  }

  // -- Internals ----------------------------------------------------------

  private getRootChildren(): MergeTreeNode[] {
    const slots = this.getMergeSlots();
    if (slots.length === 0) {
      const status = this.state.getConnectionStatus();
      if (status.state !== 'connected') {
        return [{ kind: 'empty', label: 'Waiting for daemon connection...' }];
      }
      return [{ kind: 'empty', label: 'No active merges' }];
    }

    // Group by project — deduplicate project IDs
    const projectIds = [...new Set(slots.map((s) => s.project.id))];
    return projectIds.map<ProjectGroupNode>((id) => ({
      kind: 'projectGroup',
      projectId: id,
    }));
  }

  private getMergeSlots(): SlotSummary[] {
    return this.state.getSlotsByStatuses(MERGE_STATUSES);
  }

  private buildProjectItem(node: ProjectGroupNode): vscode.TreeItem {
    const project = this.state.getProjects().find((p) => p.id === node.projectId);
    const slots = this.getMergeSlots().filter((s) => s.project.id === node.projectId);

    const item = new vscode.TreeItem(
      project?.displayName || node.projectId,
      vscode.TreeItemCollapsibleState.Expanded,
    );

    const reviewCount = slots.filter((s) => s.status === 'TASK_STATUS_REVIEW').length;
    const queueCount = slots.filter((s) => s.status === 'TASK_STATUS_MERGE_QUEUE').length;
    const resolvingCount = slots.filter((s) => s.status === 'TASK_STATUS_RESOLVING').length;

    const parts: string[] = [];
    if (reviewCount > 0) parts.push(`${reviewCount} review`);
    if (queueCount > 0) parts.push(`${queueCount} queued`);
    if (resolvingCount > 0) parts.push(`${resolvingCount} conflict`);
    item.description = parts.join(', ');

    item.contextValue = 'mergeProject';
    item.iconPath = new vscode.ThemeIcon('git-merge');

    return item;
  }

  private buildSlotItem(node: MergeSlotNode): vscode.TreeItem {
    const project = this.state.getProjects().find((p) => p.id === node.projectId);
    const slot = project?.slots.find((s) => s.id === node.slotId);
    const status = this.state.getTaskStatusForSlot(node.slotId);
    const task = this.state.getTaskById(node.slotId);
    const risk = this.computeConflictRisk(node);

    const item = new vscode.TreeItem(node.slotId, vscode.TreeItemCollapsibleState.None);
    item.contextValue = 'mergeSlot';

    // Icon based on status
    item.iconPath = new vscode.ThemeIcon(mergeStatusIcon(status), mergeStatusColor(status));

    // Description: branch + risk
    const descParts: string[] = [];
    descParts.push(formatTaskStatus(status));
    if (slot?.branch) descParts.push(slot.branch);
    descParts.push(riskLabel(risk));
    item.description = descParts.join(' · ');

    // Tooltip
    item.tooltip = new vscode.MarkdownString(
      [
        `**${node.slotId}**`,
        '',
        `Status: ${formatTaskStatus(status)}`,
        `Branch: ${slot?.branch || '-'}`,
        `Agent: ${slot?.currentAgentId || '-'}`,
        `Tokens: ${formatTokenCount(slot?.totalTokens ?? 0)}`,
        `Task: ${task?.title || slot?.task || '-'}`,
        `Conflict risk: ${riskLabel(risk)}`,
      ].join('  \n'),
    );

    return item;
  }

  /**
   * Heuristic conflict risk based on:
   * - Number of concurrent slots in REVIEW/MERGE_QUEUE for same project (more = higher risk)
   * - RESOLVING status is always High
   */
  private computeConflictRisk(node: MergeSlotNode): 'Low' | 'Medium' | 'High' {
    const status = this.state.getTaskStatusForSlot(node.slotId);
    if (status === 'TASK_STATUS_RESOLVING') {
      return 'High';
    }

    const sameProjectSlots = this.getMergeSlots().filter(
      (s) => s.project.id === node.projectId,
    );

    if (sameProjectSlots.length >= 3) return 'High';
    if (sameProjectSlots.length >= 2) return 'Medium';
    return 'Low';
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

// -- Exported for testing -------------------------------------------------

export { MERGE_STATUSES };

export function computeConflictRiskFromCount(
  status: TaskStatusName,
  concurrentCount: number,
): 'Low' | 'Medium' | 'High' {
  if (status === 'TASK_STATUS_RESOLVING') return 'High';
  if (concurrentCount >= 3) return 'High';
  if (concurrentCount >= 2) return 'Medium';
  return 'Low';
}

// -- Helpers --------------------------------------------------------------

function mergeStatusIcon(status: TaskStatusName): string {
  switch (status) {
    case 'TASK_STATUS_REVIEW':
      return 'eye';
    case 'TASK_STATUS_MERGE_QUEUE':
      return 'checklist';
    case 'TASK_STATUS_RESOLVING':
      return 'warning';
    default:
      return 'circle-outline';
  }
}

function mergeStatusColor(status: TaskStatusName): vscode.ThemeColor {
  switch (status) {
    case 'TASK_STATUS_REVIEW':
      return new vscode.ThemeColor('terminal.ansiYellow');
    case 'TASK_STATUS_MERGE_QUEUE':
      return new vscode.ThemeColor('terminal.ansiBlue');
    case 'TASK_STATUS_RESOLVING':
      return new vscode.ThemeColor('terminal.ansiRed');
    default:
      return new vscode.ThemeColor('disabledForeground');
  }
}

function riskLabel(risk: 'Low' | 'Medium' | 'High'): string {
  switch (risk) {
    case 'High':
      return 'Risk: High';
    case 'Medium':
      return 'Risk: Medium';
    case 'Low':
      return 'Risk: Low';
  }
}
