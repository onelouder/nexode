import * as vscode from 'vscode';
import type { StateCache, TaskStatusName } from './state';

const DONE_STATUSES: Set<TaskStatusName> = new Set([
  'TASK_STATUS_DONE',
  'TASK_STATUS_ARCHIVED',
]);

export class WorkspaceFolderManager implements vscode.Disposable {
  private readonly stateSubscription: vscode.Disposable;
  private knownFolders: Map<string, string> = new Map(); // slotId -> worktreePath
  private updateTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly state: StateCache) {
    this.stateSubscription = this.state.onDidChange(() => this.scheduleReconcile());
  }

  resetFolders(): void {
    // Remove all Nexode-managed folders and re-reconcile
    this.knownFolders.clear();
    this.reconcile();
  }

  private scheduleReconcile(): void {
    if (this.updateTimer !== undefined) {
      clearTimeout(this.updateTimer);
    }
    this.updateTimer = setTimeout(() => {
      this.updateTimer = undefined;
      this.reconcile();
    }, 200);
  }

  private reconcile(): void {
    if (!this.state.hasSnapshot()) {
      return;
    }

    // Build desired state from slots with worktree paths
    const desired = new Map<string, { path: string; name: string }>();
    for (const project of this.state.getProjects()) {
      for (const slot of project.slots) {
        if (slot.worktreePath && !DONE_STATUSES.has(this.getSlotTaskStatus(slot.id))) {
          desired.set(slot.id, {
            path: slot.worktreePath,
            name: `${project.displayName}/${slot.id}`,
          });
        }
      }
    }

    // Find folders to add and remove
    const currentFolders = vscode.workspace.workspaceFolders ?? [];
    const managedUris = new Set<string>();
    for (const [, path] of this.knownFolders) {
      managedUris.add(vscode.Uri.file(path).toString());
    }

    // Folders to remove (in managed set but not in desired)
    const toRemoveIndices: number[] = [];
    for (let i = currentFolders.length - 1; i >= 0; i--) {
      const uri = currentFolders[i].uri.toString();
      if (managedUris.has(uri)) {
        // This is a Nexode-managed folder — check if still desired
        const stillDesired = [...desired.values()].some(
          (d) => vscode.Uri.file(d.path).toString() === uri,
        );
        if (!stillDesired) {
          toRemoveIndices.push(i);
        }
      }
    }

    // Folders to add (in desired but not already present)
    const existingUris = new Set(currentFolders.map((f) => f.uri.toString()));
    const toAdd: { uri: vscode.Uri; name: string }[] = [];
    for (const [slotId, { path, name }] of desired) {
      const uri = vscode.Uri.file(path);
      if (!existingUris.has(uri.toString())) {
        toAdd.push({ uri, name });
      }
      this.knownFolders.set(slotId, path);
    }

    // Remove slots no longer desired from knownFolders
    for (const [slotId] of this.knownFolders) {
      if (!desired.has(slotId)) {
        this.knownFolders.delete(slotId);
      }
    }

    // Apply changes
    if (toRemoveIndices.length > 0 || toAdd.length > 0) {
      // Remove from highest index first to preserve indices
      toRemoveIndices.sort((a, b) => b - a);
      for (const idx of toRemoveIndices) {
        vscode.workspace.updateWorkspaceFolders(idx, 1);
      }
      if (toAdd.length > 0) {
        const start = vscode.workspace.workspaceFolders?.length ?? 0;
        vscode.workspace.updateWorkspaceFolders(
          start,
          0,
          ...toAdd.map((a) => ({ uri: a.uri, name: a.name })),
        );
      }
    }
  }

  private getSlotTaskStatus(slotId: string): TaskStatusName {
    return this.state.getTaskStatusForSlot(slotId);
  }

  dispose(): void {
    if (this.updateTimer !== undefined) {
      clearTimeout(this.updateTimer);
    }
    this.stateSubscription.dispose();
  }
}
