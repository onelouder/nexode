import * as path from 'path';
import * as vscode from 'vscode';

import { DaemonClient, readDaemonConfiguration } from './daemon-client';
import { registerCommands } from './commands';
import { SlotTreeProvider } from './slot-tree-provider';
import { StateCache } from './state';
import { NexodeStatusBar } from './status-bar';

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
  const treeView = vscode.window.createTreeView('nexodeSlots', {
    treeDataProvider: treeProvider,
    showCollapseAll: true,
  });

  activeClient = client;

  context.subscriptions.push(
    state,
    client,
    treeProvider,
    statusBar,
    treeView,
    client.onDidReceiveSnapshot((snapshot) => {
      state.applySnapshot(snapshot);
    }),
    client.onDidChangeConnectionStatus((status) => {
      state.setConnectionStatus(status);
    }),
    client.subscribeEvents((event) => {
      state.applyEvent(event);
    }),
    ...registerCommands(client, state, async () => {
      await vscode.commands.executeCommand('workbench.view.extension.nexode');
      try {
        await vscode.commands.executeCommand('nexodeSlots.focus');
      } catch {
        // Older builds may not expose the auto-generated focus command for contributed views.
      }
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

  await client.connect();
}

export async function deactivate(): Promise<void> {
  await activeClient?.disconnect();
  activeClient = undefined;
}
