import * as path from 'path';
import * as vscode from 'vscode';

import { DaemonClient, readDaemonConfiguration } from './daemon-client';
import { registerCommands } from './commands';
import { NexodeDecorationProvider } from './decoration-provider';
import { DiagnosticManager } from './diagnostic-manager';
import { KanbanPanel } from './kanban-panel';
import { MergeTreeProvider } from './merge-tree';
import { OutputChannelManager } from './output-channel-manager';
import { SlotTreeProvider } from './slot-tree-provider';
import { StateCache } from './state';
import { NexodeStatusBar } from './status-bar';
import { SynapseGridPanel, SynapseSidebarProvider } from './synapse-grid-panel';
import { WorkspaceFolderManager } from './workspace-folder-manager';

let activeClient: DaemonClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const protoPath = context.asAbsolutePath(path.join('proto', 'hypervisor.proto'));
  const daemonConfig = readDaemonConfiguration();
  const state = new StateCache();
  const client = new DaemonClient({
    ...daemonConfig,
    protoPath,
  });
  const treeProvider = new SlotTreeProvider(state);
  const statusBar = new NexodeStatusBar(state);
  const synapseGridPanel = new SynapseGridPanel(context.extensionUri, state, client);
  const synapseSidebarProvider = new SynapseSidebarProvider(context.extensionUri, state);
  const kanbanPanel = new KanbanPanel(context.extensionUri, state, client);
  const diagnosticManager = new DiagnosticManager(state);
  const decorationProvider = new NexodeDecorationProvider(state);
  const mergeTreeProvider = new MergeTreeProvider(state);
  const workspaceFolderManager = new WorkspaceFolderManager(state);
  const outputChannelManager = new OutputChannelManager(state);
  const treeView = vscode.window.createTreeView('nexodeSlots', {
    treeDataProvider: treeProvider,
    showCollapseAll: true,
  });
  const mergeTreeView = vscode.window.createTreeView('nexodeMergeQueue', {
    treeDataProvider: mergeTreeProvider,
    showCollapseAll: true,
  });

  activeClient = client;

  context.subscriptions.push(
    state,
    client,
    treeProvider,
    statusBar,
    synapseGridPanel,
    synapseSidebarProvider,
    kanbanPanel,
    diagnosticManager,
    decorationProvider,
    mergeTreeProvider,
    workspaceFolderManager,
    outputChannelManager,
    treeView,
    mergeTreeView,
    vscode.window.registerFileDecorationProvider(decorationProvider),
    vscode.window.registerWebviewViewProvider('nexodeSynapseSidebar', synapseSidebarProvider, {
      webviewOptions: {
        retainContextWhenHidden: true,
      },
    }),
    client.onDidReceiveSnapshot((snapshot) => {
      state.applySnapshot(snapshot);
    }),
    client.onDidChangeConnectionStatus((status) => {
      state.setConnectionStatus(status);
    }),
    client.subscribeEvents((event) => {
      state.applyEvent(event);
    }),
    client.onDidReceiveAgentOutput((output) => outputChannelManager.appendLine(output)),
    vscode.commands.registerCommand('nexode.showSlotOutput', async () => {
      const slots = state.getAllSlots();
      if (slots.length === 0) {
        vscode.window.showInformationMessage('No active slots');
        return;
      }
      const items = slots.map((s) => ({
        label: s.slot.id,
        description: s.project.displayName,
      }));
      const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Select slot to show output',
      });
      if (selected) {
        outputChannelManager.showSlotOutput(selected.label);
      }
    }),
    vscode.commands.registerCommand('nexode.resetWorkspaceFolders', () => {
      workspaceFolderManager.resetFolders();
    }),
    vscode.commands.registerCommand('nexode.openSlotDiff', async (slotId?: string) => {
      let targetSlotId = slotId;
      if (!targetSlotId) {
        const slots = state.getAllSlots();
        if (slots.length === 0) {
          vscode.window.showInformationMessage('No active slots');
          return;
        }
        const items = slots.map((s) => ({
          label: s.slot.id,
          description: s.project.displayName,
        }));
        const selected = await vscode.window.showQuickPick(items, {
          placeHolder: 'Select slot to view diff',
        });
        if (!selected) {
          return;
        }
        targetSlotId = selected.label;
      }
      const slot = state.getAllSlots().find((s) => s.slot.id === targetSlotId);
      if (!slot?.slot.worktreePath) {
        vscode.window.showInformationMessage(`No worktree path for slot ${targetSlotId}`);
        return;
      }
      const uri = vscode.Uri.file(slot.slot.worktreePath);
      await vscode.commands.executeCommand('vscode.openFolder', uri, { forceNewWindow: false });
    }),
    vscode.commands.registerCommand('nexode.mergeApprove', async (node?: { slotId?: string }) => {
      const slotId = node?.slotId;
      if (!slotId) return;
      try {
        await client.dispatchCommand({
          commandId: `merge-approve-${Date.now()}`,
          moveTask: { taskId: slotId, target: 'TASK_STATUS_MERGE_QUEUE' },
        });
      } catch {
        void vscode.window.showErrorMessage(`Failed to approve slot ${slotId}`);
      }
    }),
    vscode.commands.registerCommand('nexode.mergeReject', async (node?: { slotId?: string }) => {
      const slotId = node?.slotId;
      if (!slotId) return;
      try {
        await client.dispatchCommand({
          commandId: `merge-reject-${Date.now()}`,
          moveTask: { taskId: slotId, target: 'TASK_STATUS_WORKING' },
        });
      } catch {
        void vscode.window.showErrorMessage(`Failed to reject slot ${slotId}`);
      }
    }),
    vscode.commands.registerCommand('nexode.mergePause', async (node?: { slotId?: string }) => {
      const slotId = node?.slotId;
      if (!slotId) return;
      try {
        await client.dispatchCommand({
          commandId: `merge-pause-${Date.now()}`,
          moveTask: { taskId: slotId, target: 'TASK_STATUS_PAUSED' },
        });
      } catch {
        void vscode.window.showErrorMessage(`Failed to pause slot ${slotId}`);
      }
    }),
    ...registerCommands(client, state, async () => {
      await vscode.commands.executeCommand('workbench.view.extension.nexode');
      try {
        await vscode.commands.executeCommand('nexodeSlots.focus');
      } catch {
        // Older builds may not expose the auto-generated focus command for contributed views.
      }
    }, async () => {
      synapseGridPanel.show();
    }, async () => {
      kanbanPanel.show();
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration('nexode.daemonHost') ||
        event.affectsConfiguration('nexode.daemonPort')
      ) {
        const nextConfig = readDaemonConfiguration();
        void client.updateEndpoint(nextConfig.host, nextConfig.port);
      }
    }),
  );

  try {
    await client.connect();
  } catch {
    // Daemon may not be running yet — extension activates but stays disconnected.
    // DaemonClient will retry on its own when the daemon becomes available.
  }
}

export async function deactivate(): Promise<void> {
  await activeClient?.disconnect();
  activeClient = undefined;
}
